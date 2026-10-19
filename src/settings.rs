//! The settings (`docs/api.md`, 「Settings」): the defaults built in from `settings.toml`, then the
//! settings folder's `settings.toml` and `settings.local.toml` where they stand, then the environment.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

use crate::review::words::Language;

/// The defaults, as the repository's own settings file states them.
const DEFAULTS: &str = include_str!("../settings.toml");
/// The settings file a deployment may give.
pub const SETTINGS_FILE: &str = "settings.toml";
/// The file a deployment overrides it with.
pub const LOCAL_SETTINGS_FILE: &str = "settings.local.toml";
/// The environment variable naming the settings folder.
pub const SETTINGS_FOLDER: &str = "BMO_REVIEW_SETTINGS";
/// What every environment variable that sets a key begins with.
pub const ENVIRONMENT_PREFIX: &str = "BMO_REVIEW";

/// Everything the service is started with.
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
  pub server: Server,
  pub store: Store,
  pub job: Peer,
  pub receiver: Peer,
  pub filing: Filing,
  pub limits: Limits,
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// Where the service answers.
  pub listen: SocketAddr,
  /// The bearer token every caller presents; an empty one lets no caller in.
  pub token: String,
}

/// The reviews' store.
#[derive(Clone, Debug, Deserialize)]
pub struct Store {
  /// The SQLite file; a relative one is read from where the process starts.
  pub path: PathBuf,
}

/// A unit this service calls: its base address and the token it takes.
#[derive(Clone, Debug, Deserialize)]
pub struct Peer {
  pub address: String,
  pub token: String,
}

/// What an item filed from a review is worded and requested with.
#[derive(Clone, Debug, Deserialize)]
pub struct Filing {
  /// The language of this repository's pack the item's parts are worded in.
  pub language: Language,
  /// The owner's name, filling `{owner}` in the item's parts.
  pub owner: String,
  /// The chat agent's name, filling `{agent}` in the item's parts.
  pub agent: String,
  /// Who the item is requested by.
  pub requester: String,
  /// How much of the reviewed lines an item carries, in characters.
  pub excerpt_clip: usize,
}

/// The ceilings of reading a checkout and of what a review and a filing take (`docs/code-review.md`).
#[derive(Clone, Debug, Deserialize)]
pub struct Limits {
  /// A file's body past this many bytes carries only its beginning.
  pub file_bytes: usize,
  /// A NUL within this many bytes from the start marks a file binary.
  pub binary_probe: usize,
  /// A tree's entries.
  pub tree_entries: usize,
  /// A commit list's commits.
  pub commits: usize,
  /// The content search's words, in characters.
  pub search_query: usize,
  /// The content search's lines from one file.
  pub search_per_file: usize,
  /// The content search's lines in all.
  pub search_total: usize,
  /// The content search's time.
  pub search_timeout_ms: u64,
  /// The definition search's lines read from one file.
  pub definition_scan_per_file: usize,
  /// The definitions kept from one file.
  pub definition_per_file: usize,
  /// The definitions in all.
  pub definition_total: usize,
  /// The definition search's time.
  pub definition_timeout_ms: u64,
  /// How much of a matched line comes back, in characters.
  pub match_line: usize,
  /// How much of the reviewed lines a review keeps, in characters.
  pub excerpt: usize,
  /// A turn's words, in characters.
  pub talk: usize,
  /// A filing's title, in characters.
  pub title: usize,
  /// A filing's what to do, in characters.
  pub to_do: usize,
  /// A filing's conclusion, in characters.
  pub conclusion: usize,
}

impl Settings {
  /// The settings of this process: its settings folder, then its environment.
  pub fn load() -> Result<Self, SettingsError> {
    Self::read(&folder(), None)
  }

  /// The settings read from `dir`, then from `environment` where it is given and from the process's own
  /// environment where it is not.
  pub fn read(dir: &Path, environment: Option<HashMap<String, String>>) -> Result<Self, SettingsError> {
    let environment = Environment::with_prefix(ENVIRONMENT_PREFIX)
      .prefix_separator("_")
      .separator("__")
      .try_parsing(true)
      .source(environment);

    Ok(
      Config::builder()
        .add_source(File::from_str(DEFAULTS, FileFormat::Toml))
        .add_source(File::from(dir.join(SETTINGS_FILE)).format(FileFormat::Toml).required(false))
        .add_source(File::from(dir.join(LOCAL_SETTINGS_FILE)).format(FileFormat::Toml).required(false))
        .add_source(environment)
        .build()?
        .try_deserialize()?,
    )
  }
}

/// The settings folder: the one the environment names, else the folder above the one holding the binary,
/// a deployment's root.
fn folder() -> PathBuf {
  std::env::var_os(SETTINGS_FOLDER)
    .filter(|named| !named.is_empty())
    .map(PathBuf::from)
    .or_else(|| {
      let binary = std::env::current_exe().ok()?;

      Some(binary.parent()?.parent()?.to_path_buf())
    })
    .unwrap_or_else(|| PathBuf::from("."))
}

/// Why the settings could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the settings could not be read: {0}")]
pub struct SettingsError(#[from] config::ConfigError);
