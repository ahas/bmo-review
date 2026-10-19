//! The service's own event stream (`docs/api.md`, 「Events」): what moved, kept in a ring and replayed after the
//! cursor a reader names, then sent live as it happens.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::Method;
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream::{self, Stream, StreamExt};
use serde_json::json;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::http::{Call, Refused, Route};

// How many events the ring keeps for a reader coming back after a cursor.
const RING: usize = 256;
// How often an idle stream says it is still there.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// What moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
  /// A review was made, or an item was filed from it.
  Review,
  /// The owner's words of a review's thread were written and a turn went out.
  Talk,
  /// A review's turn ended: answered, or with the code it failed with.
  TalkEnded { failure: Option<&'static str> },
}

impl Kind {
  /// The event's name on the stream.
  pub fn name(self) -> &'static str {
    match self {
      Self::Review => "code-review",
      Self::Talk | Self::TalkEnded { .. } => "code-review-talk",
    }
  }
}

/// One event: its cursor, what moved, and the review it moved on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
  pub cursor: u64,
  pub kind: Kind,
  pub review: i64,
}

impl Event {
  fn sent(self) -> Sent {
    let mut data = json!({ "review": self.review });

    if let Kind::TalkEnded { failure } = self.kind {
      data["detail"] = json!(failure);
    }

    Sent::default()
      .id(self.cursor.to_string())
      .event(self.kind.name())
      .data(data.to_string())
  }
}

#[derive(Debug, Default)]
struct Ring {
  next: u64,
  kept: VecDeque<Event>,
}

/// The events the service sends: a ring of the latest, and the live channel every reader hears.
#[derive(Clone, Debug)]
pub struct Events {
  ring: Arc<Mutex<Ring>>,
  live: broadcast::Sender<Event>,
}

impl Default for Events {
  fn default() -> Self {
    Self::new()
  }
}

impl Events {
  pub fn new() -> Self {
    let (live, _) = broadcast::channel(RING);

    Self {
      ring: Arc::new(Mutex::new(Ring {
        next: 1,
        kept: VecDeque::new(),
      })),
      live,
    }
  }

  /// Sends that something moved on a review.
  pub fn send(&self, kind: Kind, review: i64) {
    let event = {
      let mut ring = self.ring.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      let event = Event {
        cursor: ring.next,
        kind,
        review,
      };

      ring.next += 1;
      ring.kept.push_back(event);

      if ring.kept.len() > RING {
        ring.kept.pop_front();
      }

      event
    };

    // No reader listening is no failure: the ring keeps it for the next one.
    let _ = self.live.send(event);
  }

  /// The events after `cursor` the ring still keeps, then every one sent from now on, none twice.
  pub fn after(&self, cursor: u64) -> impl Stream<Item = Event> + Send + use<> {
    // The live channel is heard before the ring is read, so nothing sent between the two is lost.
    let live = BroadcastStream::new(self.live.subscribe());
    let kept: Vec<Event> = {
      let ring = self.ring.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

      ring.kept.iter().copied().filter(|event| event.cursor > cursor).collect()
    };
    let replayed = kept.last().map_or(cursor, |event| event.cursor);
    let live = live.filter_map(move |heard| async move {
      // A reader too slow for the channel loses what it lagged past; it reads again on what follows.
      heard.ok().filter(|event| event.cursor > replayed)
    });

    stream::iter(kept).chain(live)
  }
}

/// The stream itself.
pub struct EventStream;

impl Route for EventStream {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/events";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let cursor = call.header("last-event-id").and_then(|id| id.trim().parse::<u64>().ok()).unwrap_or(0);
    let events = call.standing().events.after(cursor).map(|event| Ok::<_, Infallible>(event.sent()));
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)).into_response())
  }
}

crate::route!(EventStream);
