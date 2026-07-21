use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use std::{fs, io};

use crate::emitter::Emitter;
use crate::event::{Event, Hook, Status};

/// Install the hooks and return the process exit code.
pub fn run<W: Write>(writer: W) -> u8 {
    let mut emitter = Emitter::new(Hook::InstallHooks, writer);
    let started = Instant::now();

    match install(&mut emitter) {
        Ok(installed) => {
            let _outcome = emitter.emit(&Event::Message {
                text: format!("{installed} hook(s) installed."),
            });
            let _outcome = emitter.emit(&Event::HookResult {
                status: Status::Pass,
                exit_code: 0,
                elapsed_seconds: started.elapsed().as_secs(),
                failed_step: None,
                steps: Vec::new(),
            });
            0
        }
        Err(message) => {
            let _outcome = emitter.emit(&Event::Error { message, exit_code: 1 });
            1
        }
    }
}

fn install<W: Write>(emitter: &mut Emitter<W>) -> Result<usize, String> {
    let repo_root = git_path(&["rev-parse", "--show-toplevel"])?;
    let hooks_source = PathBuf::from(&repo_root).join(".githooks");

    if !hooks_source.is_dir() {
        return Err(format!(".githooks/ directory not found at {}", hooks_source.display()));
    }

    let hooks_destination = PathBuf::from(git_path(&["rev-parse", "--git-path", "hooks"])?);

    fs::create_dir_all(&hooks_destination)
        .map_err(|err| format!("cannot create hooks directory '{}': {err}", hooks_destination.display()))?;

    let entries = fs::read_dir(&hooks_source).map_err(|err| format!("cannot read '{}': {err}", hooks_source.display()))?;

    let mut installed = 0;

    for entry in entries {
        let entry = entry.map_err(|err| format!("cannot read a hook entry: {err}"))?;
        let source = entry.path();

        if !source.is_file() {
            continue;
        }

        let name = entry.file_name();
        let destination = hooks_destination.join(&name);

        fs::copy(&source, &destination).map_err(|err| format!("cannot install '{}': {err}", name.to_string_lossy()))?;

        make_executable(&destination).map_err(|err| format!("cannot make '{}' executable: {err}", destination.display()))?;

        let _outcome = emitter.emit(&Event::Message {
            text: format!("Installed: {} -> {}", name.to_string_lossy(), destination.display()),
        });

        installed += 1;
    }

    Ok(installed)
}

fn git_path(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .map_err(|err| format!("cannot run git: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);

    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> io::Result<()> {
    // Windows has no executable bit; git runs hooks through the shell there.
    Ok(())
}
