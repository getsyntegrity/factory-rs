//! `gate`: port of v2 `factory/bin/gate`, the deterministic gate.
//!
//! `gate <job>` runs the main checkout's `factory/checks.sh` in the job's
//! worktree and writes `round-N/checks.{log,md}` (`--merge`:
//! `merge-checks.*`); it fails when checks fail or the branch modifies or
//! deletes a file under config.json `protected_paths`. `gate --hook` is the
//! builder's SubagentStop hook: run the checks once, exit 2 on failure.

pub mod disk;

#[cfg(test)]
mod fake;

use std::io;

use serde_json::Value;

use crate::{Outcome, py};

/// A finished process, as v2's `subprocess.run(..., text=True)` sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    /// Negative for a signal, like Python's `returncode`.
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Everything gate touches outside its own logic.
pub trait Host {
    /// Reads `factory/config.json`.
    fn read_config(&self) -> io::Result<String>;
    /// Reads `factory/<path>`.
    fn read(&self, path: &str) -> io::Result<String>;
    /// Writes `factory/<path>`.
    fn write(&self, path: &str, text: &str) -> io::Result<()>;
    /// Runs `git <args>` in `worktree` (`None`: the current directory).
    fn git(&self, worktree: Option<&str>, args: &[&str]) -> io::Result<Proc>;
    /// Runs `bash factory/checks.sh` in `worktree`; also returns the whole
    /// seconds it took.
    fn checks(&self, worktree: Option<&str>) -> io::Result<(Proc, u64)>;
    /// All of stdin.
    fn stdin(&self) -> io::Result<String>;
    /// Reads a file named by the hook event, after `~` expansion.
    fn read_user_file(&self, path: &str) -> io::Result<String>;
}

/// v2's module docstring, printed by `die(__doc__.strip())`.
pub const USAGE: &str = "The deterministic gate: does the job's branch pass the repo's checks?

Usage:
  gate <job-id>            write round-N/checks.md; exit 0 on PASS, 1 on FAIL
  gate <job-id> --merge    same, as round-N/merge-checks.md (job merge runs it)
  gate --hook              builder SubagentStop hook: run the checks once, exit 2 on failure

The gate runs the main checkout's factory/checks.sh inside the job's worktree,
so a branch can't change the checks it is judged by. It also fails when the
branch modifies or deletes a file matching config.json `protected_paths`
(fnmatch patterns, where `*` also matches `/`). Adding new files is fine.";

/// Lines of the checks log quoted in checks.md, and in the hook's nudge.
const TAIL: usize = 80;
const HOOK_TAIL: usize = 40;

/// How gate stops early. v2 has two exits besides PASS/FAIL.
#[derive(Debug)]
enum Stop {
    /// v2 `die(msg)`: exit 2.
    Die(String),
    /// v2 crashes with a CPython traceback (exit 1); Rust names the cause
    /// instead (DIV-GATE-*).
    Crash(String),
}

type Result<T> = std::result::Result<T, Stop>;

fn crash<T>(msg: impl Into<String>) -> Result<T> {
    Err(Stop::Crash(msg.into()))
}

pub fn run(args: &[String], host: &dyn Host) -> Outcome {
    match gate_main(args, host) {
        Ok(outcome) => outcome,
        Err(Stop::Die(msg)) => Outcome {
            code: 2,
            stdout: String::new(),
            stderr: format!("gate: {msg}\n"),
        },
        Err(Stop::Crash(msg)) => Outcome::die("gate", &msg),
    }
}

/// v2 reads config.json at import time, before looking at the arguments.
fn gate_main(args: &[String], host: &dyn Host) -> Result<Outcome> {
    let config = read_json(host.read_config(), "factory/config.json", || {
        "factory/config.json is missing".into()
    })?;
    match args {
        [flag] if flag == "--hook" => hook(host),
        [job] if !job.starts_with('-') => gate(job, false, &config, host),
        [job, flag] if flag == "--merge" => gate(job, true, &config, host),
        _ => Err(Stop::Die(USAGE.into())),
    }
}

