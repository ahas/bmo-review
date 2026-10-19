//! The routes of the reviews (`docs/api.md`, 「Reviews」): the reviews of a worker, making one, one review with
//! its thread, where its talk stands, a turn of the talk and an item filed from it.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};

use super::filing::{self, Asked, Context};
use super::records::{self, NewReview, Review, Scope, Speaker};
use super::request::{checkouts, require_checkout, require_commit};
use super::workspace::{EntryKind, cut_units, find_entry, read_tree_file, resolve_commit};
use crate::clients::receiver::{Talk, Topic, Turn as TalkTurn};
use crate::events::Kind;
use crate::http::{Call, Refused, Route, Standing, answer, whole};

fn refused(status: StatusCode, code: &'static str, error: &str) -> Refused {
  Refused::new(status, code, error)
}

fn no_such_review() -> Refused {
  refused(StatusCode::NOT_FOUND, "no_such_review", "no review has that number")
}

fn busy() -> Refused {
  refused(StatusCode::CONFLICT, "review_busy", "a turn of that review is out")
}

/// The words a body names under `name`, trimmed; empty where it names none.
fn trimmed(body: &Value, name: &str) -> String {
  bmo_markup::js::trim(body.get(name).and_then(Value::as_str).unwrap_or_default()).to_owned()
}

/// Words over a ceiling, counted in UTF-16 units as the screen counts them.
fn over(text: &str, limit: usize) -> bool {
  bmo_markup::js::units(text) > limit
}

/// The review number a route's path names.
async fn review_id(call: &mut Call) -> Result<i64, Refused> {
  let params = call.path_params().await?;

  params
    .get("id")
    .and_then(|id| id.parse::<i64>().ok())
    .filter(|id| *id >= 1)
    .ok_or_else(no_such_review)
}

/// One turn of a review's talk: the owner's words written at once, the turn put to the receiver behind them, and
/// its answer or its failure written when it ends. Refused while a turn of the review is out.
async fn take_turn(standing: &Standing, review: &Review, said: &str) -> Result<(), Refused> {
  let earlier = records::thread(standing.store.pool(), review.id).await?;

  if !standing.talking.begin(review.id) {
    return Err(busy());
  }

  if let Err(why) =
    records::say(standing.store.pool(), review.id, Speaker::Owner, said, records::now()).await
  {
    standing.talking.end(review.id, None);
    return Err(why);
  }

  standing.events.send(Kind::Talk, review.id);

  let label = match standing.job.worker(&review.worker).await {
    Ok(Some(worker)) => worker.label.unwrap_or(worker.name),
    _ => review.worker.clone(),
  };
  let talk = Talk {
    topic: Topic {
      kind: "code-review",
      id: review.id,
      worker: review.worker.clone(),
      label,
      root: review.root.clone(),
      commit: review.commit_sha.clone(),
      scope: review.scope.clone(),
      path: review.path.clone(),
      start_line: review.start_line,
      end_line: review.end_line,
      excerpt: review.excerpt.clone(),
    },
    turns: earlier
      .iter()
      .map(|turn| TalkTurn {
        owner: turn.by_owner(),
        text: turn.text.clone(),
      })
      .collect(),
    said: said.to_owned(),
  };
  let standing = standing.clone();
  let id = review.id;

  tokio::spawn(async move {
    let failure = match standing.receiver.talk(&talk).await {
      Ok(answer) => {
        match records::say(standing.store.pool(), id, Speaker::Agent, &answer, records::now()).await {
          Ok(()) => None,
          Err(why) => {
            eprintln!("review {id}: the answer could not be written: {}", why.error);
            Some("talk_failed")
          }
        }
      }
      Err(failure) => Some(failure.code()),
    };

    standing.talking.end(id, failure);
    standing.events.send(Kind::TalkEnded { failure }, id);
  });

  Ok(())
}

/// Every review of one worker, newest first; the screen sorts them onto its files and folders.
pub struct Reviews;

impl Route for Reviews {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/reviews";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let summaries = records::summaries(call.standing().store.pool(), &checkout.name).await?;

    Ok(answer(StatusCode::OK, json!(summaries)))
  }
}

crate::route!(Reviews);

/// A review made on a place of one commit, with the owner's first words under it. The place is judged against the
/// commit's own tree — a line review's lines lie inside the file, a file review's path is a file and a folder
/// review's a folder — and a line review's lines are read out of that commit as its excerpt. The first words
/// become the thread's first turn, answered as every later one is.
pub struct MakeReview;

