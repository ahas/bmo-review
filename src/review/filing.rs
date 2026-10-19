//! The item filed from a review (`docs/code-review.md`, 「The item's shape」): its parts composed from the review
//! and its thread in this repository's own words, handed to bmo-job as filing parts.

use super::records::{Review, Scope, Turn};
use super::words::Words;
use crate::clients::job::{Filing, FilingPart, PartKind};

/// The place a review is pinned to, in the words an item uses.
pub fn place(words: &Words, review: &Review) -> String {
  let places = &words.place;
  let line = match Scope::of(&review.scope) {
    Some(Scope::Line) => &places.line,
    Some(Scope::Range) => &places.range,
    Some(Scope::Folder) => &places.folder,
    Some(Scope::File) | None => &places.file,
  };
  let path = if review.path.is_empty() { places.whole.as_str() } else { review.path.as_str() };
  let number = |line: Option<i64>| line.map_or_else(|| "null".to_owned(), |line| line.to_string());

  words.fill(
    line,
    &[("path", path), ("start", &number(review.start_line)), ("end", &number(review.end_line))],
  )
}

/// A text cut to a number of characters, an ellipsis standing for what was left out.
fn clip(text: &str, limit: usize) -> String {
  if text.chars().count() <= limit {
    text.to_owned()
  } else {
    format!("{}…", text.chars().take(limit).collect::<String>())
  }
}

/// A moment as an instant: Unix seconds written as UTC with milliseconds.
fn instant(seconds: i64) -> String {
  chrono::DateTime::from_timestamp(seconds, 0)
    .unwrap_or_default()
    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
    .to_string()
}

/// What the owner wrote in the filing dialog.
pub struct Asked<'a> {
  pub title: &'a str,
  pub to_do: &'a str,
  pub conclusion: Option<&'a str>,
}

/// What a filing is worded with beyond the review: the worker's label, who requests it, how much of the
/// excerpt it carries, and when it is filed.
pub struct Context<'a> {
  pub label: &'a str,
  pub requester: &'a str,
  pub excerpt_clip: usize,
  pub filed_at: i64,
}

/// The item filed from a review: what the work is, what is to be done, the thread whole and in order, and what it
/// came to.
pub fn filing(
  words: &Words,
  review: &Review,
  thread: &[Turn],
  asked: &Asked<'_>,
  context: &Context<'_>,
) -> Filing {
  let filing = &words.filing;
  let id = review.id.to_string();
  let place = place(words, review);
  let source = words.fill(&filing.source, &[("id", &id)]);
  let owner_filed = words.fill(&filing.owner_filed, &[("id", &id), ("at", &instant(context.filed_at))]);
  let mut what = vec![
    words.fill(
      &filing.what,
      &[
        ("label", context.label),
        ("worker", &review.worker),
        ("root", &review.root),
        ("commit", &review.commit_sha),
        ("place", &place),
      ],
    ),
    filing.pinned.clone(),
  ];

  if let Some(excerpt) = &review.excerpt {
    what.extend([String::new(), filing.excerpt.clone(), clip(excerpt, context.excerpt_clip)]);
  }

  let part = |kind: PartKind, source: &str, text: &str| FilingPart {
    kind,
    source: source.to_owned(),
    text: text.to_owned(),
  };
  let mut parts = vec![
    part(PartKind::Title, &source, asked.title),
    part(PartKind::What, &source, &what.join("\n")),
    part(PartKind::Do, &owner_filed, asked.to_do.trim()),
  ];

  for turn in thread {
    let marks = [("id", id.as_str()), ("at", &instant(turn.said_at))];

    parts.push(if turn.by_owner() {
      part(PartKind::Instruction, &words.fill(&filing.owner_turn, &marks), &turn.text)
    } else {
      part(PartKind::Reply, &words.fill(&filing.agent_turn, &marks), &turn.text)
    });
  }

  parts.push(match asked.conclusion.map(str::trim).filter(|conclusion| !conclusion.is_empty()) {
    Some(conclusion) => part(PartKind::Conclusion, &owner_filed, conclusion),
    None => part(
      PartKind::Conclusion,
      &source,
      &words.fill(&filing.conclusion, &[("id", &id), ("label", context.label), ("place", &place)]),
    ),
  });

  Filing {
    worker: review.worker.clone(),
    requester: context.requester.to_owned(),
    parts,
  }
}