fn read_json(
    text: io::Result<String>,
    path: &str,
    missing: impl FnOnce() -> String,
) -> Result<Value> {
    let text = text.map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => Stop::Crash(missing()),
        _ => Stop::Crash(format!("cannot read {path}")),
    })?;
    serde_json::from_str(&text).map_err(|e| {
        Stop::Crash(format!(
            "{path} is not valid JSON (line {}, column {})",
            e.line(),
            e.column()
        ))
    })
}

fn field<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| Stop::Crash(format!("{path} has no {key}")))
}

/// The job.json and the worktree it names (`null`: the current directory).
fn load_job(job_id: &str, host: &dyn Host) -> Result<(Value, Option<String>, String)> {
    let path = format!("factory/jobs/{job_id}/job.json");
    let job = read_json(host.read(&format!("jobs/{job_id}/job.json")), &path, || {
        format!("job {job_id} has no {path}")
    })?;
    let worktree = match field(&job, &path, "worktree")? {
        Value::Null => None,
        Value::String(w) => Some(w.clone()),
        _ => return crash(format!("{path} has a worktree that is not text")),
    };
    Ok((job, worktree, path))
}

fn git(host: &dyn Host, worktree: Option<&str>, args: &[&str]) -> Result<String> {
    let proc = host
        .git(worktree, args)
        .map_err(|_| Stop::Crash("cannot run git".into()))?;
    if proc.code != 0 {
        let stderr = py::universal_newlines(&proc.stderr);
        return Err(Stop::Die(format!(
            "`git {}` failed: {}",
            args.join(" "),
            py::strip(&stderr)
        )));
    }
    Ok(py::universal_newlines(&proc.stdout))
}

/// v2 `protected_changes()`: existing files under `protected_paths` that the
/// branch modified or deleted, as `<status letter> <old path>`.
fn protected_changes(
    config: &Value,
    worktree: Option<&str>,
    host: &dyn Host,
) -> Result<Vec<String>> {
    let base = py::str_of(field(config, "factory/config.json", "base_branch")?);
    let range = format!("origin/{base}...HEAD");
    let diff = git(host, worktree, &["diff", "--name-status", &range])?;
    let mut hits = Vec::new();
    for line in py::splitlines(&diff) {
        let mut parts = line.split('\t');
        let status = parts.next().unwrap_or_default();
        if status.starts_with('A') {
            continue;
        }
        let Some(old) = parts.next() else {
            return crash(format!("unexpected `git diff --name-status` line: {line}"));
        };
        let patterns = field(config, "factory/config.json", "protected_paths")?
            .as_array()
            .ok_or_else(|| {
                Stop::Crash("factory/config.json protected_paths is not a list".into())
            })?;
        if patterns
            .iter()
            .any(|p| p.as_str().is_some_and(|p| py::fnmatch(old, p)))
        {
            let letter: String = status.chars().take(1).collect();
            hits.push(format!("{letter} {old}"));
        }
    }
    Ok(hits)
}

/// v2 `run_checks()`: exit code, stdout then stderr, whole seconds.
fn run_checks(worktree: Option<&str>, host: &dyn Host) -> Result<(i32, String, u64)> {
    let (proc, seconds) = host
        .checks(worktree)
        .map_err(|_| Stop::Crash("cannot run bash factory/checks.sh".into()))?;
    let log = py::universal_newlines(&proc.stdout) + &py::universal_newlines(&proc.stderr);
    Ok((proc.code, log, seconds))
}

