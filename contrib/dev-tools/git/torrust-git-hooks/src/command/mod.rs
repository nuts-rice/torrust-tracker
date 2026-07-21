pub mod install_hooks;
pub mod pre_commit;
pub mod pre_push;

use std::io::Write;

use crate::emitter::Emitter;
use crate::event::{Event, Hook, Verbosity};
use crate::log_dir;
use crate::runner::Runner;
use crate::step::Step;

pub fn run_suite<W: Write>(writer: W, hook: Hook, verbosity: Verbosity, steps: &[Step]) -> u8 {
    let mut emitter = Emitter::new(hook, writer);
    let log_dir = match log_dir::resolve() {
        Ok(log_dir) => log_dir,
        Err(message) => {
            let _outcome = emitter.emit(&Event::Error { message, exit_code: 2 });
            return 2;
        }
    };
    let mut runner = Runner::new(&mut emitter, hook, verbosity, log_dir);
    match runner.run(steps) {
        Ok(outcome) => outcome.exit_code,
        Err(err) => {
            let _outcome = emitter.emit(&Event::Error {
                message: format!("{hook} runner failed: {err}", hook = hook.as_str()),
                exit_code: 1,
            });
            1
        }
    }
}
