//! The console bundle the service answers (`docs/api.md`, 「The console bundle」; bmo-ui's
//! `docs/console-bundles.md`): the folder its build writes, read when the service starts; its manifest, its words
//! and its files as routes; and the guard every route stands behind again under the screen's prefix.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::http::{Call, Refused, Route, answer};
use crate::openapi::{Stated, json_of};
use crate::wire::{Role, ScreenManifest, Words};

/// The path every route is answered again under, guarded by the reader's role: the manifest's API prefix.
pub const PREFIX: &str = "/screen";

/// The least role that opens the screen.
pub const ROLE: Role = Role::Member;

/// Whether every call the screen makes needs the reader's owner key.
pub const OWNER_KEY: bool = false;

/// The header a console names the reader's role in.
pub const OWNER_ROLE_HEADER: &str = "Owner-Role";

/// The header a console names the reader's owner key in.
pub const OWNER_KEY_HEADER: &str = "Owner-Key";

// The files the bundle's build writes at the folder's top.
const MODULE: &str = "review.js";
const STYLESHEET: &str = "review.css";
const WORDS: &str = "words.json";

// How long a reader may keep a file of the bundle: its path changes whenever its bytes do.
const KEPT: &str = "private, max-age=31536000, immutable";

impl Role {
  /// The role `Owner-Role` names, none for a name that is no role.
  pub fn named(name: &str) -> Option<Self> {
    match name {
      "administrator" => Some(Self::Administrator),
      "member" => Some(Self::Member),
      _ => None,
    }
  }

  /// Whether a reader holding this role may open what needs `needed`.
  pub fn allows(self, needed: Role) -> bool {
    self == Self::Administrator || self == needed
  }
}

/// The console bundle as its build wrote it: every file by its path in the folder, and the version they stand
/// under, a digest of every path and its bytes.
#[derive(Debug)]
pub struct Bundle {
  pub version: String,
  files: BTreeMap<String, Vec<u8>>,
  words: Words,
}

impl Bundle {
  /// The bundle in `folder`, none where the folder does not stand, or why the folder is no bundle.
  pub fn read(folder: &Path) -> Result<Option<Self>, String> {
    if !folder.is_dir() {
      return Ok(None);
    }

    let mut files = BTreeMap::new();

    gather(folder, folder, &mut files)?;

    for needed in [MODULE, STYLESHEET, WORDS] {
      if !files.contains_key(needed) {
        return Err(format!("the console bundle in {} holds no {needed}", folder.display()));
      }
    }

    let words: Words = serde_json::from_slice(&files[WORDS])
      .map_err(|why| format!("the console bundle's {WORDS} is not each locale's words: {why}"))?;
    let mut digest = Sha256::new();

    for (path, bytes) in &files {
      digest.update((path.len() as u64).to_le_bytes());
      digest.update(path.as_bytes());
      digest.update((bytes.len() as u64).to_le_bytes());
      digest.update(bytes);
    }

    let version = digest.finalize()[..8].iter().map(|byte| format!("{byte:02x}")).collect();

    Ok(Some(Self {
      version,
      files,
      words,
    }))
  }

  /// The path one of the bundle's files is answered at, under the prefix and the version.
  fn path_of(&self, name: &str) -> String {
    format!("{PREFIX}/bundle/{}/{name}", self.version)
  }

  /// The manifest the service answers for this bundle.
  pub fn manifest(&self) -> ScreenManifest {
    ScreenManifest {
      name: "code".into(),
      title: "code.title.screen".into(),
      icon: "i-material-symbols-rate-review-outline".into(),
      screen_path: "/code".into(),
      service_name: "review".into(),
      bundle: self.path_of(MODULE),
      stylesheet: self.path_of(STYLESHEET),
      words: WordsOf::PATH.into(),
      api_prefix: PREFIX.into(),
      role: ROLE,
      owner_key: OWNER_KEY,
      events: true,
      websocket: false,
      push_kinds: Vec::new(),
    }
  }
}

// Every file under `at`, by its path from `top` with its parts joined by `/`.
fn gather(top: &Path, at: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
  let unread = |path: &Path, why: std::io::Error| format!("{} could not be read: {why}", path.display());
  let entries = std::fs::read_dir(at).map_err(|why| unread(at, why))?;

  for entry in entries {
    let path: PathBuf = entry.map_err(|why| unread(at, why))?.path();

    if path.is_dir() {
      gather(top, &path, files)?;
      continue;
    }

    let name = path
      .strip_prefix(top)
      .map_err(|why| format!("{} lies outside the bundle: {why}", path.display()))?
      .components()
      .map(|part| part.as_os_str().to_string_lossy())
      .collect::<Vec<_>>()
      .join("/");
    let bytes = std::fs::read(&path).map_err(|why| unread(&path, why))?;

    files.insert(name, bytes);
  }

  Ok(())
}

