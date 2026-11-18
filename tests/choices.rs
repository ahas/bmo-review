//! Every value the screen lets be chosen from a list is chosen in bmo-ui's one select (`docs/screen.md`, what it is
//! built with), so the layer alone decides when a list offers its search. The rule itself is bmo-ui's tests'; this
//! holds that no part of the screen stands the component library's select bare beside it.

use std::path::{Path, PathBuf};

/// The screen's sources.
const SCREEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/client/app");

/// The component library's select, select menu and input menu as a template opens them, in either spelling; the
/// select's opening also opens the select menu's.
const BARE_SELECTS: [&str; 4] = ["<USelect", "<UInputMenu", "<u-select", "<u-input-menu"];

fn templates(folder: &Path, found: &mut Vec<PathBuf>) {
  for entry in std::fs::read_dir(folder).expect("the screen's folder is read") {
    let path = entry.expect("an entry of the screen's folder is read").path();

    if path.is_dir() {
      templates(&path, found);
    } else if path.extension().is_some_and(|ending| ending == "vue") {
      found.push(path);
    }
  }
}

#[test]
fn every_list_choice_stands_in_the_one_select() {
  let mut found = Vec::new();
  templates(Path::new(SCREEN), &mut found);
  assert!(!found.is_empty(), "the screen holds its templates");

  let bare: Vec<String> = found
    .iter()
    .flat_map(|path| {
      let text = std::fs::read_to_string(path).expect("a template is read");

      BARE_SELECTS
        .iter()
        .filter(|tag| text.contains(*tag))
        .map(|tag| format!("{} stands {tag}", path.strip_prefix(SCREEN).unwrap_or(path).display()))
        .collect::<Vec<_>>()
    })
    .collect();

  assert!(bare.is_empty(), "a list choice stands outside bmo-ui's ChoiceSelect: {bare:?}");
}
