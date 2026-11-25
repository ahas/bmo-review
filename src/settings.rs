//! The settings (`docs/api.md`, 「Settings」): the defaults built in from `settings.toml`, then the
//! settings folder's `settings.toml` and `settings.local.toml` where they stand, then the environment.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

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
  /// The projects: each checkout that can be read, under the name it is asked by.
  #[serde(default)]
  pub projects: Vec<Project>,
  pub job: Peer,
  pub agency: Agency,
  pub console: Console,
  pub limits: Limits,
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// Where the service answers.
  pub listen: SocketAddr,
  /// The service token, the bearer every caller of the routes at their own paths presents; an empty one lets no
  /// caller in.
  pub token: String,
}

/// The store.
#[derive(Clone, Debug, Deserialize)]
pub struct Store {
  /// The SQLite file; a relative one is read from where the process starts.
  pub path: PathBuf,
}

/// The console face (`docs/api.md`, 「The console face」).
#[derive(Clone, Debug, Deserialize)]
pub struct Console {
  /// The console's credential; an empty one leaves the service with no console face.
  pub token: String,
  /// The folder the bundle's build writes; a relative one is read from where the process starts.
  pub folder: PathBuf,
}

/// One project: the name it is asked by and the absolute path of its checkout.
#[derive(Clone, Debug, Deserialize)]
pub struct Project {
  pub name: String,
  pub checkout: String,
}

/// A unit this service calls: its base address and the token it takes.
#[derive(Clone, Debug, Deserialize)]
pub struct Peer {
  pub address: String,
  pub token: String,
}

/// The agency, asked whose the repositories are for a member's call through the console (`docs/api.md`, 「The
/// console face」).
#[derive(Clone, Debug, Deserialize)]
pub struct Agency {
  /// Its base address; an empty one leaves no agency to ask.
  pub address: String,
  /// The file holding the token it takes, read once at start; a relative one is read from where the process
  /// starts, and an empty one leaves no agency to ask.
  pub token_file: PathBuf,
}

/// The ceilings of reading a checkout and of what a review and a comment take (`docs/code-review.md`).
#[derive(Clone, Debug, Deserialize)]
pub struct Limits {
  /// A file's body past this many bytes carries only its beginning.
  pub file_bytes: usize,
  /// A NUL within this many bytes from the start marks a file binary.
  pub binary_probe: usize,
  /// The most entries one page of a tree holds.
  pub tree_entries: usize,
  /// A commit's diff past this many bytes carries only its beginning.
  pub diff_bytes: usize,
  /// The content search's words, in characters.
  pub search_query: usize,
  /// The content search's lines from one file.
  pub search_per_file: usize,
  /// The content search's lines in all.
  pub search_total: usize,
  /// The definition search's lines read from one file.
  pub definition_scan_per_file: usize,
  /// The definitions kept from one file.
  pub definition_per_file: usize,
  /// The definitions in all.
  pub definition_total: usize,
  /// How much of a matched line comes back, in characters.
  pub match_line: usize,
  /// How much of the lines a comment points at it keeps, in characters.
  pub excerpt: usize,
  /// A review's title, in characters.
  pub title: usize,
  /// A comment's text, in characters.
  pub comment: usize,
  /// An author's name, in characters.
  pub author_name: usize,
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
