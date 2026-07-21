use std::io::Write;

use crate::command::run_suite;
use crate::event::{Hook, Verbosity};
use crate::step::Step;

/// The same four steps the Bash script ran, in the same order.
pub const STEPS: &[Step] = &[
    Step {
        name: "Checking for unused dependencies  (cargo machete --with-metadata)",
        command: "cargo machete --with-metadata",
    },
    Step {
        name: "Checking workspace layer boundary bans",
        command: "cargo deny check bans",
    },
    Step {
        name: "Running all linters",
        command: "linter all",
    },
    Step {
        name: "Running documentation tests",
        command: "cargo test --doc --workspace",
    },
];

pub fn run<W: Write>(writer: W, verbosity: Verbosity) -> u8 {
    run_suite(writer, Hook::PreCommit, verbosity, STEPS)
}
