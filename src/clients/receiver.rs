//! The receiver, as its `docs/boundary-api.md` states `POST /talks`: the talk under a review, a round of the
//! owner's private chat on the chat agent's session, composed by the receiver from the review's facts.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

// Whether the receiver's boundary document lists a `code-review` kind of a talk's topic. It lists only `item` and
// `document` (`docs/api.md`, 「What the siblings lack」), so no talk is sent until it does, and every turn ends
// with `receiver_talk_unsupported`.
const CODE_REVIEW_TOPIC_DOCUMENTED: bool = false;

/// The review's facts, the topic the receiver words the turn's head from.
#[derive(Clone, Debug, Serialize)]
pub struct Topic {
  pub kind: &'static str,
  pub id: i64,
  pub worker: String,
  pub label: String,
  /// The checkout's absolute path.
  pub root: String,
  pub commit: String,
  pub scope: String,
  pub path: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub excerpt: Option<String>,
}

/// One earlier turn of the thread.
#[derive(Clone, Debug, Serialize)]
pub struct Turn {
  pub owner: bool,
  pub text: String,
}

/// A talk: the topic, the thread's earlier turns and the words just said.
#[derive(Clone, Debug, Serialize)]
pub struct Talk {
  pub topic: Topic,
  pub turns: Vec<Turn>,
  pub said: String,
}

/// Why a talk came back with nothing, as the thread's failure (`docs/api.md`, 「Refusals」).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
  /// The receiver carries no talk topic for a code review.
  Unsupported,
  /// The agent rests, or the owner's chat has fallen silent.
  Resting,
  /// The provider's limit is reached.
  Limit,
  /// The provider is out.
  Outage,
  /// The talk came back with no answer.
  NoAnswer,
  /// Any other refusal, or the receiver could not be reached.
  Failed,
}

impl Failure {
  /// The code the thread keeps the failure under.
  pub fn code(self) -> &'static str {
    match self {
      Self::Unsupported => "receiver_talk_unsupported",
      Self::Resting => "resting",
      Self::Limit => "limit",
      Self::Outage => "outage",
      Self::NoAnswer => "no_answer",
      Self::Failed => "talk_failed",
    }
  }

  fn of(code: &str) -> Self {
    match code {
      "resting" => Self::Resting,
      "limit" => Self::Limit,
      "outage" => Self::Outage,
      _ => Self::Failed,
    }
  }
}

#[derive(Deserialize)]
struct Answered {
  answer: Option<String>,
}

#[derive(Deserialize)]
struct Refusal {
  code: String,
}

/// The receiver, at its base address with the token it takes.
#[derive(Clone, Debug)]
pub struct Receiver {
  client: Client,
  address: String,
  token: String,
}

impl Receiver {
  pub fn new(address: &str, token: &str) -> Self {
    Self {
      client: Client::new(),
      address: address.trim_end_matches('/').to_owned(),
      token: token.to_owned(),
    }
  }

  /// `POST /talks`: the agent's answer, or why it gave none.
  pub async fn talk(&self, talk: &Talk) -> Result<String, Failure> {
    if !CODE_REVIEW_TOPIC_DOCUMENTED {
      return Err(Failure::Unsupported);
    }

    let mut asked = self.client.post(format!("{}/talks", self.address)).json(talk);

    if !self.token.is_empty() {
      asked = asked.bearer_auth(&self.token);
    }

    let answer = asked.send().await.map_err(|_| Failure::Failed)?;

    if answer.status() == StatusCode::OK {
      return answer
        .json::<Answered>()
        .await
        .map_err(|_| Failure::Failed)?
        .answer
        .ok_or(Failure::NoAnswer);
    }

    let code = answer.json::<Refusal>().await.map(|refusal| refusal.code).unwrap_or_default();

    Err(Failure::of(&code))
  }
}
