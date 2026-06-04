//! `ndoc` — northdoc CLI entrypoint.
//!
//! Thin binary shell: parse args with clap, dispatch into the [`cli`] module,
//! and map any error into a process exit code. All real logic lives in the
//! library modules so it can be unit/snapshot tested independently of `main`.

use std::process::ExitCode;

use clap::Parser;

use northdoc::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // anyhow chains print the full cause stack on the alternate formatter.
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