impl Route for MakeReview {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/api/code/reviews";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let body = call.body().await?;
    let standing = call.standing().clone();
    let limits = &standing.settings.limits;
    let checkout = require_checkout(&standing, body.get("project").and_then(Value::as_str)).await?;
    let text = trimmed(&body, "text");

    if text.is_empty() {
      return Err(refused(StatusCode::BAD_REQUEST, "review_empty", "a review's first words are empty"));
    }

    if over(&text, limits.talk) {
      return Err(
        refused(StatusCode::BAD_REQUEST, "too_long", "the words are over the talk's ceiling")
          .with("limit", limits.talk),
      );
    }

    let scope = body.get("scope").and_then(Value::as_str).and_then(Scope::of).ok_or_else(|| {
      refused(StatusCode::BAD_REQUEST, "bad_review_place", "scope is not line, range, file or folder")
    })?;
    let commit =
      require_commit(&checkout, Some(body.get("commit").and_then(Value::as_str).unwrap_or_default()))
        .await?;
    let path = body.get("path").and_then(Value::as_str).unwrap_or_default().to_owned();
    let wanted = if scope == Scope::Folder { EntryKind::Folder } else { EntryKind::File };

    if find_entry(&checkout.root, &commit, &path)
      .await
      .is_none_or(|entry| entry.kind != wanted)
    {
      return Err(if scope == Scope::Folder {
        refused(StatusCode::NOT_FOUND, "no_such_folder", "no folder of that commit by that path")
      } else {
        refused(StatusCode::NOT_FOUND, "no_such_file", "no file of that commit that may be shown")
      });
    }

    let (mut start_line, mut end_line, mut excerpt) = (None, None, None);

    if scope.lined() {
      let line = |name: &str| body.get(name).and_then(whole).filter(|line| *line >= 1);
      let from = line("startLine");
      let to = if scope == Scope::Line { from } else { line("endLine") };
      let (Some(from), Some(to)) = (from, to) else {
        return Err(refused(
          StatusCode::BAD_REQUEST,
          "bad_review_line",
          "the lines are not whole numbers from 1",
        ));
      };

      if scope == Scope::Range && from >= to {
        return Err(refused(
          StatusCode::BAD_REQUEST,
          "bad_review_line",
          "a range's start is not before its end",
        ));
      }

      let file = read_tree_file(&checkout.root, &commit, &path, limits).await;
      let Some(body_text) = file.and_then(|file| file.text) else {
        return Err(refused(
          StatusCode::CONFLICT,
          "no_line_review",
          "a line review of a file with no body",
        ));
      };
      let lines: Vec<&str> = body_text.split('\n').collect();

      if to as usize > lines.len() {
        return Err(refused(StatusCode::BAD_REQUEST, "no_such_line", "a line lies past the file's last"));
      }

      start_line = Some(from);
      end_line = Some(to);
      excerpt = Some(cut_units(&lines[from as usize - 1..to as usize].join("\n"), limits.excerpt));
    }

    let root = checkout.root.to_string_lossy().into_owned();
    let id = records::make(
      standing.store.pool(),
      &NewReview {
        worker: &checkout.name,
        root: &root,
        commit: &commit,
        scope,
        path: &path,
        start_line,
        end_line,
        excerpt: excerpt.as_deref(),
        created_at: records::now(),
      },
    )
    .await?;

    standing.events.send(Kind::Review, id);

    if let Some(review) = records::review(standing.store.pool(), id).await? {
      // The first words' turn is taken as every later one; a turn refused here still leaves the review made.
      let _ = take_turn(&standing, &review, &text).await;
    }

    Ok(answer(StatusCode::ACCEPTED, json!({ "ok": true, "review": id })))
  }
}

crate::route!(MakeReview);

/// One review with its thread and where its talk stands. Whether its commit can still be read is asked of the
/// worker's checkout as it is now; a worker that is gone reads nothing.
pub struct OneReview;

impl Route for OneReview {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/reviews/{id}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = review_id(&mut call).await?;
    let standing = call.standing();
    let summary = records::summary(standing.store.pool(), id).await?.ok_or_else(no_such_review)?;
    let checkout = checkouts(standing).await?.into_iter().find(|checkout| checkout.name == summary.worker);
    let readable = match checkout {
      Some(checkout) => resolve_commit(&checkout.root, &summary.commit).await.is_some(),
      None => false,
    };
    let turns = records::thread(standing.store.pool(), id).await?;
    let state = standing.talking.state(id);
    let mut review = json!(summary);

