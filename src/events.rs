//! The service's one event stream (`docs/api.md`, 「Events」): each write numbered and kept in the store with the
//! member it moved, a window of the latest kept for a reader resuming, and handed to every live reader in the
//! order the numbers run, each reader hearing the events of the projects they see.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode, header};
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream::{self, StreamExt};
use schemars::generate::SchemaGenerator;
use serde::Serialize;
use serde_json::json;
use sqlx::{FromRow, Sqlite, Transaction};
use tokio::sync::{Mutex, MutexGuard, broadcast, watch};
use tokio_stream::wrappers::BroadcastStream;

use crate::http::{Call, MOST_PAGE, PAGE, Refused, Route, answer, page_size};
use crate::openapi::{Stated, query};
use crate::owners::Seen;
use crate::store::{Store, now_ms};
use crate::wire::{Event, EventQuery, Page};

/// How many past events the store keeps for a reader resuming.
pub const WINDOW: i64 = 1_000;
// How many events a live reader may fall behind before it is let go, to resume from the window.
const BACKLOG: usize = 256;
// How often an idle stream says it is still there.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// The kinds of event, a resource and its change.
pub const REVIEW_CREATED: &str = "review.created";
pub const REVIEW_CHANGED: &str = "review.changed";
pub const COMMENT_CREATED: &str = "comment.created";
pub const FILING_CREATED: &str = "filing.created";

/// An event as the live readers hear it, with the project its review is of.
#[derive(Clone, Debug)]
struct Heard {
  event: Event,
  project_name: String,
}

/// The event log of the service.
#[derive(Clone)]
pub struct Events {
  store: Store,
  live: broadcast::Sender<Heard>,
  // Held from a write's start to its events' announcement, so live readers hear the numbers in order.
  writing: Arc<Mutex<()>>,
  // Set once the service is told to stop: every live stream ends then, so the stop waits on no reader.
  stopping: Arc<watch::Sender<bool>>,
}

/// A write under way: its events are numbered inside its transaction and announced once it is committed.
pub struct Writing<'a> {
  events: &'a Events,
  _held: MutexGuard<'a, ()>,
  recorded: Vec<Heard>,
}

impl Writing<'_> {
  /// One event of `kind` about `data`, belonging to `review_id`, numbered in `transaction`.
  pub async fn record(
    &mut self,
    transaction: &mut Transaction<'static, Sqlite>,
    kind: &str,
    review_id: i64,
    data: &impl Serialize,
  ) -> Result<(), Refused> {
    let data = serde_json::to_value(data).map_err(Refused::failed)?;
    let project_name: String = sqlx::query_scalar("SELECT project_name FROM reviews WHERE id = ?")
      .bind(review_id)
      .fetch_one(&mut **transaction)
      .await
      .map_err(Refused::failed)?;
    let at = now_ms();
    let id: i64 =
      sqlx::query_scalar("INSERT INTO events (kind, review_id, at, data) VALUES (?, ?, ?, ?) RETURNING id")
        .bind(kind)
        .bind(review_id)
        .bind(at)
        .bind(data.to_string())
        .fetch_one(&mut **transaction)
        .await
        .map_err(Refused::failed)?;

    sqlx::query("DELETE FROM events WHERE id <= ?")
      .bind(id - WINDOW)
      .execute(&mut **transaction)
      .await
      .map_err(Refused::failed)?;

    self.recorded.push(Heard {
      event: Event {
        id,
        kind: kind.to_owned(),
        at,
        data,
      },
      project_name,
    });

    Ok(())
  }

  /// Hands the events recorded to the live readers, once their transaction is committed.
  pub fn announce(self) {
    for heard in self.recorded {
      // No reader listening is no failure: the window keeps it for the next one.
      let _ = self.events.live.send(heard);
    }
  }
}

#[derive(FromRow)]
struct Row {
  id: i64,
  kind: String,
  at: i64,
  data: String,
}

impl Row {
  fn event(self) -> Event {
    Event {
      id: self.id,
      kind: self.kind,
      at: self.at,
      data: serde_json::from_str(&self.data).unwrap_or_default(),
    }
  }
}

/// What a reader narrowed the stream to, and the projects they see as it is read.
#[derive(Clone, Debug)]
struct Narrowing {
  kinds: Vec<String>,
  review_id: Option<i64>,
  seen: Seen,
}

impl Narrowing {
  fn of(query: &EventQuery, seen: Seen) -> Self {
    let kinds = query
      .kinds
      .iter()
      .flat_map(|kinds| kinds.split(','))
      .map(str::trim)
      .filter(|kind| !kind.is_empty())
      .map(str::to_owned)
      .collect();

    Self {
      kinds,
      review_id: query.review_id,
      seen,
    }
  }

  fn wants(&self, event: &Event) -> bool {
    let kind_wanted = self.kinds.is_empty() || self.kinds.contains(&event.kind);
    let review_wanted = self.review_id.is_none_or(|wanted| review_of(event) == Some(wanted));

    kind_wanted && review_wanted
  }

  /// The kinds as the store's query takes them, a JSON array, or none where every kind is wanted.
  fn kinds_json(&self) -> Option<String> {
    (!self.kinds.is_empty()).then(|| serde_json::Value::from(self.kinds.clone()).to_string())
  }
}

