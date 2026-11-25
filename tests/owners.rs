//! Whose projects a reader sees through the console (`docs/api.md`, 「The console face」, 「Whose projects a reader
//! sees」): a member the projects whose repository a stand-in agency records as their own account's, an
//! administrator every one, and a member nothing where no agency answers.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use axum::Router;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use bmo_agency_client::{Repository, RepositoryList, SupportCount};
use bmo_review::http::{Standing, router};
use bmo_review::settings::Settings;
use bmo_review::store::Store;
use serde_json::{Value, json};

use crate::screen::{build, installed_layer, scratch};

const SERVICE: &str = "service-test-token";
const CONSOLE: &str = "console-test-credential";
const AGENCY: &str = "agency-test-token";
/// The member's account, as the console names it.
const MEMBER: i64 = 7;

/// A checkout of one commit at `folder`, its HEAD's hash. Each git run ends before the next starts.
fn checkout(folder: &Path) -> String {
  std::fs::create_dir_all(folder).expect("the checkout's folder is made");
  std::fs::write(folder.join("README.md"), "A line.\n").expect("the file is written");

  let git = |arguments: &[&str]| {
    let ran = Command::new("git")
      .args(["-c", "user.name=agent-a", "-c", "user.email=agent-a", "-c", "commit.gpgsign=false"])
      .args(arguments)
      .current_dir(folder)
      .env("GIT_CONFIG_GLOBAL", "/dev/null")
      .env("GIT_CONFIG_NOSYSTEM", "1")
      .output()
      .expect("git runs");
    assert!(ran.status.success(), "git {arguments:?}: {}", String::from_utf8_lossy(&ran.stderr));

    String::from_utf8_lossy(&ran.stdout).trim().to_owned()
  };

  git(&["init", "--quiet"]);
  git(&["add", "README.md"]);
  git(&["commit", "--quiet", "-m", "First"]);
  git(&["rev-parse", "HEAD"])
}

/// A repository row as the agency answers it, owned by `account` or by the organisation unit `unit`.
fn repository(name: &str, account: Option<i64>, unit: Option<i64>) -> Repository {
  Repository {
    name: name.to_owned(),
    label: None,
    icon: None,
    owner_account: account,
    owner_unit: unit,
    path: format!("/srv/{name}"),
    node_name: None,
    support_count: SupportCount::Count(0),
    prefix_documents: Vec::new(),
    preamble: None,
    rules: None,
    dev_command: None,
    pin: None,
    umbrella: false,
    downgrade_heal: true,
    sampling: None,
    sampling_settings: Vec::new(),
    checkouts: Vec::new(),
  }
}

/// A stand-in agency answering `GET /repositories` to its token: project-a the member's, project-b another
/// account's and project-c an organisation unit's. Its address.
async fn agency() -> String {
  let listed = RepositoryList {
    items: vec![
      repository("project-a", Some(MEMBER), None),
      repository("project-b", Some(MEMBER + 1), None),
      repository("project-c", None, Some(3)),
    ],
    more: false,
  };
  let answer = move |headers: HeaderMap| async move {
    let bearer = headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok());

    if bearer == Some(&format!("Bearer {AGENCY}")) {
      Json(listed).into_response()
    } else {
      StatusCode::UNAUTHORIZED.into_response()
    }
  };
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port is taken");
  let address = format!("http://{}", listener.local_addr().expect("the port is known"));
  let routes: Router = Router::new().route("/repositories", get(answer));

  tokio::spawn(async move { axum::serve(listener, routes).await });

  address
}

/// An address nothing answers at: a port taken and let go.
async fn silent() -> String {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port is taken");

  format!("http://{}", listener.local_addr().expect("the port is known"))
}

/// The three projects' checkouts under `folder`, and the console bundle beside them; the HEAD of each by name.
fn stand_projects(folder: &Path) -> HashMap<&'static str, String> {
  build(&folder.join("dist"), &installed_layer());

  ["project-a", "project-b", "project-c"]
    .into_iter()
    .map(|name| (name, checkout(&folder.join(name))))
    .collect()
}

/// A service over the projects under `folder`, asking the agency at `agency` (none where empty); its address.
async fn serve(folder: &Path, store: &str, agency: &str) -> String {
  std::fs::write(folder.join("agency.token"), format!("{AGENCY}\n")).expect("the token is written");

  let projects: String = ["project-a", "project-b", "project-c"]
    .iter()
    .map(|name| format!("[[projects]]\nname = '{name}'\ncheckout = '{}'\n", folder.join(name).display()))
    .collect();
  let settings = format!(
    "[server]\ntoken = '{SERVICE}'\n\n[agency]\naddress = '{agency}'\ntoken_file = '{token}'\n\n\
     [console]\ntoken = '{CONSOLE}'\nfolder = '{dist}'\n\n{projects}",
    token = folder.join("agency.token").display(),
    dist = folder.join("dist").display(),
  );
  let settings_folder = folder.join(store);

  std::fs::create_dir_all(&settings_folder).expect("the settings folder is made");
  std::fs::write(settings_folder.join("settings.toml"), settings).expect("the settings are written");

  let settings = Settings::read(&settings_folder, Some(HashMap::new())).expect("the settings read");
  let store = Store::open(&settings_folder.join("review.db")).await.expect("the store opens");
  let router =
    router(Standing::new(settings, store).expect("the service stands")).expect("the routes stand");
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port is taken");
  let address = format!("http://{}", listener.local_addr().expect("the port is known"));

  tokio::spawn(async move { axum::serve(listener, router).await });

  address
}

