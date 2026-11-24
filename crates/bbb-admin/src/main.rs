use std::{io::Write, process::ExitCode};

use bbb_admin::{run, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut out = std::io::stdout();
    match run(cli, &mut out).await {
        Ok(outcome) => {
            let _ = out.flush();
            // A refusal is a real answer, so it is reported as a distinct exit
            // status rather than as a failure to talk to the server.
            ExitCode::from(outcome.exit_code() as u8)
        }
        Err(error) => {
            eprintln!("bbb-admin: {error}");
            ExitCode::from(2)
        }
    }
}
