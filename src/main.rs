//! `bmo-review`: the service, its health, the harness plugin's MCP server and its OpenAPI document, in one binary
//! (`docs/api.md`).

use std::io::Write;
use std::process::ExitCode;

use bmo_review::client::ReviewClient;
use bmo_review::settings::Settings;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-review", version, about = "Code review over git checkouts")]
struct Command {
  #[command(subcommand)]
  word: Word,
}

#[derive(Subcommand)]
enum Word {
  /// Opens the store and answers the routes.
  Serve,
  /// Whether the service the settings name answers ready; ends 0 where it does.
  Health,
  /// Runs the harness plugin's MCP server on standard input and output, calling the service at its address.
  Mcp {
    /// The service's base address; `http://` and the settings' listen address where absent.
    #[arg(long)]
    address: Option<String>,
    /// The service's bearer token; the settings' token where absent.
    #[arg(long)]
    token: Option<String>,
  },
  /// Prints the OpenAPI document, as `docs/openapi.json` keeps it.
  Openapi,
}

#[tokio::main]
async fn main() -> ExitCode {
  let command = Command::parse();

  if let Word::Openapi = command.word {
    let written = std::io::stdout().lock().write_all(bmo_review::openapi::openapi_json().as_bytes());

    return if written.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
  }

  let settings = match Settings::load() {
    Ok(settings) => settings,
    Err(why) => {
      eprintln!("bmo-review: {why}");
      return ExitCode::FAILURE;
    }
  };
  let done = match command.word {
    Word::Serve => bmo_review::serve(settings).await,
    Word::Health => bmo_review::health::ask(&settings).await,
    Word::Mcp { address, token } => {
      let address = address.unwrap_or_else(|| format!("http://{}", settings.server.listen));
      let token = token.unwrap_or_else(|| settings.server.token.clone());

      match ReviewClient::new(&address, &token) {
        Ok(client) => bmo_review::mcp::serve(client).await,
        Err(why) => Err(why.to_string()),
      }
    }
    Word::Openapi => Ok(()),
  };

  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo-review: {why}");
      ExitCode::FAILURE
    }
  }
}