/// A GET of `path` through the console, by `role` with the member's account as the owner key: its status and its
/// body.
async fn read(address: &str, path: &str, role: &str) -> (u16, Value) {
  let answered = reqwest::Client::new()
    .get(format!("{address}/screen{path}"))
    .bearer_auth(CONSOLE)
    .header("Owner-Role", role)
    .header("Owner-Key", MEMBER.to_string())
    .header(header::ACCEPT, "application/json")
    .send()
    .await
    .expect("the service answers");
  let status = answered.status().as_u16();

  (status, answered.json().await.unwrap_or(Value::Null))
}

/// The names of a page's items under `key`.
fn names(page: &Value, key: &str) -> Vec<String> {
  page["items"]
    .as_array()
    .expect("a page")
    .iter()
    .map(|item| item[key].as_str().expect("a name").to_owned())
    .collect()
}

/// A review opened at the service's own path, on `project` at `revision`: its id.
async fn open_review(address: &str, project: &str, revision: &str) -> i64 {
  let answered = reqwest::Client::new()
    .post(format!("{address}/reviews"))
    .bearer_auth(SERVICE)
    .json(
      &json!({ "projectName": project, "revision": revision, "title": "A look", "authorName": "agent-a" }),
    )
    .send()
    .await
    .expect("the service answers");
  assert_eq!(answered.status().as_u16(), 201);

  answered.json::<Value>().await.expect("the review")["id"].as_i64().expect("its id")
}

#[tokio::test]
async fn a_member_reads_only_the_projects_their_account_owns() {
  let folder = scratch("owners-member");
  let heads = stand_projects(&folder);
  let address = serve(&folder, "service", &agency().await).await;
  let theirs = open_review(&address, "project-a", &heads["project-a"]).await;
  let others = open_review(&address, "project-b", &heads["project-b"]).await;

  let (status, projects) = read(&address, "/projects", "member").await;
  assert_eq!(status, 200, "{projects}");
  assert_eq!(names(&projects, "name"), ["project-a"]);
  assert_eq!(read(&address, "/projects/project-a/tree", "member").await.0, 200);

  for path in [
    "/projects/project-b/tree".to_owned(),
    "/projects/project-b/commits".to_owned(),
    "/projects/project-c/tree".to_owned(),
    format!("/reviews/{others}"),
  ] {
    let (status, refused) = read(&address, &path, "member").await;
    assert_eq!((status, refused["code"].as_str()), (404, Some("not-found")), "{path}");
  }

  let (_, reviews) = read(&address, "/reviews", "member").await;
  let ids: Vec<i64> = reviews["items"]
    .as_array()
    .expect("a page")
    .iter()
    .filter_map(|r| r["id"].as_i64())
    .collect();
  assert_eq!(ids, [theirs]);

  let (_, events) = read(&address, "/events", "member").await;
  let projects_heard: Vec<&str> = events["items"]
    .as_array()
    .expect("a page")
    .iter()
    .filter_map(|e| e["data"]["projectName"].as_str())
    .collect();
  assert_eq!(projects_heard, ["project-a"]);

  let (status, every) = read(&address, "/projects", "administrator").await;
  assert_eq!(status, 200, "{every}");
  assert_eq!(names(&every, "name"), ["project-a", "project-b", "project-c"]);
}

#[tokio::test]
async fn a_member_sees_nothing_without_an_agency_that_answers() {
  let folder = scratch("owners-none");
  stand_projects(&folder);
  let unanswered = serve(&folder, "unanswered", &silent().await).await;
  let unnamed = serve(&folder, "unnamed", "").await;

  for path in ["/projects", "/projects/project-a/tree"] {
    let (status, refused) = read(&unanswered, path, "member").await;
    assert_eq!((status, refused["code"].as_str()), (502, Some("upstream-failed")), "{path}");
  }

  let (status, projects) = read(&unnamed, "/projects", "member").await;
  assert_eq!(status, 200, "{projects}");
  assert!(names(&projects, "name").is_empty());

  // The administrator asks no agency.
  let (status, every) = read(&unanswered, "/projects", "administrator").await;
  assert_eq!(status, 200, "{every}");
  assert_eq!(names(&every, "name").len(), 3);
}
