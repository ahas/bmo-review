//! Where each review's talk stands (`docs/code-review.md`, 「The same road as other talks」): one turn out at a time
//! per review, and the code its last turn came back with, held while the service runs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Where one review's talk stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TalkState {
  /// A turn of the review is out.
  pub answering: bool,
  /// The code the last turn came back with, none where it was answered.
  pub failure: Option<&'static str>,
}

/// Every review's talk.
#[derive(Clone, Debug, Default)]
pub struct Talking {
  states: Arc<Mutex<HashMap<i64, TalkState>>>,
}

impl Talking {
  fn with<T>(&self, then: impl FnOnce(&mut HashMap<i64, TalkState>) -> T) -> T {
    then(&mut self.states.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
  }

  /// Where a review's talk stands.
  pub fn state(&self, review: i64) -> TalkState {
    self.with(|states| states.get(&review).copied().unwrap_or_default())
  }

  /// Marks a turn of a review out; false where one is out already.
  pub fn begin(&self, review: i64) -> bool {
    self.with(|states| {
      let state = states.entry(review).or_default();

      if state.answering {
        return false;
      }

      *state = TalkState {
        answering: true,
        failure: None,
      };
      true
    })
  }

  /// Marks a review's turn ended, with the code it came back with.
  pub fn end(&self, review: i64, failure: Option<&'static str>) {
    self.with(|states| {
      states.insert(
        review,
        TalkState {
          answering: false,
          failure,
        },
      );
    });
  }
}
