//! The routes of reviews and their comments (`docs/api.md`, 「Reviews」): listing, opening, reading and closing
//! a review, and reading and writing its comments, each place judged against the review's revision.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::projects::{self, unreadable};
use super::records::{self, Written};
use super::workspace::{cut_units, find_entry, head_of, read_tree_file, resolve_commit};
use crate::events::{COMMENT_CREATED, REVIEW_CHANGED, REVIEW_CREATED};
use crate::http::{Call, MOST_PAGE, PAGE, Refused, Route, Standing, answer, page_size};
use crate::openapi::{Stated, json_of, query, review_key, schema_of};
use crate::owners::Seen;
use crate::settings::Limits;
use crate::store::now_ms;
use crate::wire::{
  Comment, CommentQuery, EntryKind, NewComment, NewReview, Page, Review, ReviewChange, ReviewQuery,
  ReviewState,
};

/// A text a caller gives, refused where it is empty or past its ceiling.
pub fn required(field: &str, text: &str, most: usize) -> Result<(), Refused> {
  if bmo_text::js::trim(text).is_empty() {
    return Err(Refused::bad_request(format!("{field} is empty")));
  }

  if bmo_text::js::units(text) > most {
    return Err(Refused::bad_request(format!("{field} is past {most} characters")));
  }

  Ok(())
}

/// The review a route's path names, a review of a project the reader does not see being none.
pub async fn review_of(call: &mut Call) -> Result<Review, Refused> {
  let id = call.path_id("reviewId").await?;
  let seen = call.seen().await?;

  records::review(call.standing().store.pool(), id)
    .await?
    .filter(|review| seen.holds(&review.project_name))
    .ok_or_else(|| Refused::not_found(format!("no review has id {id}")))
}

/// `GET /reviews`: the reviews of the projects the reader sees a query narrows to, newest first.
pub struct Reviews;

impl Route for Reviews {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/reviews";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let query: ReviewQuery = call.query()?;
    let limit = page_size(query.limit, PAGE, MOST_PAGE)?;
    let seen = call.seen().await?;
    let mut items = records::reviews(call.standing().store.pool(), &query, &seen, limit).await?;
    let more = items.len() > limit;

    items.truncate(limit);

    Ok(answer(StatusCode::OK, &Page { items, more }))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listReviews",
      tag: "reviews",
      summary: "Reviews by project, revision and state, newest first.",
      parameters: query::<ReviewQuery>(generator),
      body: None,
      answer: (200, json_of::<Page<Review>>(generator)),
      codes: Vec::new(),
    }
  }
}

crate::route!(Reviews);

/// `POST /reviews`: a review opened on a revision of a project, which the revision is resolved in.
pub struct OpenReview;

impl Route for OpenReview {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/reviews";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let asked: NewReview = call.body().await?;
    let standing = call.standing();
    let limits = &standing.settings.limits;

    required("title", &asked.title, limits.title)?;
    required("authorName", &asked.author_name, limits.author_name)?;

    let seen = call.seen().await?;
    let checkout = projects::find(standing, &seen, &asked.project_name).ok_or_else(|| {
      Refused::unprocessable("unknown-project", format!("no project is named {}", asked.project_name))
    })?;
    let revision = projects::revision(&checkout, Some(&asked.revision)).await?.ok_or_else(|| {
      Refused::unprocessable(
        "unknown-revision",
        format!("revision {} is no commit of project {}", asked.revision, checkout.name),
      )
    })?;
    let mut writing = standing.events.write().await;
    let mut transaction = records::begin(standing.store.pool()).await?;
    let id = records::open_review(
      &mut transaction,
      &checkout.name,
      &revision,
      bmo_text::js::trim(&asked.title),
      bmo_text::js::trim(&asked.author_name),
      now_ms(),
    )
    .await?;
    let review = records::review_in(&mut transaction, id)
      .await?
      .ok_or_else(|| Refused::failed("the review just opened is gone"))?;

    writing.record(&mut transaction, REVIEW_CREATED, id, &review).await?;
    records::commit(transaction).await?;
    writing.announce();

    Ok(answer(StatusCode::CREATED, &review))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "createReview",
      tag: "reviews",
      summary: "Open a review of a revision.",
      parameters: Vec::new(),
      body: Some(schema_of::<NewReview>(generator)),
      answer: (201, json_of::<Review>(generator)),
      codes: vec!["unknown-project", "unknown-revision", "unreadable-checkout"],
    }
  }
}

crate::route!(OpenReview);

/// `GET /reviews/{reviewId}`: one review.
pub struct ReadReview;

