//! The console face as the service serves it (`docs/api.md`, 「The console face」): the manifest read through
//! bmo-ui's own reading of the contract on the release the client vendors, which credential each of the service's
//! routes takes, no face where no bundle is built, and the reach the testbed's console probe asks by. The face's
//! own rules are bmo-console-face's tests'.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bmo_console_face::read_manifest;
use bmo_review::http::{Standing, router};
use bmo_review::settings::Settings;
use bmo_review::store::Store;
use serde_json::{Value, json};

const SERVICE: &str = "service-test-token";
const CONSOLE: &str = "console-test-credential";

/// The client's folder.
const CLIENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/client");

fn scratch(name: &str) -> PathBuf {
  let folder = std::env::temp_dir().join(format!("bmo-review-screen-{}-{name}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).expect("the scratch folder is made");

  folder
}

/// A bundle as bmo-ui's bundle build writes one, built on `layer`.
fn build(folder: &Path, layer: &str) {
  std::fs::create_dir_all(folder.join("chunks")).expect("the bundle's folders are made");
  std::fs::write(folder.join("review.js"), "export default { routes: [] };")
    .expect("the module is written");
  std::fs::write(folder.join("review.css"), ".code-review-lines {}").expect("the sheet is written");
  std::fs::write(folder.join("chunks/code-a1.js"), "export const page = 1;").expect("the chunk is written");
  std::fs::write(
    folder.join("words.json"),
    json!({ "en": { "code": { "title": { "screen": "Code" } } } }).to_string(),
  )
  .expect("the words are written");
  std::fs::write(folder.join("layer.json"), json!({ "layer": layer }).to_string())
    .expect("the layer is written");
}

/// The release of bmo-ui the client has installed, as its own layer file names it.
fn installed_layer() -> String {
  let path = Path::new(CLIENT).join("node_modules/bmo-ui/shared/layer.json");
  let file: Value =
    serde_json::from_slice(&std::fs::read(path).expect("the client's packages are installed"))
      .expect("JSON");

  file["layer"].as_str().expect("the layer's version").to_owned()
}

/// The bmo-ui release file the client vendors, as its package manifest names it.
fn vendored_release() -> PathBuf {
  let manifest: Value = serde_json::from_slice(
    &std::fs::read(Path::new(CLIENT).join("package.json")).expect("the manifest reads"),
  )
  .expect("JSON");
  let taken = manifest["devDependencies"]["bmo-ui"].as_str().expect("bmo-ui is a dependency");

  Path::new(CLIENT).join(taken.strip_prefix("file:").expect("bmo-ui is taken as a file"))
}

// A service standing on `folder` as its bundle's folder, answering at the address given back.
async fn serve(scratch: &Path, folder: &Path) -> String {
  let environment = HashMap::from([
    ("BMO_REVIEW_SERVER__TOKEN".to_owned(), SERVICE.to_owned()),
    ("BMO_REVIEW_CONSOLE__TOKEN".to_owned(), CONSOLE.to_owned()),
    ("BMO_REVIEW_CONSOLE__FOLDER".to_owned(), folder.display().to_string()),
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

// A GET of `path` under `bearer`, with `Owner-Role: role` where given: its status and its body.
async fn get(address: &str, path: &str, bearer: &str, role: Option<&str>) -> (u16, String) {
  let mut request = reqwest::Client::new().get(format!("{address}{path}")).bearer_auth(bearer);

  if let Some(role) = role {
    request = request.header("Owner-Role", role);
  }

  let answered = request.send().await.expect("the service answers");
  let status = answered.status().as_u16();

  (status, answered.text().await.expect("the body reads"))
}

#[tokio::test]
async fn the_served_manifest_is_read_by_the_contract_on_the_vendored_release() {
  let folder = scratch("contract");
  let layer = installed_layer();
  build(&folder.join("dist"), &layer);
  let address = serve(&folder, &folder.join("dist")).await;

  let (status, served) = get(&address, "/console-manifest", CONSOLE, None).await;
  assert_eq!(status, 200, "{served}");
  let served: Value = serde_json::from_str(&served).expect("the manifest is JSON");
  let read = read_manifest(&served, &vendored_release()).unwrap_or_else(|why| panic!("{why}"));

  assert_eq!(read["layer"], layer.as_str());
  assert_eq!(read["name"], "code");
  assert_eq!(read["screenPath"], "/code");
  assert_eq!(read["serviceName"], "review");
  assert_eq!(read["apiPrefix"], "/screen");
  assert_eq!(read["role"], "member");
  assert_eq!(read["sidebarSection"], "administration");
  assert_eq!(read["ownerKey"], false);
  assert_eq!(read["events"], true);
}

#[tokio::test]
async fn each_route_takes_its_own_credential() {
  let folder = scratch("credentials");
  build(&folder.join("dist"), &installed_layer());
  let address = serve(&folder, &folder.join("dist")).await;

  // A route at its own path: the service token alone.
  assert_eq!(get(&address, "/health", SERVICE, None).await.0, 200);
  assert_eq!(get(&address, "/health", CONSOLE, Some("member")).await.0, 401);

  // The same route under the console's prefix: the console credential alone, and a role that opens the screen.
  assert_eq!(get(&address, "/screen/health", CONSOLE, Some("member")).await.0, 200);
  assert_eq!(get(&address, "/screen/health", CONSOLE, Some("administrator")).await.0, 200);
  assert_eq!(get(&address, "/screen/health", CONSOLE, None).await.0, 403);
  assert_eq!(get(&address, "/screen/health", SERVICE, Some("member")).await.0, 401);

  // The face: the console credential alone.
  assert_eq!(get(&address, "/console-manifest", SERVICE, None).await.0, 401);
}

#[tokio::test]
async fn no_face_stands_where_no_bundle_is_built() {
  let folder = scratch("none");
  let address = serve(&folder, &folder.join("absent")).await;

  assert_eq!(get(&address, "/console-manifest", CONSOLE, None).await.0, 404);
  assert_eq!(get(&address, "/screen/health", CONSOLE, Some("member")).await.0, 404);
  assert_eq!(get(&address, "/health", SERVICE, None).await.0, 200);
}

#[test]
fn the_reach_names_the_address_and_both_credentials() {
  let folder = scratch("reach");
  let environment = HashMap::from([
    ("BMO_REVIEW_SERVER__LISTEN".to_owned(), "127.0.0.1:4711".to_owned()),
    ("BMO_REVIEW_SERVER__TOKEN".to_owned(), SERVICE.to_owned()),
    ("BMO_REVIEW_CONSOLE__TOKEN".to_owned(), CONSOLE.to_owned()),
  ]);
  let settings = Settings::read(&folder, Some(environment)).expect("the settings read");

  assert_eq!(
    bmo_review::console::reach(&settings),
    json!({ "address": "http://127.0.0.1:4711", "consoleToken": CONSOLE, "token": SERVICE })
  );
}
