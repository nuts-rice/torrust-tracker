pub mod cli;
pub mod command;
pub mod emitter;
pub mod event;
pub mod log_dir;
pub mod step;

pub mod runner;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use clap::error::ErrorKind;

use crate::cli::{Cli, Command};
use crate::emitter::Emitter;
use crate::event::Event;

fn main() -> ExitCode {
    let stderr = io::stderr();

    ExitCode::from(run(stderr.lock()))
}

fn run<W: Write>(writer: W) -> u8 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let mut emitter = Emitter::global(writer);
            return report_parse_error(&mut emitter, &err);
        }
    };

    match cli.command {
        Command::PreCommit(args) => command::pre_commit::run(writer, args.verbosity.into()),
        Command::PrePush(args) => command::pre_push::run(writer, args.verbosity.into()),
        Command::InstallHooks => command::install_hooks::run(writer),
    }
}

/// Help and version text has nowhere to go under a JSON-only contract, so it is wrapped in a
/// `message` event on stderr. Everything else is a usage error.
fn report_parse_error<W: Write>(emitter: &mut Emitter<W>, err: &clap::Error) -> u8 {
    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _outcome = emitter.emit(&Event::Message {
                text: err.render().to_string(),
            });
            0
        }
        _ => {
            let _outcome = emitter.emit(&Event::Error {
                message: err.render().to_string(),
                exit_code: 2,
            });
            2
        }
    }
}
