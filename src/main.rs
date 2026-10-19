//! `bmo-review`: the service in one binary (`docs/api.md`).

use std::process::ExitCode;

use bmo_review::settings::Settings;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-review", version, about = "The code review over the workers' checkouts")]
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
}

#[tokio::main]
async fn main() -> ExitCode {
  let command = Command::parse();
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
  };

  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo-review: {why}");
      ExitCode::FAILURE
    }
  }
}
