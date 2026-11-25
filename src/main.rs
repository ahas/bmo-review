//! `bmo-review`: the service, its health, the harness plugin's MCP server and its OpenAPI document, in one binary
//! (`docs/api.md`).

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use bmo_review::client::ReviewClient;
use bmo_review::settings::Settings;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-review", version = bmo_review::VERSION, about = "Code review over git checkouts")]
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
  /// Prints how a console reaches the service, as the testbed's console probe asks it: the base address, the
  /// console credential and the service token, as one JSON object.
  ConsoleReach,
  /// Moves the old archive's code reviews from the item's export into this service's store (TODO #1923).
  Import {
    /// The export directory the item's export hand wrote.
    #[arg(long)]
    export: PathBuf,
    /// The store's SQLite file; made and migrated where it does not stand.
    #[arg(long)]
    database: PathBuf,
    /// Reads the export and reports what it would write, writing nothing.
    #[arg(long)]
    dry_run: bool,
  },
}

#[tokio::main]
async fn main() -> ExitCode {
  let command = Command::parse();

  if let Word::Openapi = command.word {
    let written = std::io::stdout().lock().write_all(bmo_review::openapi::openapi_json().as_bytes());

    return if written.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
  }

  // The import reads its own arguments and never the deployment's settings.
  if let Word::Import {
    export,
    database,
    dry_run,
  } = &command.word
  {
    return match bmo_review::import::run(database, export, *dry_run).await {
      Ok(report) => {
        report.lines().iter().for_each(|line| println!("{line}"));
        if report.sample.failures.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
      }
      Err(why) => {
        eprintln!("bmo-review: {why}");
        ExitCode::FAILURE
      }
    };
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
    Word::ConsoleReach => {
      println!("{}", bmo_review::console::reach(&settings));
      Ok(())
    }
    Word::Import { .. } => unreachable!("the import is answered before the settings are read"),
  };

  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo-review: {why}");
      ExitCode::FAILURE
    }
  }
}
