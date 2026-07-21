use std::io::Write;

use crate::command::run_suite;
use crate::event::{Hook, Verbosity};
use crate::step::Step;

/// The same four steps the Bash script ran, in the same order.
pub const STEPS: &[Step] = &[
    Step {
        name: "Checking format with nightly toolchain",
        command: "cargo +nightly fmt --check",
    },
    Step {
        name: "Checking workspace with nightly toolchain",
        command: "cargo +nightly check --tests --benches --examples --workspace --all-targets --all-features",
    },
    Step {
        name: "Building documentation with nightly toolchain",
        command: "cargo +nightly doc --no-deps --bins --examples --workspace --all-features",
    },
    Step {
        name: "Running all tests",
        command: "cargo +stable test --tests --benches --examples --workspace --all-targets --all-features",
    },
];

/// Run the pre-push suite and return the process exit code.
pub fn run<W: Write>(writer: W, verbosity: Verbosity) -> u8 {
    run_suite(writer, Hook::PrePush, verbosity, STEPS)
}