    review["excerpt"] = json!(summary.excerpt);

    Ok(answer(
      StatusCode::OK,
      json!({
        "review": review,
        "readable": readable,
        "turns": turns,
        "answering": state.answering,
        "failure": state.failure,
      }),
    ))
  }
}

crate::route!(OneReview);

/// Where the talk under a review stands.
pub struct TalkState;

impl Route for TalkState {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/reviews/{id}/talk";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = review_id(&mut call).await?;

    Ok(answer(StatusCode::OK, json!(call.standing().talking.state(id))))
  }
}

crate::route!(TalkState);

/// One turn of the talk under a review: the owner's words written at once and the turn put to the receiver; the
/// answer arrives as an event of the thread moving.
pub struct TalkTurnRoute;

impl Route for TalkTurnRoute {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/api/code/reviews/{id}/talk";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = review_id(&mut call).await?;
    let body = call.body().await?;
    let standing = call.standing().clone();
    let limit = standing.settings.limits.talk;
    let text = trimmed(&body, "text");

    if text.is_empty() {
      return Err(refused(StatusCode::BAD_REQUEST, "talk_empty", "a turn's words are empty"));
    }

    if over(&text, limit) {
      return Err(
        refused(StatusCode::BAD_REQUEST, "too_long", "the words are over the talk's ceiling")
          .with("limit", limit),
      );
    }

    let review = records::review(standing.store.pool(), id).await?.ok_or_else(no_such_review)?;

    take_turn(&standing, &review, &text).await?;

    Ok(answer(StatusCode::ACCEPTED, json!({ "ok": true })))
  }
}

crate::route!(TalkTurnRoute);

/// An item filed from a review: composed from the review and its whole thread, assigned to the review's worker and
/// left waiting for approval; refused while a turn of the review is out and for a worker gone from the catalogue.
pub struct FileItem;

impl Route for FileItem {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/api/code/reviews/{id}/todo";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = review_id(&mut call).await?;
    let body = call.body().await?;
    let standing = call.standing().clone();
    let limits = &standing.settings.limits;
    let (title, to_do, conclusion) =
      (trimmed(&body, "title"), trimmed(&body, "toDo"), trimmed(&body, "conclusion"));

    if title.is_empty() || over(&title, limits.title) {
      return Err(
        refused(StatusCode::BAD_REQUEST, "title_too_long", "the title is empty or over its ceiling")
          .with("limit", limits.title),
      );
    }

    if to_do.is_empty() || over(&to_do, limits.to_do) {
      return Err(
        refused(StatusCode::BAD_REQUEST, "todo_too_long", "what to do is empty or over its ceiling")
          .with("limit", limits.to_do),
      );
    }

    if over(&conclusion, limits.conclusion) {
      return Err(
        refused(StatusCode::BAD_REQUEST, "conclusion_too_long", "the conclusion is over its ceiling")
          .with("limit", limits.conclusion),
      );
    }

    let review = records::review(standing.store.pool(), id).await?.ok_or_else(no_such_review)?;

    if standing.talking.state(id).answering {
      return Err(busy());
    }

    let worker = standing
      .job
      .worker(&review.worker)
      .await?
      .filter(|worker| worker.cwd.as_deref().is_some_and(|cwd| !cwd.is_empty()))
      .ok_or_else(|| {
        refused(StatusCode::CONFLICT, "no_worker", "the review's worker is gone from the catalogue")
          .with("worker", &review.worker)
      })?;
    let thread = records::thread(standing.store.pool(), id).await?;
    let label = worker.label.unwrap_or(worker.name);
    let item = filing::filing(
      &standing.words,
      &review,
      &thread,
      &Asked {
        title: &title,
        to_do: &to_do,
        conclusion: Some(conclusion.as_str()),
      },
      &Context {
        label: &label,
        requester: &standing.settings.filing.requester,
        excerpt_clip: standing.settings.filing.excerpt_clip,
        filed_at: records::now(),
      },
    );
    let raised = standing.job.file(&item).await?;

    records::filed(standing.store.pool(), id, raised).await?;
    standing.events.send(Kind::Review, id);

    Ok(answer(StatusCode::OK, json!({ "ok": true, "raised": raised })))
  }
}

crate::route!(FileItem);