// The media type a file of the bundle is answered under, by its ending.
fn media_type(name: &str) -> &'static str {
  match name.rsplit_once('.').map(|(_, ending)| ending) {
    Some("js") => "text/javascript; charset=utf-8",
    Some("css") => "text/css; charset=utf-8",
    Some("json") => "application/json",
    Some("svg") => "image/svg+xml",
    Some("ttf") => "font/ttf",
    Some("woff2") => "font/woff2",
    _ => "application/octet-stream",
  }
}

// The bundle standing, or the refusal a call to it is answered where none stands.
fn standing_bundle(call: &Call) -> Result<&Bundle, Refused> {
  call
    .standing()
    .bundle
    .as_deref()
    .ok_or_else(|| Refused::not_found("no console bundle stands"))
}

/// Under the prefix, a call is let in only where its `Owner-Role` allows the screen's role, and where the screen
/// needs the owner key, only where its `Owner-Key` stands.
pub async fn require_role(request: Request, next: Next) -> Response {
  let headers = request.headers();
  let held = headers
    .get(OWNER_ROLE_HEADER)
    .and_then(|value| value.to_str().ok())
    .and_then(Role::named);
  let refusal = match held {
    None => Some("the call names no role the screen knows"),
    Some(role) if !role.allows(ROLE) => Some("the role the call names does not open the screen"),
    Some(_) if OWNER_KEY && !headers.contains_key(OWNER_KEY_HEADER) => Some("the call names no owner key"),
    Some(_) => None,
  };

  match refusal {
    Some(why) => Refused::new(StatusCode::FORBIDDEN, "forbidden", why).into_response(),
    None => next.run(request).await,
  }
}

/// `GET /console-manifest`: the console bundle's manifest.
pub struct Manifest;

impl Route for Manifest {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/console-manifest";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    Ok(answer(StatusCode::OK, &standing_bundle(&call)?.manifest()))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readConsoleManifest",
      tag: "screen",
      summary: "The console bundle's manifest.",
      parameters: Vec::new(),
      body: None,
      answer: (200, json_of::<ScreenManifest>(generator)),
      codes: vec!["not-found"],
    }
  }
}

crate::route!(Manifest);

/// `GET /words`: the console bundle's words, each locale's tree under the locale's code.
pub struct WordsOf;

impl Route for WordsOf {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/words";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    Ok(answer(StatusCode::OK, &standing_bundle(&call)?.words))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readWords",
      tag: "screen",
      summary: "The console bundle's words, each locale's under its code.",
      parameters: Vec::new(),
      body: None,
      answer: (200, json_of::<Words>(generator)),
      codes: vec!["not-found"],
    }
  }
}

crate::route!(WordsOf);

/// `GET /bundle/{version}/{fileName}`: one file of the console bundle, answered only under the version standing.
pub struct BundleFile;

impl Route for BundleFile {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/bundle/{version}/{*fileName}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let mut params = call.path_params().await?;
    let version = params.remove("version").unwrap_or_default();
    let name = params.remove("fileName").unwrap_or_default();
    let bundle = standing_bundle(&call)?;
    let bytes = bundle.files.get(&name).filter(|_| version == bundle.version).ok_or_else(|| {
      Refused::not_found(format!("no file {name} of the bundle stands under version {version}"))
    })?;
    let mut response = bytes.clone().into_response();
    let headers = response.headers_mut();

    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type(&name)));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(KEPT));

    Ok(response)
  }

  fn stated(&self, _generator: &mut SchemaGenerator) -> Stated {
    let key = |name: &str, description: &str| json!({ "name": name, "in": "path", "required": true, "description": description, "schema": { "type": "string" } });
    let bytes = json!({ "type": "string" });
    let content: Map<String, Value> = ["text/javascript", "text/css", "application/octet-stream"]
      .into_iter()
      .map(|media| (media.to_owned(), json!({ "schema": bytes })))
      .collect();

    Stated {
      id: "readBundleFile",
      tag: "screen",
      summary: "One file of the console bundle, under its own media type.",
      parameters: vec![
        key("version", "The bundle's version, as the manifest names it."),
        key("fileName", "The file's path in the bundle, its slashes as they are."),
      ],
      body: None,
      answer: (200, Value::Object(content)),
      codes: vec!["not-found"],
    }
  }
}

crate::route!(BundleFile);