/// The review an event belongs to: a review's own id, or the review its comment or filing stands under.
fn review_of(event: &Event) -> Option<i64> {
  let key = if event.kind.starts_with("review.") { "id" } else { "reviewId" };

  event.data.get(key).and_then(serde_json::Value::as_i64)
}

impl Events {
  pub fn new(store: Store) -> Self {
    Self {
      store,
      live: broadcast::channel(BACKLOG).0,
      writing: Arc::new(Mutex::new(())),
      stopping: Arc::new(watch::channel(false).0),
    }
  }

  /// Ends every live stream: the service is stopping.
  pub fn end_streams(&self) {
    self.stopping.send_replace(true);
  }

  /// Starts a write whose events are announced in number order.
  pub async fn write(&self) -> Writing<'_> {
    Writing {
      events: self,
      _held: self.writing.lock().await,
      recorded: Vec::new(),
    }
  }

  /// The kept events past `after` a reader wants, at most `limit` and one more to tell whether more stand;
  /// refused where `after` stands before the window, so events past it were let go.
  async fn since(
    &self,
    after: i64,
    narrowing: &Narrowing,
    limit: Option<usize>,
  ) -> Result<Vec<Event>, Refused> {
    let first: Option<i64> = sqlx::query_scalar("SELECT MIN(id) FROM events")
      .fetch_one(self.store.pool())
      .await
      .map_err(Refused::failed)?;

    if first.is_some_and(|first| after < first - 1) {
      return Err(Refused::new(
        StatusCode::GONE,
        "gone",
        format!("the events after {after} are no longer kept; read the collections again"),
      ));
    }

    let rows: Vec<Row> = sqlx::query_as(
      "SELECT id, kind, at, data FROM events WHERE id > ?1 \
       AND (?2 IS NULL OR review_id = ?2) \
       AND (?3 IS NULL OR kind IN (SELECT value FROM json_each(?3))) \
       AND (?5 IS NULL OR review_id IN (SELECT id FROM reviews \
         WHERE project_name IN (SELECT value FROM json_each(?5)))) \
       ORDER BY id LIMIT ?4",
    )
    .bind(after)
    .bind(narrowing.review_id)
    .bind(narrowing.kinds_json())
    .bind(limit.map_or(-1, |limit| i64::try_from(limit + 1).unwrap_or(i64::MAX)))
    .bind(narrowing.seen.names_json())
    .fetch_all(self.store.pool())
    .await
    .map_err(Refused::failed)?;

    Ok(rows.into_iter().map(Row::event).collect())
  }
}

/// `GET /events`: the stream, or a page of it to a reader asking for JSON.
pub struct EventStream;

impl Route for EventStream {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/events";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let query: EventQuery = call.query()?;
    let resumed = call.header("last-event-id").and_then(|id| id.trim().parse::<i64>().ok());
    let after = query.after.or(resumed).unwrap_or(0);
    let narrowing = Narrowing::of(&query, call.seen().await?);
    let events = &call.standing().events;

    if call.accepts_json() {
      let limit = page_size(query.limit, PAGE, MOST_PAGE)?;
      let mut items = events.since(after, &narrowing, Some(limit)).await?;
      let more = items.len() > limit;

      items.truncate(limit);

      return Ok(answer(StatusCode::OK, &Page { items, more }));
    }

    // The live channel is heard before the window is read, so nothing published between the two is lost.
    let live = BroadcastStream::new(events.live.subscribe());
    let mut stopping = events.stopping.subscribe();
    let kept = events.since(after, &narrowing, None).await?;
    let replayed = kept.last().map_or(after, |event| event.id);
    let owners = call.standing().owners.clone();
    let reader = call.reader();
    let live = live
      // A reader too slow for the channel is let go; it resumes from the window with the last number it read.
      .take_while(|heard| std::future::ready(heard.is_ok()))
      .filter_map(move |heard| {
        let wanted = heard.ok().filter(|heard| heard.event.id > replayed && narrowing.wants(&heard.event));
        let owners = owners.clone();
        let reader = reader.clone();

        // Each event heard live is judged by the projects the reader sees as it is heard; where that cannot be
        // read, the event is not sent.
        async move {
          let heard = wanted?;
          let seen = owners.seen(&reader).await.ok()?;

          seen.holds(&heard.project_name).then_some(heard.event)
        }
      });
    let sent = stream::iter(kept)
      .chain(live)
      .map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();

        Ok::<_, Infallible>(Sent::default().id(event.id.to_string()).event(event.kind.clone()).data(data))
      })
      .take_until(async move {
        let _ = stopping.wait_for(|stopping| *stopping).await;
      });

    let mut response = Sse::new(sent).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)).into_response();

    response
      .headers_mut()
      .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-cache"));

    Ok(response)
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    let events = json!({
      "text/event-stream": { "schema": { "type": "string", "description": "Server-sent events, each an event." } },
      "application/json": { "schema": generator.subschema_for::<Page<Event>>() },
    });

    Stated {
      id: "listEvents",
      tag: "events",
      summary: "The one stream: reviews, comments, filings (SSE), or a page of it with Accept: application/json.",
      parameters: query::<EventQuery>(generator),
      body: None,
      answer: (200, events),
      codes: vec!["gone"],
    }
  }
}

crate::route!(EventStream);
