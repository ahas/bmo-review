//! Filing an item from a review into the job service (`docs/code-review.md`, 「Filing an item」): the item is
//! created through the job service's client with the caller's title, brief and queue and the review as its
//! labels, and its number remembered under the review.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use bmo_job_client::{ClientFailure, Labels, NewItem};
use schemars::generate::SchemaGenerator;

use super::records;
use super::reviews::review_of;
use crate::events::{FILING_CREATED, REVIEW_CHANGED};
use crate::http::{Call, Refused, Route, answer};
use crate::openapi::{Stated, json_of, review_key, schema_of};
use crate::store::now_ms;
use crate::wire::{Filing, NewFiling, Review};

/// The labels an item filed from a review carries: the review it came from.
fn labels_of(review: &Review) -> Labels {
  Labels::from([
    ("reviewId".to_owned(), review.id.to_string()),
    ("projectName".to_owned(), review.project_name.clone()),
    ("revision".to_owned(), review.revision.clone()),
  ])
}

/// The job service's failure as this service answers it: its refusal under this service's own code, anything
/// else as a failure upstream.
fn refused(failure: &ClientFailure) -> Refused {
  match failure {
    ClientFailure::Answered {
      status,
      answer: Some(answer),
      ..
    } if (400..500).contains(status) => Refused::unprocessable(
      "filing-refused",
      format!("the job service refused the item with {status} {}: {}", answer.code, answer.error),
    ),
    other => Refused::upstream(other.to_string()),
  }
}

/// A text the job service takes, refused where it is empty.
fn given(field: &str, text: &str) -> Result<(), Refused> {
  if bmo_text::js::trim(text).is_empty() {
    Err(Refused::bad_request(format!("{field} is empty")))
  } else {
    Ok(())
  }
}

/// `POST /reviews/{reviewId}/filings`: an item filed from a review.
pub struct FileItem;

impl Route for FileItem {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/reviews/{reviewId}/filings";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let review = review_of(&mut call).await?;
    let asked: NewFiling = call.body().await?;

    given("title", &asked.title)?;
    given("brief", &asked.brief)?;
    given("queueName", &asked.queue_name)?;

    let standing = call.standing();
    let item = NewItem {
      title: asked.title,
      brief: asked.brief,
      queue_name: asked.queue_name,
      labels: labels_of(&review),
      ..NewItem::default()
    };
    let created = standing.job.create_item(&item).await.map_err(|failure| refused(&failure))?;
    let mut writing = standing.events.write().await;
    let mut transaction = records::begin(standing.store.pool()).await?;
    let filing = records::write_filing(&mut transaction, review.id, created.id, now_ms()).await?;
    let changed = records::review_in(&mut transaction, review.id)
      .await?
      .ok_or_else(|| Refused::failed("the review just filed from is gone"))?;

    writing.record(&mut transaction, FILING_CREATED, review.id, &filing).await?;
    writing.record(&mut transaction, REVIEW_CHANGED, review.id, &changed).await?;
    records::commit(transaction).await?;
    writing.announce();

    Ok(answer(StatusCode::CREATED, &filing))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "createFiling",
      tag: "reviews",
      summary: "File an item from a review into the job service.",
      parameters: vec![review_key()],
      body: Some(schema_of::<NewFiling>(generator)),
      answer: (201, json_of::<Filing>(generator)),
      codes: vec!["not-found", "filing-refused", "upstream-failed"],
    }
  }
}

crate::route!(FileItem);
