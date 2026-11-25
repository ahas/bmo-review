//! The review service: projects' checkouts read at a revision, the reviews of a revision with their comments, and
//! the items filed from them into the job service; with the service's own client, its OpenAPI document and the
//! harness plugin's MCP server (`docs/code-review.md`, `docs/api.md`).

pub mod client;
pub mod console;
pub mod events;
pub mod health;
pub mod http;
pub mod import;
pub mod mcp;
pub mod openapi;
pub mod owners;
pub mod review;
pub mod settings;
pub mod store;
pub mod wire;

use std::io;

use tokio::net::TcpListener;

use crate::http::Standing;
use crate::settings::Settings;
use crate::store::Store;

/// The version this build answers with: the set's, which the testbed hands the build as `BMO_VERSION`, else
/// the manifest's placeholder (bmo-chat's `docs/conventions.md`, 「Versions」).
pub const VERSION: &str = match option_env!("BMO_VERSION") {
  Some(version) => version,
  None => env!("CARGO_PKG_VERSION"),
};

/// Opens the store, then answers the routes on the settings' address until the process is told to stop. A stop
/// ends the event streams and waits for the requests in hand to finish.
pub async fn serve(settings: Settings) -> Result<(), String> {
  let store = Store::open(&settings.store.path).await.map_err(|why| why.to_string())?;
  let listen = settings.server.listen;
  let standing = Standing::new(settings, store)?;
  let events = standing.events.clone();
  let router = http::router(standing).map_err(|faults| faults.join("; "))?;
  let listener = TcpListener::bind(listen)
    .await
    .map_err(|why| format!("{listen} could not be listened on: {why}"))?;

  axum::serve(listener, router)
    .with_graceful_shutdown(async move {
      stopped().await;
      events.end_streams();
    })
    .await
    .map_err(|why: io::Error| why.to_string())
}

/// Resolves once the process is told to stop: an interrupt, or the service manager's terminate.
async fn stopped() {
  let interrupt = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending().await,
    }
  };

  tokio::select! {
    () = interrupt => {}
    () = terminate => {}
  }
}