impl Route for ReadReview {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/reviews/{reviewId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    Ok(answer(StatusCode::OK, &review_of(&mut call).await?))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readReview",
      tag: "reviews",
      summary: "One review.",
      parameters: vec![review_key()],
      body: None,
      answer: (200, json_of::<Review>(generator)),
      codes: vec!["not-found"],
    }
  }
}

crate::route!(ReadReview);

/// The state a review moves to on a change, or the conflict its states make of it (`docs/code-review.md`, 「Its
/// states」): closing an open or answered one, reopening a closed one into the state its last comment gives it.
fn moved(
  review: &Review,
  wanted: ReviewState,
  last_commenter: Option<&str>,
) -> Result<ReviewState, Refused> {
  match (review.state, wanted) {
    (ReviewState::Open | ReviewState::Answered, ReviewState::Closed) => Ok(ReviewState::Closed),
    (ReviewState::Closed, ReviewState::Open) => Ok(state_after(review, last_commenter)),
    (from, to) => Err(Refused::conflict(format!(
      "review {} cannot move from {} to {}; answered is reached by a comment",
      review.id,
      from.as_str(),
      to.as_str()
    ))),
  }
}

/// The state an open review stands in after a comment by `commenter`, or by nobody yet.
fn state_after(review: &Review, commenter: Option<&str>) -> ReviewState {
  match commenter {
    Some(commenter) if commenter != review.author_name => ReviewState::Answered,
    _ => ReviewState::Open,
  }
}

/// `PATCH /reviews/{reviewId}`: a review closed or reopened.
pub struct ChangeReview;

impl Route for ChangeReview {
  const METHOD: Method = Method::PATCH;
  const PATH: &'static str = "/reviews/{reviewId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = review_of(&mut call).await?.id;
    let change: ReviewChange = call.body().await?;
    let standing = call.standing();
    let mut writing = standing.events.write().await;
    let mut transaction = records::begin(standing.store.pool()).await?;
    let review = records::review_in(&mut transaction, id)
      .await?
      .ok_or_else(|| Refused::not_found(format!("no review has id {id}")))?;
    let last = records::last_commenter(&mut transaction, id).await?;
    let state = moved(&review, change.state, last.as_deref())?;

    records::set_state(&mut transaction, id, state, now_ms()).await?;

    let review = records::review_in(&mut transaction, id)
      .await?
      .ok_or_else(|| Refused::failed("the review just changed is gone"))?;

    writing.record(&mut transaction, REVIEW_CHANGED, id, &review).await?;
    records::commit(transaction).await?;
    writing.announce();

    Ok(answer(StatusCode::OK, &review))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "changeReview",
      tag: "reviews",
      summary: "Close or reopen a review.",
      parameters: vec![review_key()],
      body: Some(schema_of::<ReviewChange>(generator)),
      answer: (200, json_of::<Review>(generator)),
      codes: vec!["not-found", "conflict"],
    }
  }
}

crate::route!(ChangeReview);

/// `GET /reviews/{reviewId}/comments`: a review's comments, oldest first.
pub struct Comments;

impl Route for Comments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/reviews/{reviewId}/comments";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: CommentQuery = call.query()?;
    let limit = page_size(query.limit, PAGE, MOST_PAGE)?;
    let review = review_of(&mut call).await?;
    let mut items = records::comments(call.standing().store.pool(), review.id, &query, limit).await?;
    let more = items.len() > limit;

    items.truncate(limit);

    Ok(answer(StatusCode::OK, &Page { items, more }))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listComments",
      tag: "reviews",
      summary: "A review's comments, oldest first.",
      parameters: [vec![review_key()], query::<CommentQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<Page<Comment>>(generator)),
      codes: vec!["not-found"],
    }
  }
}

crate::route!(Comments);

/// A comment's place as the review's revision holds it: its path, its lines and the excerpt of those lines.
struct Place {
  path: Option<String>,
  line: Option<u32>,
  end_line: Option<u32>,
  excerpt: Option<String>,
}

fn no_such_place(error: impl Into<String>) -> Refused {
  Refused::unprocessable("no-such-place", error)
}

