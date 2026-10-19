//! git, run in a worker's checkout (`docs/code-review.md`, 「How git is called」): an argument list with no
//! shell, pathspecs taken literally, output read up to a cap and the run held to a time limit.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::{Instant, timeout_at};

/// The cap and the time limit a run takes where it names none.
pub const OUTPUT_CAP: usize = 64 * 1024 * 1024;
pub const TIME_LIMIT: Duration = Duration::from_secs(20);

// The exit code a run stands under when it was ended by a signal or by its time limit: a failure, and one a
// search reads as a run that did not finish.
const ENDED: i32 = 255;
// How much is read from the output at a time.
const CHUNK: usize = 64 * 1024;

/// One run's end: its exit code, what it wrote, and whether the cap left the rest unread.
#[derive(Debug)]
pub struct Run {
  pub code: i32,
  pub out: Vec<u8>,
  pub cut: bool,
}

impl Run {
  fn failed() -> Self {
    Self {
      code: ENDED,
      out: Vec::new(),
      cut: false,
    }
  }

  pub fn ok(&self) -> bool {
    self.code == 0
  }

  /// The output as text, a byte that is no UTF-8 read as the replacement character.
  pub fn text(&self) -> String {
    String::from_utf8_lossy(&self.out).into_owned()
  }
}

/// How reading the output ended.
enum Read {
  /// The output closed: git is done writing.
  Closed,
  /// The cap was reached; `cut` where more stood beyond it.
  Capped { cut: bool },
  /// The time limit came first.
  Late,
}

/// One git command in `root`, its output read up to `cap` bytes and the run ended at `limit`. A run that
/// reached its cap is ended there and stands as a success; one ended by its limit keeps what it wrote and
/// stands as a failure.
pub async fn git(root: &Path, args: &[&str], cap: usize, limit: Duration) -> Run {
  let deadline = Instant::now() + limit;
  let mut command = Command::new("git");

  command
    .args(["--no-pager", "--literal-pathspecs", "-c", "core.quotePath=false"])
    .args(args)
    .current_dir(root)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .env("GIT_OPTIONAL_LOCKS", "0")
    .env("GIT_TERMINAL_PROMPT", "0")
    .env("LC_ALL", "C")
    .kill_on_drop(true);

  let Ok(mut child) = command.spawn() else {
    return Run::failed();
  };
  let Some(mut stdout) = child.stdout.take() else {
    end(&mut child).await;
    return Run::failed();
  };
  let mut out = Vec::new();
  let mut chunk = vec![0; CHUNK];
  let read = loop {
    let count = match timeout_at(deadline, stdout.read(&mut chunk)).await {
      Err(_) => break Read::Late,
      Ok(Err(_) | Ok(0)) => break Read::Closed,
      Ok(Ok(count)) => count,
    };
    let room = cap - out.len();

    if count >= room {
      out.extend_from_slice(&chunk[..room]);
      break Read::Capped { cut: count > room };
    }

    out.extend_from_slice(&chunk[..count]);
  };

  match read {
    Read::Capped { cut } => {
      end(&mut child).await;
      Run { code: 0, out, cut }
    }
    Read::Late => {
      end(&mut child).await;
      Run {
        code: ENDED,
        out,
        cut: false,
      }
    }
    Read::Closed => {
      let code = match timeout_at(deadline, child.wait()).await {
        Ok(Ok(status)) => status.code().unwrap_or(ENDED),
        _ => {
          end(&mut child).await;
          ENDED
        }
      };

      Run {
        code,
        out,
        cut: false,
      }
    }
  }
}

async fn end(child: &mut Child) {
  let _ = child.kill().await;
}
