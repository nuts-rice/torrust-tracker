use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::emitter::Emitter;
use crate::event::{Event, Hook, PlannedStep, Status, StepSummary, Verbosity};
use crate::step::{Step, sanitize_name_for_log, strip_ansi};

pub const CONCISE_FAILURE_TAIL_LINES: usize = 10;

#[derive(Debug)]
pub struct RunOutcome {
    pub status: Status,
    pub exit_code: u8,
    pub elapsed_seconds: u64,
    pub failed_step: Option<String>,
    pub steps: Vec<StepSummary>,
}

pub struct Runner<'a, W: Write> {
    emitter: &'a mut Emitter<W>,
    hook: Hook,
    verbosity: Verbosity,
    log_dir: PathBuf,
}

impl<'a, W: Write> Runner<'a, W> {
    pub const fn new(emitter: &'a mut Emitter<W>, hook: Hook, verbosity: Verbosity, log_dir: PathBuf) -> Self {
        Self {
            emitter,
            hook,
            verbosity,
            log_dir,
        }
    }

    /// Run every step in order, stopping at the first failure.
    ///
    /// Emits `hook_start`, a `step_start`/`step_end` pair per attempted step, and a final
    /// `hook_result`.
    ///
    /// # Errors
    ///
    /// Returns an error if an event cannot be emitted, or if a step's log file cannot be
    /// created or its subprocess cannot be spawned.
    pub fn run(&mut self, steps: &[Step]) -> io::Result<RunOutcome> {
        let started = Instant::now();
        let total_steps = steps.len();

        self.emitter.emit(&Event::HookStart {
            total_steps,
            verbosity: self.verbosity,
            log_dir: self.log_dir.to_string_lossy().into_owned(),
            steps: steps
                .iter()
                .enumerate()
                .map(|(offset, step)| PlannedStep {
                    index: offset + 1,
                    name: step.name.to_owned(),
                })
                .collect(),
        })?;
        let mut summaries = Vec::with_capacity(total_steps);
        let mut failed_step = None;

        for (offset, step) in steps.iter().enumerate() {
            let step_index = offset + 1;
            let command = self.command_field(step);

            self.emitter.emit(&Event::StepStart {
                step_index,
                total_steps,
                name: step.name.to_owned(),
                command: command.clone(),
            })?;

            let log_path = self.log_path(step_index, step);
            let step_started = Instant::now();
            let passed = run_command(step.command, &log_path)?;
            let elapsed_seconds = step_started.elapsed().as_secs();

            let status = if passed { Status::Pass } else { Status::Fail };
            let failure_tail = if passed { None } else { Some(self.failure_tail(&log_path)) };
            let log_path_text = log_path.to_string_lossy().into_owned();

            self.emitter.emit(&Event::StepEnd {
                step_index,
                total_steps,
                name: step.name.to_owned(),
                command,
                status,
                elapsed_seconds,
                log_path: log_path_text.clone(),
                failure_tail,
            })?;

            summaries.push(StepSummary {
                index: step_index,
                name: step.name.to_owned(),
                status,
                elapsed_seconds,
                log_path: log_path_text,
            });

            if !passed {
                failed_step = Some(step.name.to_owned());
                break;
            }
        }

        let status = if failed_step.is_some() { Status::Fail } else { Status::Pass };
        let exit_code = u8::from(failed_step.is_some());
        let elapsed_seconds = started.elapsed().as_secs();

        self.emitter.emit(&Event::HookResult {
            status,
            exit_code,
            elapsed_seconds,
            failed_step: failed_step.clone(),
            steps: summaries.clone(),
        })?;

        Ok(RunOutcome {
            status,
            exit_code,
            elapsed_seconds,
            failed_step,
            steps: summaries,
        })
    }

    fn command_field(&self, step: &Step) -> Option<String> {
        if self.verbosity.echoes_commands() {
            Some(step.command.to_owned())
        } else {
            None
        }
    }

    fn log_path(&self, step_index: usize, step: &Step) -> PathBuf {
        let safe_name = sanitize_name_for_log(step.name);
        self.log_dir.join(format!(
            "{}-{step_index:02}-{safe_name}-{}.log",
            self.hook.as_str(),
            std::process::id()
        ))
    }

    fn failure_tail(&self, log_path: &Path) -> Vec<String> {
        let Ok(bytes) = fs::read(log_path) else {
            return vec![format!("(could not read log file: {})", log_path.display())];
        };

        let contents = String::from_utf8_lossy(&bytes);
        let lines: Vec<String> = contents.lines().map(strip_ansi).collect();

        if self.verbosity.echoes_commands() {
            return lines;
        }

        let skip = lines.len().saturating_sub(CONCISE_FAILURE_TAIL_LINES);

        lines.into_iter().skip(skip).collect()
    }
}

/// Execute one step, sending its combined stdout and stderr to `log_path`.
fn run_command(command: &str, log_path: &Path) -> io::Result<bool> {
    let log_file = File::create(log_path)?;
    let log_file_for_stderr = log_file.try_clone()?;

    let status = Command::new("bash")
        .args(["-o", "pipefail", "-c", command])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_for_stderr))
        .status()?;

    Ok(status.success())
}
