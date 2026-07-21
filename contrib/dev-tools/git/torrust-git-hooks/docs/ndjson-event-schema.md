# `torrust-git-hooks` NDJSON Event Schema

Version: `1`

This document defines the output contract for the `torrust-git-hooks` binary. It satisfies
task T3 of [issue #1843](../../../../../docs/issues/open/1843-migrate-git-hooks-scripts-from-bash-to-rust.md)
and conforms to the [global CLI output contract ADR](../../../../../docs/adrs/20260519000000_define_global_cli_output_contract.md).

## Channel contract

The binary is classified `no-stdout-result` (ADR §3).

- **stdout**: always empty, in every mode, at every verbosity, on success and on failure.
- **stderr**: NDJSON — one complete JSON object per line, flushed as it is written.
- Pass/fail is communicated **only** via the exit code.

TTY state does not affect the output format. There is no `--format` flag; the format is always
NDJSON. `--verbosity` controls how much detail each event carries, never whether it is JSON.

Because every line is flushed on write, consumers can process events in real time while the
hook is still running.

### Exit codes (ADR §2)

| Code | Meaning                                                          |
| ---- | ---------------------------------------------------------------- |
| `0`  | All steps passed (or `--help` / `--version` was requested)       |
| `1`  | A step failed, or a runtime failure occurred                     |
| `2`  | Usage error — unknown flag, invalid value, unwritable log dir    |

## Common fields

Every event carries these fields:

| Field            | Type      | Description                                             |
| ---------------- | --------- | ------------------------------------------------------- |
| `schema_version` | integer   | Always `1` for this revision                            |
| `kind`           | string    | Discriminator; see the table below                      |
| `hook`           | string    | `pre-commit`, `pre-push`, or `install-hooks`            |

Consumers **must** dispatch on `kind` and **must** ignore unknown `kind` values and unknown
fields, so that Phase 2 events can be added without breaking existing consumers.

## Event kinds

| `kind`        | Phase | Emitted when                                     |
| ------------- | ----- | ------------------------------------------------ |
| `hook_start`  | 1     | Immediately on invocation, before any step runs  |
| `step_start`  | 1     | Before a step's subprocess is spawned            |
| `step_end`    | 1     | When a step's subprocess exits                   |
| `hook_result` | 1     | Last event, always emitted for a completed run   |
| `error`       | 1     | Usage or runtime error; terminal for the run     |
| `message`     | 1     | Help/version text, and `install-hooks` progress  |
| `heartbeat`   | 2     | Periodically while a long step is still running  |
| `step_skip`   | 2     | A step was skipped by smart step selection       |
| `cache_hit`   | 2     | A pass record short-circuited the run            |

### `hook_start` (Phase 1)

Emitted within milliseconds of invocation so consumers get an immediate signal that the hook is
alive (AC5).

| Field         | Type             | Notes                                             |
| ------------- | ---------------- | ------------------------------------------------- |
| `total_steps` | integer          | Number of steps that will be attempted            |
| `verbosity`   | string           | `concise` or `verbose`                            |
| `log_dir`     | string           | Directory receiving per-step log files            |
| `steps`       | array of objects | `{ "index": integer, "name": string }`, 1-indexed |

```ndjson
{"schema_version":1,"kind":"hook_start","hook":"pre-commit","total_steps":4,"verbosity":"concise","log_dir":"/tmp","steps":[{"index":1,"name":"Checking for unused dependencies (cargo machete --with-metadata)"}]}
```

### `step_start` (Phase 1)

| Field         | Type    | Notes                                    |
| ------------- | ------- | ---------------------------------------- |
| `step_index`  | integer | 1-indexed                                |
| `total_steps` | integer |                                          |
| `name`        | string  | Human-readable step description          |
| `command`     | string  | **`verbose` only**; omitted in `concise` |

### `step_end` (Phase 1)

| Field             | Type             | Notes                                                  |
| ----------------- | ---------------- | ------------------------------------------------------ |
| `step_index`      | integer          | 1-indexed                                              |
| `total_steps`     | integer          |                                                        |
| `name`            | string           |                                                        |
| `command`         | string           | **`verbose` only**                                     |
| `status`          | string           | `pass` or `fail`                                       |
| `elapsed_seconds` | integer          | Wall-clock duration of the step                        |
| `log_path`        | string           | Full combined output of the step, always written       |
| `failure_tail`    | array of strings | **Failed steps only**; ANSI-stripped output lines      |

`failure_tail` holds the last 10 output lines under `concise`, and the complete step output
under `verbose`. Passing steps never carry output inline — `log_path` always points at the full
log, so nothing is lost.

### `hook_result` (Phase 1)

The final event of a completed run.

| Field             | Type             | Notes                                                       |
| ----------------- | ---------------- | ----------------------------------------------------------- |
| `status`          | string           | `pass` or `fail`                                            |
| `exit_code`       | integer          | Matches the process exit code                               |
| `elapsed_seconds` | integer          | Total wall-clock duration                                   |
| `failed_step`     | string           | Present only when `status` is `fail`                        |
| `steps`           | array of objects | `{ index, name, status, elapsed_seconds, log_path }`        |

Steps that were never reached (the runner stops at the first failure) are absent from `steps`.

### `error` (Phase 1)

Terminal event for usage errors (exit 2) and runtime failures (exit 1). No `hook_result`
follows an `error`.

| Field       | Type    | Notes                                                |
| ----------- | ------- | ---------------------------------------------------- |
| `message`   | string  | Human-readable description of what went wrong        |
| `exit_code` | integer | `1` for runtime failure, `2` for usage error         |

### `message` (Phase 1)

Carries text that has nowhere else to go under a JSON-only contract: `--help` and `--version`
output, and per-hook progress from `install-hooks`.

| Field  | Type   | Notes                                                        |
| ------ | ------ | ------------------------------------------------------------ |
| `text` | string | May contain newlines; they are escaped, keeping the line valid |

## Phase 2 events (specified, not yet implemented)

These are defined now so consumers can be written against the complete schema (T3), and are
implemented in Phase 2 (T17–T20).

### `heartbeat` (Phase 2 — T17)

Emitted every 20–30 seconds while a step is still running, so an active-but-slow step is
distinguishable from a stalled or failed one.

| Field             | Type    | Notes                                     |
| ----------------- | ------- | ----------------------------------------- |
| `step_index`      | integer |                                           |
| `name`            | string  | Step currently running                    |
| `elapsed_seconds` | integer | Time this step has been running so far    |

### `step_skip` (Phase 2 — T18)

Emitted for each step skipped by staged-file-type analysis, so the event record stays complete.

| Field        | Type    | Notes                                                |
| ------------ | ------- | ---------------------------------------------------- |
| `step_index` | integer |                                                      |
| `name`       | string  |                                                      |
| `reason`     | string  | e.g. `markdown-only-changeset`                       |

### `cache_hit` (Phase 2 — T19/T20)

Emitted when an idempotency pass record short-circuits the run.

| Field         | Type   | Notes                                                        |
| ------------- | ------ | ------------------------------------------------------------ |
| `cache_key`   | string | Staged tree SHA + step-config hash, or a pushed commit SHA    |
| `step_subset` | string | `full` or `markdown-only` — which subset the record covers    |

## Consumer notes

- Filter by `kind`. Do not assume event order beyond: `hook_start` first, `hook_result` last.
- A run killed mid-flight has no `hook_result`; treat its absence as an incomplete run, not a pass.
- Per ADR §9, AI agents should capture with `2>.tmp/<command>.stderr`. The matching stdout file
  will always be empty for this binary.