/// The place a comment asks for, judged against the review's revision (`docs/code-review.md`, 「Comments」).
async fn place_of(
  standing: &Standing,
  review: &Review,
  asked: &NewComment,
  limits: &Limits,
) -> Result<Place, Refused> {
  match (asked.path.as_deref(), asked.line, asked.end_line) {
    (None, Some(_), _) => return Err(Refused::bad_request("line is given without path")),
    (_, None, Some(_)) => return Err(Refused::bad_request("endLine is given without line")),
    (_, Some(0), _) => return Err(Refused::bad_request("line counts from 1")),
    (_, Some(line), Some(end)) if end <= line => {
      return Err(Refused::bad_request("endLine is not past line"));
    }
    _ => {}
  }

  let Some(path) = asked.path.clone() else {
    return Ok(Place {
      path: None,
      line: None,
      end_line: None,
      excerpt: None,
    });
  };
  // The review's own project, which the reader sees since they reached the review.
  let checkout = projects::find(standing, &Seen::All, &review.project_name).ok_or_else(|| {
    Refused::unprocessable("unknown-project", format!("no project is named {}", review.project_name))
  })?;

  head_of(&checkout.root).await.map_err(|state| unreadable(&checkout.name, state))?;

  let revision = resolve_commit(&checkout.root, &review.revision)
    .await
    .ok_or_else(|| no_such_place(format!("revision {} can no longer be read", review.revision)))?;
  let entry = find_entry(&checkout.root, &revision, &path).await.ok_or_else(|| {
    no_such_place(format!("{path} is no file or folder of revision {revision} that may be shown"))
  })?;
  let Some(line) = asked.line else {
    return match entry.kind {
      EntryKind::File | EntryKind::Folder => Ok(Place {
        path: Some(path),
        line: None,
        end_line: None,
        excerpt: None,
      }),
      EntryKind::Link | EntryKind::Submodule => {
        Err(no_such_place(format!("{path} is a link or a submodule")))
      }
    };
  };

  if entry.kind != EntryKind::File {
    return Err(no_such_place(format!("{path} is no file, so it has no lines")));
  }

  let file = read_tree_file(&checkout.root, &revision, &path, limits)
    .await
    .ok_or_else(|| no_such_place(format!("{path} cannot be read at revision {revision}")))?;
  let body = file.text.ok_or_else(|| no_such_place(format!("{path} has no body to point at")))?;
  let lines: Vec<&str> = body.split('\n').collect();
  let last = asked.end_line.unwrap_or(line);

  if last as usize > lines.len() {
    return Err(no_such_place(format!("line {last} lies past the last line of {path}, {}", lines.len())));
  }

  let excerpt = cut_units(&lines[line as usize - 1..last as usize].join("\n"), limits.excerpt);

  Ok(Place {
    path: Some(path),
    line: Some(line),
    end_line: asked.end_line,
    excerpt: Some(excerpt),
  })
}

/// `POST /reviews/{reviewId}/comments`: a comment written on a place of the review's revision, moving the
/// review's state.
pub struct WriteComment;

impl Route for WriteComment {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/reviews/{reviewId}/comments";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let review = review_of(&mut call).await?;
    let asked: NewComment = call.body().await?;
    let standing = call.standing();
    let limits = &standing.settings.limits;

    required("text", &asked.text, limits.comment)?;
    required("authorName", &asked.author_name, limits.author_name)?;

    let place = place_of(standing, &review, &asked, limits).await?;
    let author_name = bmo_text::js::trim(&asked.author_name);
    let at = now_ms();
    let mut writing = standing.events.write().await;
    let mut transaction = records::begin(standing.store.pool()).await?;
    let review = records::review_in(&mut transaction, review.id)
      .await?
      .ok_or_else(|| Refused::not_found(format!("no review has id {}", review.id)))?;
    let comment = records::write_comment(
      &mut transaction,
      &Written {
        review_id: review.id,
        path: place.path.as_deref(),
        line: place.line,
        end_line: place.end_line,
        excerpt: place.excerpt.as_deref(),
        text: &asked.text,
        author_name,
        at,
      },
    )
    .await?;
    let state = match review.state {
      ReviewState::Closed => ReviewState::Closed,
      _ => state_after(&review, Some(author_name)),
    };

    records::set_state(&mut transaction, review.id, state, at).await?;

    let changed = records::review_in(&mut transaction, review.id)
      .await?
      .ok_or_else(|| Refused::failed("the review just commented on is gone"))?;

    writing.record(&mut transaction, COMMENT_CREATED, review.id, &comment).await?;
    writing.record(&mut transaction, REVIEW_CHANGED, review.id, &changed).await?;
    records::commit(transaction).await?;
    writing.announce();

    Ok(answer(StatusCode::CREATED, &comment))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "createComment",
      tag: "reviews",
      summary: "Comment on a review: on the whole, a folder, a file, a line or a range of its revision.",
      parameters: vec![review_key()],
      body: Some(schema_of::<NewComment>(generator)),
      answer: (201, json_of::<Comment>(generator)),
      codes: vec![
        "not-found",
        "no-such-place",
        "unknown-project",
        "unreadable-checkout",
        "upstream-failed",
      ],
    }
  }
}

crate::route!(WriteComment);
