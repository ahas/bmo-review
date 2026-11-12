//! The console face the service serves (`docs/api.md`, 「The console face」), through bmo-chat's
//! `bmo-console-face`: the review's declaration of the manifest fields it decides, over the bundle build's folder,
//! guarded by the console's credential.

use std::path::Path;

use bmo_console_face::{
  BundleFolder, CONSOLE_FILES_PREFIX, CONSOLE_MANIFEST_ROUTE, ConsoleFace, ConsoleFaceError,
};
use serde_json::json;

use crate::openapi::Stated;

/// The manifest fields the review decides, as bmo-ui's console plugin contract names them.
pub const DECLARATION: &str = include_str!("console.json");

// What the bundle's build writes into its folder.
const MODULE_FILE: &str = "review.js";
const STYLESHEET_FILE: &str = "review.css";
const WORDS_FILE: &str = "words.json";

/// The face over the bundle built into `folder`, guarded by `credential`, or why it cannot be served.
pub fn face(folder: &Path, credential: &str) -> Result<ConsoleFace, ConsoleFaceError> {
  let bundle = BundleFolder::new(folder, MODULE_FILE, WORDS_FILE).with_stylesheet(STYLESHEET_FILE);

  ConsoleFace::new(DECLARATION, &bundle, credential)
}

/// The face's routes as the OpenAPI document states them, each `GET` beside its path; the face answers them as
/// the contract states, so what they take and answer is the contract's.
pub fn stated() -> Vec<(String, Stated)> {
  let file = json!({ "name": "file", "in": "path", "required": true, "description": "The file's name, as the manifest gives it.", "schema": { "type": "string" } });

  vec![
    (
      CONSOLE_MANIFEST_ROUTE.to_owned(),
      Stated {
        id: "readConsoleManifest",
        tag: "console",
        summary: "The console face's manifest, by bmo-ui's console plugin contract, to the console's credential alone.",
        parameters: Vec::new(),
        body: None,
        answer: (200, json!({ "application/json": { "schema": { "type": "object" } } })),
        codes: Vec::new(),
      },
    ),
    (
      format!("{CONSOLE_FILES_PREFIX}{{file}}"),
      Stated {
        id: "readConsoleFile",
        tag: "console",
        summary: "A file of the console bundle at the path the manifest names: the words to the console's credential \
          alone, the module, the stylesheet and every other file to it with the manifest's role.",
        parameters: vec![file],
        body: None,
        answer: (200, json!({ "*/*": { "schema": { "type": "string" } } })),
        codes: vec!["forbidden", "not-found"],
      },
    ),
  ]
}
