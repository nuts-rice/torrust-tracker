use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::event::Verbosity;

#[derive(Debug, Parser)]
#[command(
    name = "torrust-git-hooks",
    version,
    about = "Runs the Torrust Tracker git hook check suites.",
    long_about = "Runs the Torrust Tracker git hook check suites and installs the repository \
                  git hooks. All output is NDJSON on stderr; stdout is always empty. Pass/fail \
                  is reported through the exit code: 0 pass, 1 failure, 2 usage error."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    PreCommit(HookArgs),
    PrePush(HookArgs),
    InstallHooks,
}

#[derive(Debug, Args)]
pub struct HookArgs {
    #[arg(long, value_enum, default_value_t = VerbosityArg::Concise)]
    pub verbosity: VerbosityArg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VerbosityArg {
    Concise,
    Verbose,
}

impl From<VerbosityArg> for Verbosity {
    fn from(val: VerbosityArg) -> Self {
        match val {
            VerbosityArg::Concise => Self::Concise,
            VerbosityArg::Verbose => Self::Verbose,
        }
    }
}
