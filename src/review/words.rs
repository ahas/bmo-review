//! This repository's own words for an item filed from a review (`docs/code-review.md`, 「The item's shape」): the
//! pack of one language, read once from `lang/<language>/`, its lines filled by `{mark}`.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::settings::Filing;

/// The words a place is named in.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceWords {
  pub whole: String,
  pub line: String,
  pub range: String,
  pub file: String,
  pub folder: String,
}

/// The words an item's parts are made of.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilingWords {
  pub source: String,
  pub owner_filed: String,
  pub owner_turn: String,
  pub agent_turn: String,
  pub what: String,
  pub pinned: String,
  pub excerpt: String,
  pub conclusion: String,
}

/// A language this repository holds a pack for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Language {
  #[serde(rename = "ko")]
  Korean,
}

impl Language {
  fn place(self) -> &'static str {
    match self {
      Self::Korean => include_str!("../../lang/ko/place.yaml"),
    }
  }

  fn filing(self) -> &'static str {
    match self {
      Self::Korean => include_str!("../../lang/ko/filing.yaml"),
    }
  }
}

/// One language's pack, with the names the settings give the owner and the agent.
#[derive(Clone, Debug)]
pub struct Words {
  pub place: PlaceWords,
  pub filing: FilingWords,
  owner: String,
  agent: String,
}

fn read<T: DeserializeOwned>(language: Language, part: &str, text: &str) -> Result<T, String> {
  serde_saphyr::from_str(text)
    .map_err(|why| format!("the {language:?} pack's {part} part does not read: {why}"))
}

impl Words {
  /// The pack of the language the settings name.
  pub fn of(filing: &Filing) -> Result<Self, String> {
    let language = filing.language;

    Ok(Self {
      place: read(language, "place", language.place())?,
      filing: read(language, "filing", language.filing())?,
      owner: filing.owner.clone(),
      agent: filing.agent.clone(),
    })
  }

  /// A line with its marks filled: `{owner}` and `{agent}` by the settings' names, every other by `marks`.
  pub fn fill(&self, line: &str, marks: &[(&str, &str)]) -> String {
    let named = [("owner", self.owner.as_str()), ("agent", self.agent.as_str())];

    fill(line, named.iter().chain(marks))
  }
}

/// A line with each `{name}` the marks name put in its place, once, left to right; a mark no value names stays.
fn fill<'a>(line: &str, marks: impl Iterator<Item = &'a (&'a str, &'a str)> + Clone) -> String {
  let mut filled = String::with_capacity(line.len());
  let mut rest = line;

  while let Some(open) = rest.find('{') {
    filled.push_str(&rest[..open]);

    let after = &rest[open + 1..];
    let value = after.find('}').and_then(|close| {
      marks
        .clone()
        .find(|(name, _)| *name == &after[..close])
        .map(|(_, value)| (close, *value))
    });

    match value {
      Some((close, value)) => {
        filled.push_str(value);
        rest = &after[close + 1..];
      }
      None => {
        filled.push('{');
        rest = after;
      }
    }
  }

  filled.push_str(rest);
  filled
}
