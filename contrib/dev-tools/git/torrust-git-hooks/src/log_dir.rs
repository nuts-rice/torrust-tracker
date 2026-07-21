use std::fs;
use std::path::PathBuf;

pub const LOG_DIR_ENV: &str = "TORRUST_GIT_HOOKS_LOG_DIR";

/// Directory used when [`LOG_DIR_ENV`] is unset.
pub const DEFAULT_LOG_DIR: &str = "/tmp";

/// Resolve the log directory, creating it if needed and proving it is writable.
///
/// # Errors
///
/// Returns a human-readable message if the directory cannot be created or written to. Callers
/// treat this as a usage error (exit code 2), matching the former Bash scripts.
pub fn resolve() -> Result<PathBuf, String> {
    let configured = std::env::var(LOG_DIR_ENV).unwrap_or_else(|_| DEFAULT_LOG_DIR.to_owned());
    resolve_from(&configured)
}

/// [`resolve`] against an explicit directory, so tests need not mutate the environment.
///
/// # Errors
///
/// See [`resolve`].
pub fn resolve_from(configured: &str) -> Result<PathBuf, String> {
    let log_dir = PathBuf::from(configured);

    fs::create_dir_all(&log_dir).map_err(|err| format!("cannot create log directory '{configured}': {err} "))?;

    let probe = log_dir.join(format!(".torrust-git-hooks-probe-{}", std::process::id()));

    fs::write(&probe, b"").map_err(|err| format!("log directory '{configured}' is not writable: {err}"))?;

    drop(fs::remove_file(&probe));

    Ok(log_dir)
}
