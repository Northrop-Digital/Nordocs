//! `ndoc` — nordocs CLI entrypoint.
//!
//! Thin binary shell: parse args with clap, dispatch into the [`cli`] module,
//! and map any error into a process exit code. All real logic lives in the
//! library modules so it can be unit/snapshot tested independently of `main`.

use std::process::ExitCode;

use clap::Parser;

use nordocs::cli::output::emit_json_error;
use nordocs::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if json {
                emit_json_error(&format!("{err:#}"));
            } else {
                eprintln!("error: {err:#}");
            }
            ExitCode::FAILURE
        }
    }
}
