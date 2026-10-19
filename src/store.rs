//! The reviews' store: one SQLite file, opened once as the service starts and shared by every request
//! (`docs/code-review.md`, 「The review table」).

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

// How long a statement waits on another writer before it gives up: requests write the one file at once.
const BUSY: Duration = Duration::from_secs(5);

/// The store, open.
#[derive(Clone, Debug)]
pub struct Store {
  pool: SqlitePool,
}

impl Store {
  /// The store at `path`, made where it does not stand yet, its folder with it, and migrated.
  pub async fn open(path: &Path) -> Result<Self, StoreError> {
    if let Some(folder) = path.parent().filter(|folder| !folder.as_os_str().is_empty()) {
      std::fs::create_dir_all(folder)
        .map_err(|why| StoreError::Folder(folder.display().to_string(), why))?;
    }

    let options = SqliteConnectOptions::from_str("sqlite:")?
      .filename(path)
      .create_if_missing(true)
      .journal_mode(SqliteJournalMode::Wal)
      .busy_timeout(BUSY);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(Self { pool })
  }

  /// The connection pool the store is read and written through.
  pub fn pool(&self) -> &SqlitePool {
    &self.pool
  }
}

/// Why the store could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
  #[error("the store's folder {0} could not be made: {1}")]
  Folder(String, std::io::Error),
  #[error("the store could not be opened: {0}")]
  Open(#[from] sqlx::Error),
  #[error("the store could not be migrated: {0}")]
  Migrate(#[from] sqlx::migrate::MigrateError),
}
