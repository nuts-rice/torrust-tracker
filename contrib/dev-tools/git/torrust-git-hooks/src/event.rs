use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hook {
    PreCommit,
    PrePush,
    InstallHooks,
}

impl Hook {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::PrePush => "pre-push",
            Self::InstallHooks => "install-hooks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verbosity {
    Concise,
    Verbose,
}

impl Verbosity {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Concise => "concise",
            Self::Verbose => "verbose",
        }
    }

    #[must_use]
    pub fn echoes_commands(self) -> bool {
        self == Self::Verbose
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedStep {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepSummary {
    pub index: usize,
    pub name: String,
    pub status: Status,
    pub elapsed_seconds: u64,
    pub log_path: String,
}
/// A single NDJSON record.
///
/// `kind` is the serde tag, so each variant serialises flat: the variant's fields sit alongside
/// `schema_version`, `kind`, and `hook` in one object.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    HookStart {
        total_steps: usize,
        verbosity: Verbosity,
        log_dir: String,
        steps: Vec<PlannedStep>,
    },
    StepStart {
        step_index: usize,
        total_steps: usize,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
    StepEnd {
        step_index: usize,
        total_steps: usize,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        status: Status,
        elapsed_seconds: u64,
        log_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        failure_tail: Option<Vec<String>>,
    },
    HookResult {
        status: Status,
        exit_code: u8,
        elapsed_seconds: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        failed_step: Option<String>,
        steps: Vec<StepSummary>,
    },
    Error {
        message: String,
        exit_code: u8,
    },
    Message {
        text: String,
    },
}

/// An [`Event`] paired with the envelope fields shared by every record.
///
/// Serialising this, rather than [`Event`] directly, is what puts `schema_version` and `hook`
/// into the output. [`Event`]'s own `kind` tag is flattened in alongside them.
#[derive(Debug, Serialize)]
pub struct Record<'a> {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook: Option<Hook>,
    #[serde(flatten)]
    pub event: &'a Event,
}

impl<'a> Record<'a> {
    #[must_use]
    pub const fn new(hook: Hook, event: &'a Event) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hook: Some(hook),
            event,
        }
    }

    #[must_use]
    pub const fn global(event: &'a Event) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hook: None,
            event,
        }
    }
}
