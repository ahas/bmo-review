//! The checkouts the code review reads (`docs/code-review.md`, 「Where the projects come from」): one for each
//! worker of bmo-job's catalogue naming a working directory, the first worker naming one directory standing for
//! it, and a support worker standing on its base's.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::clients::job::Worker;

/// One worker's checkout.
#[derive(Clone, Debug)]
pub struct Checkout {
  /// The worker's name, the project's.
  pub name: String,
  /// What the worker is shown as: its label, its name where it has none.
  pub label: String,
  /// The directory, standing on its own.
  pub root: PathBuf,
}

impl Checkout {
  fn of(worker: &Worker) -> Option<Self> {
    let cwd = worker.cwd.as_deref().filter(|cwd| !cwd.is_empty())?;

    if worker.support_of.as_deref().is_some_and(|base| !base.is_empty()) {
      return None;
    }

    Some(Self {
      name: worker.name.clone(),
      label: worker.label.clone().unwrap_or_else(|| worker.name.clone()),
      root: resolved(Path::new(cwd)),
    })
  }
}

/// A directory as a path standing on its own: a relative one read from where the service runs, and its `.` and
/// `..` parts read away without asking the disk.
fn resolved(path: &Path) -> PathBuf {
  let joined = if path.is_absolute() {
    path.to_path_buf()
  } else {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")).join(path)
  };
  let mut whole = PathBuf::from("/");

  for part in joined.components() {
    match part {
      Component::ParentDir => {
        whole.pop();
      }
      Component::Normal(name) => whole.push(name),
      Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
    }
  }

  whole
}

/// The checkouts out of the workers, in the order bmo-job lists them, each directory once.
pub fn checkouts_of(workers: &[Worker]) -> Vec<Checkout> {
  let mut seen = HashSet::new();

  workers
    .iter()
    .filter_map(Checkout::of)
    .filter(|checkout| seen.insert(checkout.root.clone()))
    .collect()
}
