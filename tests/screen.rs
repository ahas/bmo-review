//! The console bundle as the service answers it (`docs/api.md`, 「The console bundle」): the manifest read through
//! bmo-ui's own reading of the contract, the guard's refusals under the screen's prefix, and the module's path
//! changing with the bundle's bytes.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use bmo_review::http::{Standing, router};
use bmo_review::settings::Settings;
use bmo_review::store::Store;
use serde_json::Value;

const TOKEN: &str = "screen-test-token";

fn scratch(name: &str) -> PathBuf {
  let folder = std::env::temp_dir().join(format!("bmo-review-screen-{}-{name}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).expect("the scratch folder is made");

  folder
}

// A bundle as its build writes one, its module saying `module`.
fn bundle(folder: &Path, module: &str) {
  std::fs::create_dir_all(folder.join("chunks")).expect("the bundle's folders are made");
  std::fs::write(folder.join("review.js"), module).expect("the module is written");
  std::fs::write(folder.join("review.css"), ".code-review-glyph-add { cursor: pointer; }")
    .expect("the sheet is written");
  std::fs::write(folder.join("chunks/code-a1.js"), "export const page = 1;").expect("the chunk is written");
  std::fs::write(folder.join("words.json"), r#"{"en":{"code":{"title":{"screen":"Code"}}}}"#)
    .expect("the words are written");
}

// A service standing on `bundle` as its bundle folder, answering at the address given back.
async fn serve(scratch: &Path, bundle: &Path) -> String {
  let environment = HashMap::from([
    ("BMO_REVIEW_SERVER__TOKEN".to_owned(), TOKEN.to_owned()),
    ("BMO_REVIEW_SCREEN__BUNDLE".to_owned(), bundle.display().to_string()),
  ]);
  let settings = Settings::read(scratch, Some(environment)).expect("the settings read");
  let store = Store::open(&scratch.join("review.db")).await.expect("the store opens");
  let router =
    router(Standing::new(settings, store).expect("the service stands")).expect("the routes stand");
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port is taken");
  let address = format!("http://{}", listener.local_addr().expect("the port is known"));

  tokio::spawn(async move { axum::serve(listener, router).await });

  address
}

// The status a GET of `path` is answered with, sending the token where `token` and `Owner-Role: role` where given.
async fn status(address: &str, path: &str, token: bool, role: Option<&str>) -> u16 {
  let mut request = reqwest::Client::new().get(format!("{address}{path}"));

  if token {
    request = request.bearer_auth(TOKEN);
  }

  if let Some(role) = role {
    request = request.header("Owner-Role", role);
  }

  request.send().await.expect("the service answers").status().as_u16()
}

async fn manifest(address: &str) -> Value {
  reqwest::Client::new()
    .get(format!("{address}/console-manifest"))
    .bearer_auth(TOKEN)
    .send()
    .await
    .expect("the service answers")
    .json()
    .await
    .expect("the manifest is JSON")
}

// The manifest as bmo-ui's `consoleManifest` reads it, run by Bun in the bundle's own project, or the refusal.
fn read_through_the_contract(manifest: &Value) -> Result<Value, String> {
  let client = Path::new(env!("CARGO_MANIFEST_DIR")).join("client");
  let mut reading = Command::new("bun")
    .arg("bundle/read-manifest.ts")
    .current_dir(&client)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Bun runs in client/");

  reading
    .stdin
    .take()
    .expect("the reading takes its input")
    .write_all(manifest.to_string().as_bytes())
    .expect("the manifest is handed over");

  let done = reading.wait_with_output().expect("the reading ends");

  if done.status.success() {
    Ok(serde_json::from_slice(&done.stdout).expect("the reading answers JSON"))
  } else {
    Err(String::from_utf8_lossy(&done.stderr).into_owned())
  }
}

#[tokio::test]
async fn the_manifest_keeps_the_contract() {
  let folder = scratch("contract");
  bundle(&folder.join("dist"), "export default { routes: [] };");
  let address = serve(&folder, &folder.join("dist")).await;
  let answered = manifest(&address).await;
  let read = read_through_the_contract(&answered).expect("the manifest keeps the contract");

  assert_eq!(read["name"], "code");
  assert_eq!(read["screenPath"], "/code");
  assert_eq!(read["serviceName"], "review");
  assert_eq!(read["role"], "member");
  assert_eq!(read["ownerKey"], false);
  assert_eq!(read["apiPrefix"], "/screen");
  assert_eq!(read["events"], true);
  assert_eq!(read["websocket"], false);
  assert_eq!(read["pushKinds"], Value::Array(Vec::new()));
  assert_eq!(read["words"], "/words");

  let mut broken = answered.clone();
  broken["role"] = "guest".into();
  let refusal =
    read_through_the_contract(&broken).expect_err("a role the contract does not know is refused");
  assert!(refusal.contains("role"), "{refusal}");
}

#[tokio::test]
async fn the_screen_is_guarded_by_the_token_and_the_role() {
  let folder = scratch("guard");
  bundle(&folder.join("dist"), "export default { routes: [] };");
  let address = serve(&folder, &folder.join("dist")).await;
  let module = manifest(&address).await["bundle"].as_str().expect("the module's path").to_owned();
  let stylesheet = manifest(&address).await["stylesheet"].as_str().expect("the sheet's path").to_owned();

  // The manifest and the words: the token alone.
  assert_eq!(status(&address, "/console-manifest", true, None).await, 200);
  assert_eq!(status(&address, "/console-manifest", false, None).await, 401);
  assert_eq!(status(&address, "/words", true, None).await, 200);
  assert_eq!(status(&address, "/words", false, None).await, 401);

  // The module, the stylesheet and every route under the prefix: the token, then a role that opens the screen.
  for path in [module.as_str(), stylesheet.as_str(), "/screen/health"] {
    assert_eq!(status(&address, path, false, Some("member")).await, 401, "{path}");
    assert_eq!(status(&address, path, true, None).await, 403, "{path}");
    assert_eq!(status(&address, path, true, Some("guest")).await, 403, "{path}");
    assert_eq!(status(&address, path, true, Some("member")).await, 200, "{path}");
    assert_eq!(status(&address, path, true, Some("administrator")).await, 200, "{path}");
  }

  // A route at its own path asks no role.
  assert_eq!(status(&address, "/health", true, None).await, 200);

  let answered = reqwest::Client::new()
    .get(format!("{address}{module}"))
    .bearer_auth(TOKEN)
    .header("Owner-Role", "member")
    .send()
    .await
    .expect("the service answers");
  let media = answered.headers()["content-type"].to_str().expect("a media type").to_owned();

  assert!(media.starts_with("text/javascript"), "{media}");
  assert_eq!(answered.text().await.expect("the module's text"), "export default { routes: [] };");
}

#[tokio::test]
async fn the_module_path_follows_the_bundle_bytes() {
  let folder = scratch("version");
  bundle(&folder.join("first"), "export default { routes: [] };");
  bundle(&folder.join("again"), "export default { routes: [] };");
  bundle(&folder.join("changed"), "export default { routes: [{ path: '/code' }] };");
  let first = serve(&folder, &folder.join("first")).await;
  let again = serve(&folder, &folder.join("again")).await;
  let changed = serve(&folder, &folder.join("changed")).await;
  let first_module = manifest(&first).await["bundle"].as_str().expect("the module's path").to_owned();
  let again_module = manifest(&again).await["bundle"].as_str().expect("the module's path").to_owned();
  let changed_module = manifest(&changed).await["bundle"].as_str().expect("the module's path").to_owned();
  let changed_sheet = manifest(&changed).await["stylesheet"].as_str().expect("the sheet's path").to_owned();

  assert_eq!(first_module, again_module, "the same bytes stand at the same path");
  assert_ne!(first_module, changed_module, "other bytes stand at another path");
  assert!(
    !changed_sheet.contains(first_module.rsplit_once('/').expect("a version").0),
    "every path moves"
  );

  // Only the version standing answers, and a chunk is answered beside the module under it.
  assert_eq!(status(&changed, &first_module, true, Some("member")).await, 404);
  assert_eq!(status(&changed, &changed_module, true, Some("member")).await, 200);
  let chunk = changed_module.replace("review.js", "chunks/code-a1.js");
  assert_eq!(status(&changed, &chunk, true, Some("member")).await, 200);
}

#[tokio::test]
async fn no_folder_answers_no_screen() {
  let folder = scratch("none");
  let address = serve(&folder, &folder.join("absent")).await;

  assert_eq!(status(&address, "/console-manifest", true, None).await, 404);
  assert_eq!(status(&address, "/words", true, None).await, 404);
  assert_eq!(status(&address, "/health", true, None).await, 200);
}