/// The last `n` lines of the log, as v2 quotes them.
fn tail(log: &str, n: usize) -> String {
    let lines = py::splitlines(py::rstrip(log));
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn gate(job_id: &str, merge: bool, config: &Value, host: &dyn Host) -> Result<Outcome> {
    let (job, worktree, path) = load_job(job_id, host)?;
    let worktree = worktree.as_deref();
    let round = field(&job, &path, "round")?;
    let round_n = round
        .as_f64()
        .ok_or_else(|| Stop::Crash(format!("{path} has a round that is not a number")))?;
    if round_n < 1.0 {
        return Err(Stop::Die(format!("{job_id} hasn't started a build round")));
    }
    let round = py::str_of(round);
    let round_dir = format!("jobs/{job_id}/round-{round}");

    let commit = py::strip(&git(host, worktree, &["rev-parse", "HEAD"])?).to_string();
    let protected = protected_changes(config, worktree, host)?;
    let (code, log, seconds) = run_checks(worktree, host)?;
    let pass = code == 0 && protected.is_empty();
    let verdict = if pass { "PASS" } else { "FAIL" };
    let name = if merge { "merge-checks" } else { "checks" };

    let write = |file: &str, text: &str| {
        host.write(&format!("{round_dir}/{file}"), text)
            .map_err(|_| Stop::Crash(format!("cannot write factory/{round_dir}/{file}")))
    };
    write(&format!("{name}.log"), &log)?;

    let checks_line = if code == 0 {
        format!("checks.sh passed in {seconds}s")
    } else {
        format!("checks.sh failed with exit {code} in {seconds}s")
    };
    let protected_line = if protected.is_empty() {
        "Protected paths: none changed".to_string()
    } else {
        format!("Protected paths: changed: {}", protected.join(", "))
    };
    let short: String = commit.chars().take(7).collect();
    // v2 says "main" here whatever base_branch is.
    let commit_line = if merge {
        format!("Commit {short}, synced with the latest main before merging")
    } else {
        format!("Commit {short}")
    };
    let protected_section = if protected.is_empty() {
        "None changed.".to_string()
    } else {
        protected
            .iter()
            .map(|p| format!("- {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let md = format!(
        "GATE: {verdict}\nCOMMIT: {commit}\n\n## Summary\n- {checks_line}\n- {protected_line}\n- {commit_line}\n\n\
         ## Protected paths\n{protected_section}\n\n## checks.sh\nExit {code} in {seconds}s. \
         Last {TAIL} lines of {name}.log:\n\n```\n{}\n```\n",
        tail(&log, TAIL)
    );
    write(&format!("{name}.md"), &md)?;

    let protected_note = if protected.is_empty() {
        String::new()
    } else {
        format!(", protected paths changed: {}", protected.join(", "))
    };
    Ok(Outcome {
        code: if pass { 0 } else { 1 },
        stdout: format!(
            "GATE: {verdict} ({job_id} round {round}, checks.sh exit {code} in {seconds}s{protected_note})\n"
        ),
        stderr: String::new(),
    })
}

/// v2 `hook()`: the builder's SubagentStop hook.
fn hook(host: &dyn Host) -> Result<Outcome> {
    let silent = |code| Outcome {
        code,
        stdout: String::new(),
        stderr: String::new(),
    };
    let event = read_json(host.stdin(), "the hook event on stdin", String::new)?;
    if event.get("stop_hook_active").is_some_and(py::truthy) {
        return Ok(silent(0)); // one nudge per stop; the gate decides
    }
    let transcript = py::str_of(field(&event, "the hook event", "agent_transcript_path")?);
    let text = host
        .read_user_file(&transcript)
        .map_err(|_| Stop::Crash(format!("cannot read {transcript}")))?;
    let text = py::universal_newlines(&text);
    let first = text.split_inclusive('\n').next().unwrap_or_default();
    let first: Value = serde_json::from_str(first)
        .map_err(|_| Stop::Crash(format!("the first line of {transcript} is not valid JSON")))?;
    let content = first
        .get("message")
        .and_then(|m| m.get("content"))
        .ok_or_else(|| Stop::Crash(format!("the prompt in {transcript} has no message content")))?;
    let prompt = match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .map(|b| match b {
                Value::Object(block) => Ok(block.get("text").map(py::str_of).unwrap_or_default()),
                _ => crash(format!(
                    "the prompt in {transcript} has a block that is not an object"
                )),
            })
            .collect::<Result<Vec<_>>>()?
            .join(" "),
        _ => {
            return crash(format!(
                "the prompt in {transcript} is neither text nor blocks"
            ));
        }
    };
    let Some((_, after)) = prompt.split_once("Job: ") else {
        return crash("the subagent prompt names no job (\"Job: <id>.\")");
    };
    let job_id = after.split_once('.').map_or(after, |(id, _)| id);
    let (_, worktree, _) = load_job(job_id, host)?;
    let (code, log, _) = run_checks(worktree.as_deref(), host)?;
    if code == 0 {
        return Ok(silent(0));
    }
    Ok(Outcome {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "factory/checks.sh fails in your worktree (exit {code}). The gate will reject this \
             build. Fix it, or write STATUS: BLOCKED in build.md if you can't.\n\n{}\n",
            tail(&log, HOOK_TAIL)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::fake::{self, case};
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    macro_rules! recorded {
        ($test:ident, $case:literal, $argv:expr) => {
            #[test]
            fn $test() {
                let (host, want) = case!($case);
                assert_eq!(run(&args(&$argv), &host), want.outcome);
                host.assert_writes(&want);
            }
        };
    }

    const ID: &str = "001-add-a-thing";

    // ---- recorded v2 cases (tests/parity/gate/rust/app-cases) -------------
    recorded!(argv_01_no_args_prints_usage, "argv-01-no-args", []);
    recorded!(argv_02_dash_arg_prints_usage, "argv-02-dash-arg", ["-x"]);
    recorded!(
        argv_03_three_args_print_usage,
        "argv-03-three-args",
        [ID, "--merge", "x"]
    );
    recorded!(
        argv_04_second_arg_must_be_merge,
        "argv-04-second-not-merge",
        [ID, "--fast"]
    );
    recorded!(
        argv_05_hook_takes_no_args,
        "argv-05-hook-with-extra",
        ["--hook", "x"]
    );
    recorded!(gate_01_pass_writes_checks_files, "gate-01-pass", [ID]);
    recorded!(
        gate_02_failing_checks_log_stdout_then_stderr,
        "gate-02-checks-fail",
        [ID]
    );
    recorded!(
        gate_03_protected_paths_follow_fnmatch,
        "gate-03-protected-paths",
        [ID]
    );
    recorded!(
        gate_04_protected_and_failing,
        "gate-04-protected-and-fa",
        [ID]
    );
    recorded!(
        gate_05_tail_is_80_lines_with_universal_newlines,
        "gate-05-long-crlf-output",
        [ID]
    );
    recorded!(gate_06_no_output, "gate-06-no-output", [ID]);
    recorded!(gate_07_round_zero_is_refused, "gate-07-round-zero", [ID]);
    recorded!(
        gate_08_rev_parse_failure_is_reported,
        "gate-08-rev-parse-fails",
        [ID]
    );
    recorded!(gate_09_diff_failure_is_reported, "gate-09-diff-fails", [ID]);
    recorded!(
        gate_10_diff_uses_the_base_branch,
        "gate-10-other-base-branc",
        [ID]
    );
    recorded!(
        gate_14_missing_checks_sh_fails,
        "gate-14-checks-sh-missin",
        [ID]
    );
    recorded!(gate_15_unicode_output, "gate-15-unicode-output", [ID]);
    recorded!(
        merge_01_pass_writes_merge_checks,
        "merge-01-pass",
        [ID, "--merge"]
    );
    recorded!(merge_02_fail, "merge-02-fail", [ID, "--merge"]);
    recorded!(
        hook_01_stop_hook_active_is_silent,
        "hook-01-stop-hook-active",
        ["--hook"]
    );
    recorded!(
        hook_02_passing_checks_are_silent,
        "hook-02-checks-pass",
        ["--hook"]
    );
    recorded!(
        hook_03_failing_checks_nudge_with_40_lines,
        "hook-03-checks-fail",
        ["--hook"]
    );

    // ---- accepted divergences (MIGRATION.md DIV-GATE-*) ------------------
    // v2 ends these with a CPython traceback (exit 1). Rust keeps exit 1,
    // empty stdout and v2's file effects, and names the cause.

    macro_rules! diagnostic {
        ($test:ident, $case:literal, $argv:expr, $stderr:literal) => {
            #[test]
            fn $test() {
                let (host, want) = case!($case);
                let out = run(&args(&$argv), &host);
                assert_eq!(out.code, 1);
                assert_eq!(out.code, want.outcome.code);
                assert_eq!(out.stdout, want.outcome.stdout);
                assert_eq!(out.stderr, $stderr);
                host.assert_writes(&want);
            }
        };
    }

    diagnostic!(
        div_gate_01_missing_job_is_named,
        "gate-11-job-missing",
        ["999-nope"],
        "gate: job 999-nope has no factory/jobs/999-nope/job.json\n"
    );
    diagnostic!(
        div_gate_02_missing_round_folder_is_named,
        "gate-12-round-dir-missin",
        [ID],
        "gate: cannot write factory/jobs/001-add-a-thing/round-1/checks.log\n"
    );
    diagnostic!(
        div_gate_03_missing_config_is_named,
        "gate-13-config-missing",
        [ID],
        "gate: factory/config.json is missing\n"
    );
    diagnostic!(
        div_gate_04_invalid_hook_event_is_named,
        "hook-04-invalid-stdin",
        ["--hook"],
        "gate: the hook event on stdin is not valid JSON (line 1, column 2)\n"
    );
    diagnostic!(
        div_gate_05_prompt_without_job_is_named,
        "hook-05-no-job-in-prompt",
        ["--hook"],
        "gate: the subagent prompt names no job (\"Job: <id>.\")\n"
    );
    diagnostic!(
        div_gate_06_merge_of_missing_job_is_named,
        "merge-03-dash-job",
        ["-weird", "--merge"],
        "gate: job -weird has no factory/jobs/-weird/job.json\n"
    );
    diagnostic!(
        div_gate_07_config_is_read_before_the_arguments,
        "example-001",
        ["$@"],
        "gate: factory/config.json is missing\n"
    );

    // ---- unrecorded v2 crash paths -----------------------------------------

    /// gate-01-pass with `edit` applied to its input files.
    fn pass_case(edit: impl FnOnce(&mut fake::Fake)) -> Outcome {
        let (mut host, _) = case!("gate-01-pass");
        edit(&mut host);
        run(&args(&[ID]), &host)
    }

    fn job_with(field: &str, value: &str) -> String {
        format!(r#"{{"id": "{ID}", "round": 1, "worktree": "wt", "{field}": {value}}}"#)
    }

    fn assert_crash(out: Outcome, stderr: &str) {
        assert_eq!(out, Outcome::die("gate", stderr));
    }

    #[test]
    fn null_worktree_runs_in_the_current_directory() {
        let out = pass_case(|h| {
            h.files.insert(
                format!("factory/jobs/{ID}/job.json"),
                job_with("worktree", "null"),
            );
            h.files.remove("git-double/cwd");
        });
        assert_eq!(out.code, 0, "{out:?}");
    }

    #[test]
    fn job_json_shape_problems_are_named() {
        let file = format!("factory/jobs/{ID}/job.json");
        for (json, want) in [
            (
                job_with("worktree", "7"),
                format!("{file} has a worktree that is not text"),
            ),
            (
                job_with("round", r#""one""#),
                format!("{file} has a round that is not a number"),
            ),
            (
                format!(r#"{{"id": "{ID}", "round": 1}}"#),
                format!("{file} has no worktree"),
            ),
        ] {
            let out = pass_case(|h| {
                h.files.insert(file.clone(), json);
            });
            assert_crash(out, &want);
        }
    }

    #[test]
    fn unreadable_config_is_named() {
        let out = pass_case(|h| h.fail.push("config"));
        assert_crash(out, "cannot read factory/config.json");
    }

    #[test]
    fn processes_that_cannot_start_are_named() {
        assert_crash(pass_case(|h| h.fail.push("git")), "cannot run git");
        assert_crash(
            pass_case(|h| h.fail.push("checks")),
            "cannot run bash factory/checks.sh",
        );
    }

    #[test]
    fn git_double_refusals_surface_as_git_failures() {
        let out = pass_case(|h| {
            h.files.remove("git-double/rev-parse.args");
        });
        assert_eq!(out.code, 2);
        assert_eq!(
            out.stderr,
            "gate: `git rev-parse HEAD` failed: git double: no answer for: rev-parse HEAD\n"
        );
        let out = pass_case(|h| {
            h.files.insert("git-double/cwd".into(), "elsewhere".into());
        });
        assert_eq!(
            out.stderr,
            "gate: `git rev-parse HEAD` failed: git double: unexpected call\n"
        );
    }

    #[test]
    fn diff_lines_without_a_tab_are_named() {
        let out = pass_case(|h| {
            h.files
                .insert("git-double/diff.stdout".into(), "M\n".into());
        });
        assert_crash(out, "unexpected `git diff --name-status` line: M");
    }

    #[test]
    fn protected_paths_must_be_a_list() {
        let out = pass_case(|h| {
            h.files.insert(
                "factory/config.json".into(),
                r#"{"base_branch": "main", "protected_paths": "factory/*"}"#.into(),
            );
            h.files.insert(
                "git-double/diff.stdout".into(),
                "M\tfactory/checks.sh\n".into(),
            );
        });
        assert_crash(out, "factory/config.json protected_paths is not a list");
    }

    #[test]
    fn checks_output_keeps_quotes_and_backslashes() {
        let out = pass_case(|h| {
            h.files.insert(
                "factory/checks.sh".into(),
                "printf '%s' $'it\\'s a \\\\ path\\n'\nexit 0\n".into(),
            );
        });
        assert_eq!(out.code, 0);
    }

    /// hook-02-checks-pass with its transcript replaced by `text`.
    fn hook_with_transcript(text: Option<&str>) -> Outcome {
        let (mut host, _) = case!("hook-02-checks-pass");
        match text {
            Some(text) => host
                .files
                .insert("transcripts/agent-1.jsonl".into(), text.into()),
            None => host.files.remove("transcripts/agent-1.jsonl"),
        };
        run(&args(&["--hook"]), &host)
    }

    #[test]
    fn hook_transcript_problems_are_named() {
        let t = "transcripts/agent-1.jsonl";
        assert_crash(hook_with_transcript(None), &format!("cannot read {t}"));
        assert_crash(
            hook_with_transcript(Some("nope\n")),
            &format!("the first line of {t} is not valid JSON"),
        );
        assert_crash(
            hook_with_transcript(Some("{}\n")),
            &format!("the prompt in {t} has no message content"),
        );
        assert_crash(
            hook_with_transcript(Some(r#"{"message": {"content": ["Job: x."]}}"#)),
            &format!("the prompt in {t} has a block that is not an object"),
        );
        assert_crash(
            hook_with_transcript(Some(r#"{"message": {"content": 5}}"#)),
            &format!("the prompt in {t} is neither text nor blocks"),
        );
    }

    #[test]
    fn hook_job_id_runs_to_the_first_dot_or_the_end() {
        let out = hook_with_transcript(Some(
            r#"{"message": {"content": [{"text": "Job: 001-add-a-thing"}]}}"#,
        ));
        assert_eq!(out.code, 0, "{out:?}");
    }
}
