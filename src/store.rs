//! The store: one SQLite file of reviews, comments, filings and events, opened once as the service starts and
//! shared by every request (`docs/code-review.md`, 「Reviews」).

use std::ffi::{c_int, c_void};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

// How often a statement that finds another writer holding the file looks again: requests write the one file at
// once, and a statement waits for the lock however long the other holds it.
const LOOK_AGAIN: Duration = Duration::from_millis(5);

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
      .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
      .after_connect(|connection, _| {
        Box::pin(async move {
          let mut handle = connection.lock_handle().await?;

          // SAFETY: the handle is this connection's open database, held locked for the call, and the handler
          // reads no data through its pointer.
          unsafe {
            libsqlite3_sys::sqlite3_busy_handler(
              handle.as_raw_handle().as_ptr(),
              Some(wait_for_lock),
              std::ptr::null_mut(),
            );
          }

          Ok(())
        })
      })
      .connect_with(options)
      .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(Self { pool })
  }

  /// The connection pool the store is read and written through.
  pub fn pool(&self) -> &SqlitePool {
    &self.pool
  }
}

// SQLite's busy handler: the lock is still held, so wait a moment and ask SQLite to try again. It runs on the
// connection's own worker thread, which does nothing else meanwhile.
extern "C" fn wait_for_lock(_: *mut c_void, _: c_int) -> c_int {
  std::thread::sleep(LOOK_AGAIN);
  1
}

/// Now, in epoch milliseconds: the time a row is written with.
pub fn now_ms() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
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
