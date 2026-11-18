//! Port of the v2 `factory/bin/job` subcommands `sync`, `agents`,
//! `summary`, `feedback` and `reject`. Every other argv goes to the v2
//! original (see [`Command::parse`] and `src/job/disk.rs`).

mod advance;
mod clock;
mod cost;
pub mod disk;
mod shell;
mod summary;
mod sync;
mod waiting;

#[cfg(test)]
mod fake;

use std::io;
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::{Outcome, Proc, py};

/// A job's `job.json`, as loaded.
pub type Job = Map<String, Value>;

/// Everything `job` touches outside its own logic. Paths are relative to
/// `factory/jobs/`.
pub trait Workspace {
    /// Reads `factory/config.json`.
    fn read_config(&self) -> io::Result<String>;
    /// Reads `factory/jobs/<path>`; `None` when it does not exist.
    fn read(&self, path: &str) -> io::Result<Option<String>>;
    /// Whether `factory/jobs/<id>/job.json` exists.
    fn has_job(&self, id: &str) -> bool;
    /// Every `factory/jobs/<id>` holding a job.json, sorted by id.
    fn job_ids(&self) -> io::Result<Vec<String>>;
    /// Creates `factory/jobs/<id>/.lock` and holds an exclusive lock on it
    /// until the process exits; again for the same id is a no-op.
    fn lock(&self, id: &str) -> io::Result<()>;
    /// Replaces `factory/jobs/<id>/job.json` through `job.tmp`.
    fn write_job(&self, id: &str, text: &str) -> io::Result<()>;
    /// Absolute path of `factory/jobs/<id>`, as transcripts name it.
    fn job_folder(&self, id: &str) -> String;
    /// The text of every Claude Code subagent transcript of this repository.
    fn transcripts(&self) -> io::Result<Vec<String>>;
    /// All of stdin.
    fn stdin(&self) -> io::Result<String>;
    /// Writes `factory/jobs/<path>`.
    fn write(&self, path: &str, text: &str) -> io::Result<()>;
    /// Names of the entries of `factory/jobs/<dir>`, in no particular order.
    fn list(&self, dir: &str) -> io::Result<Vec<String>>;
    /// Modification time of `factory/jobs/<path>`.
    fn mtime(&self, path: &str) -> io::Result<SystemTime>;
    /// Creates `factory/jobs/<path>` unless it exists (parents must exist).
    fn mkdir(&self, path: &str) -> io::Result<()>;
    /// `Path(path).exists()`: relative paths against the process directory.
    fn exists(&self, path: &str) -> bool;
    /// Runs `program args` in the repository root.
    fn run(&self, program: &str, args: &[&str]) -> io::Result<Proc>;
    /// The current time, in microseconds since the Unix epoch.
    fn now(&self) -> i64;
}

/// The argv shapes the Rust `job` answers itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Sync { job: String },
    Agents { job: String, spec: String },
    Summary { jobs: Vec<String> },
    Feedback { job: String },
    Reject { job: String },
}

impl Command {
    /// `None`: the argv belongs to the v2 fallback.
    pub fn parse(args: &[String]) -> Option<Command> {
        match args {
            [cmd, job] if cmd == "sync" => Some(Command::Sync { job: job.clone() }),
            [cmd, job, spec] if cmd == "agents" => Some(Command::Agents {
                job: job.clone(),
                spec: spec.clone(),
            }),
            [cmd, jobs @ ..] if cmd == "summary" => Some(Command::Summary {
                jobs: jobs.to_vec(),
            }),
            [cmd, job] if cmd == "feedback" => Some(Command::Feedback { job: job.clone() }),
            [cmd, job] if cmd == "reject" => Some(Command::Reject { job: job.clone() }),
            _ => None,
        }
    }
}

/// v2 loads config.json at import time, before reading the arguments, so
/// every argv (fallback included) first needs a readable config with the keys
/// v2 reads at import.
pub fn load_config(ws: &dyn Workspace) -> Result<Value, String> {
    let text = ws.read_config().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => "factory/config.json is missing".to_string(),
        _ => "cannot read factory/config.json".to_string(),
    })?;
    let config = parse_json(&text, "factory/config.json")?;
    for key in ["labels", "base_branch", "concurrency"] {
        if config.get(key).is_none() {
            return Err(format!("factory/config.json has no {key}"));
        }
    }
    Ok(config)
}

/// Runs a command the Rust `job` owns.
pub fn run(command: &Command, config: &Value, ws: &dyn Workspace) -> Outcome {
    let result = match command {
        Command::Sync { job } => cmd_sync(job, config, ws),
        Command::Agents { job, spec } => cmd_agents(job, spec, config, ws),
        Command::Summary { jobs } => summary::cmd_summary(jobs, config, ws),
        Command::Feedback { job } => waiting::cmd_feedback(job, config, ws),
        Command::Reject { job } => waiting::cmd_reject(job, config, ws),
    };
    match result {
        Ok(stdout) => Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        },
        Err(msg) => Outcome::die("job", &msg),
    }
}

/// `job sync <job>`: refresh, save, print the job's state line.
fn cmd_sync(arg: &str, config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let (mut job, file) = resolve(arg, ws)?;
    sync::sync(&mut job, &file, config, ws)?;
    save(&job, &file, ws)?;
    // v2 prints after saving, so a job.json without `reviews` is saved and
    // then fails (DIV-JOB-01).
    let reviews: Vec<String> = match sync::field(&job, &file, "reviews")? {
        Value::Object(reviews) => reviews
            .iter()
            .map(|(k, v)| format!("{k}:{}", py::str_of(v)))
            .collect(),
        _ => return Err(format!("{file} has reviews that are not a table")),
    };
    let gate = sync::field(&job, &file, "gate")?;
    let line = format!(
        "{}: {} round {} gate {} {}",
        py::str_of(sync::field(&job, &file, "id")?),
        py::str_of(sync::field(&job, &file, "stage")?),
        py::str_of(sync::field(&job, &file, "round")?),
        if py::truthy(gate) {
            py::str_of(gate)
        } else {
            "-".into()
        },
        reviews.join(" ")
    );
    Ok(format!("{}\n", py::rstrip(&line)))
}

/// `job agents <job> <json>`: record the routed agent set on an accepted
/// intake job.
fn cmd_agents(arg: &str, spec: &str, config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let (mut job, file) = resolve(arg, ws)?;
    sync::sync(&mut job, &file, config, ws)?;
    // `job["stage"] != "intake" or (job["intake"] or {}).get("decision") != "ACCEPT"`
    let accepted = sync::field(&job, &file, "stage")?.as_str() == Some("intake") && {
        let intake = sync::field(&job, &file, "intake")?;
        py::truthy(intake) && intake.get("decision").and_then(Value::as_str) == Some("ACCEPT")
    };
    if !accepted {
        let id = py::str_of(sync::field(&job, &file, "id")?);
        return Err(format!(
            "{id}: route only an intake job that intake ACCEPTed"
        ));
    }
    let agents = serde_json::from_str(spec).map_err(|e| {
        format!(
            "the agent set is not valid JSON (line {}, column {})",
            e.line(),
            e.column()
        )
    })?;
    job.insert("agents".into(), agents);
    save(&job, &file, ws)?;
    Ok(String::new())
}

/// v2 `resolve()`: an issue number (all digits) or a job id. Returns the job
/// and the path of its job.json, for diagnostics.
pub(crate) fn resolve(arg: &str, ws: &dyn Workspace) -> Result<(Job, String), String> {
    if arg.is_empty() || !arg.bytes().all(|b| b.is_ascii_digit()) {
        return load(arg, ws);
    }
    let issue: Option<i64> = arg.parse().ok();
    let ids = ws
        .job_ids()
        .map_err(|_| "cannot list factory/jobs".to_string())?;
    let mut found = None;
    for id in ids {
        let loaded = load(&id, ws)?;
        let matches =
            issue.is_some() && sync::field(&loaded.0, &loaded.1, "issue")?.as_i64() == issue;
        if matches && found.is_none() {
            found = Some(loaded);
        }
    }
    found.ok_or_else(|| format!("no job for issue #{arg}"))
}

/// v2 `load()`: check, lock, then read.
pub(crate) fn load(id: &str, ws: &dyn Workspace) -> Result<(Job, String), String> {
    let file = format!("factory/jobs/{id}/job.json");
    if !ws.has_job(id) {
        return Err(format!("no job {id}"));
    }
    ws.lock(id)
        .map_err(|_| format!("cannot lock factory/jobs/{id}/.lock"))?;
    let text = ws
        .read(&format!("{id}/job.json"))
        .map_err(|_| format!("cannot read {file}"))?
        .ok_or_else(|| format!("no job {id}"))?;
    match parse_json(&text, &file)? {
        Value::Object(job) => Ok((job, file)),
        _ => Err(format!("{file} is not a JSON object")),
    }
}

/// v2 `save()`: `json.dumps(job, indent=2)` plus a newline, to the job's own
/// folder (by its `id` field).
pub(crate) fn save(job: &Job, file: &str, ws: &dyn Workspace) -> Result<(), String> {
    let id = sync::job_id(job, file)?;
    let text = format!("{}\n", py::dumps_indent(&Value::Object(job.clone())));
    ws.write_job(&id, &text)
        .map_err(|_| format!("cannot write factory/jobs/{id}/job.json"))
}

pub(crate) fn parse_json(text: &str, file: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| {
        format!(
            "{file} is not valid JSON (line {}, column {})",
            e.line(),
            e.column()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::fake::{Fake, case};
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    fn run_case(fake: &Fake, argv: &[&str]) -> Outcome {
        let config = match load_config(fake) {
            Ok(config) => config,
            Err(msg) => return Outcome::die("job", &msg),
        };
        let command = Command::parse(&args(argv)).expect("owned argv");
        run(&command, &config, fake)
    }

    // ---- argv ownership --------------------------------------------------

    #[test]
    fn owns_sync_with_one_job_and_agents_with_job_and_spec() {
        assert_eq!(
            Command::parse(&args(&["sync", "7"])),
            Some(Command::Sync { job: "7".into() })
        );
        assert_eq!(
            Command::parse(&args(&["agents", "7", "{}"])),
            Some(Command::Agents {
                job: "7".into(),
                spec: "{}".into()
            })
        );
    }

    #[test]
    fn owns_summary_feedback_and_reject() {
        assert_eq!(
            Command::parse(&args(&["summary"])),
            Some(Command::Summary { jobs: vec![] })
        );
        assert_eq!(
            Command::parse(&args(&["summary", "1", "x"])),
            Some(Command::Summary {
                jobs: args(&["1", "x"])
            })
        );
        assert_eq!(
            Command::parse(&args(&["feedback", "x"])),
            Some(Command::Feedback { job: "x".into() })
        );
        assert_eq!(
            Command::parse(&args(&["reject", "x"])),
            Some(Command::Reject { job: "x".into() })
        );
    }

    #[test]
    fn leaves_every_other_argv_to_the_fallback() {
        for argv in [
            &[][..],
            &["--help"],
            &["-h"],
            &["sync"],
            &["sync", "a", "b"],
            &["agents", "a"],
            &["agents", "a", "b", "c"],
            &["feedback"],
            &["feedback", "a", "b"],
            &["reject"],
            &["reject", "a", "b"],
            &["advance", "a", "build"],
            &["$@"],
        ] {
            assert_eq!(Command::parse(&args(argv)), None, "{argv:?}");
        }
    }

    // ---- recorded v2 cases (tests/parity/job/rust/app-cases) --------------

    #[test]
    fn sync_01_fresh_job_rewrites_job_json_unchanged() {
        let (fake, want) = case!("sync-01-fresh-job");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_02_intake_header_is_recorded() {
        let (fake, want) = case!("sync-02-intake-full");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_03_header_keys_are_case_and_space_insensitive_and_stop_at_blank() {
        let (fake, want) = case!("sync-03-intake-odd-heade");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_04_crlf_intake_and_blank_surfaces() {
        let (fake, want) = case!("sync-04-intake-crlf");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_05_review_round_reads_gate_and_verdicts() {
        let (fake, want) = case!("sync-05-review-round");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_06_reviews_are_kept_outside_review_stages() {
        let (fake, want) = case!("sync-06-build-stage-keep");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_07_round_without_checks_has_no_gate() {
        let (fake, want) = case!("sync-07-round-without-ch");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_08_lone_cr_ends_the_first_line() {
        let (fake, want) = case!("sync-08-review-cr-only");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_09_issue_number_loads_and_locks_every_job() {
        let (fake, want) = case!("sync-09-by-issue-number");
        assert_eq!(run_case(&fake, &["sync", "2"]), want.outcome);
        fake.assert_files(&want);
        assert_eq!(fake.locks(), ["001-add-a-thing", "002-second", "003-third"]);
    }

    #[test]
    fn sync_10_unknown_issue_number() {
        let (fake, want) = case!("sync-10-issue-not-found");
        assert_eq!(run_case(&fake, &["sync", "99"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_11_unknown_job() {
        let (fake, want) = case!("sync-11-job-missing");
        assert_eq!(run_case(&fake, &["sync", "999-nope"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_12_cost_from_transcripts() {
        let (fake, want) = case!("sync-12-cost-from-transc");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_13_unknown_model_is_refused_without_saving() {
        let (fake, want) = case!("sync-13-cost-unknown-mod");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_14_json_is_written_like_python() {
        let (fake, want) = case!("sync-14-json-formatting");
        assert_eq!(run_case(&fake, &["sync", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn sync_18_id_with_path_characters() {
        let (fake, want) = case!("sync-18-invalid-chars-id");
        assert_eq!(run_case(&fake, &["sync", "bad id/../x"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn agents_01_accepted_intake_records_the_agent_set() {
        let (fake, want) = case!("agents-01-accepted");
        let out = run_case(
            &fake,
            &[
                "agents",
                "001-add-a-thing",
                r#"{"reviewers": {"code": "haiku"}}"#,
            ],
        );
        assert_eq!(out, want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn agents_02_only_intake_stage() {
        let (fake, want) = case!("agents-02-not-intake-sta");
        assert_eq!(
            run_case(
                &fake,
                &["agents", "001-add-a-thing", r#"{"reviewers": {}}"#]
            ),
            want.outcome
        );
        fake.assert_files(&want);
    }

    #[test]
    fn agents_03_only_accepted_intake() {
        let (fake, want) = case!("agents-03-rejected");
        assert_eq!(
            run_case(
                &fake,
                &["agents", "001-add-a-thing", r#"{"reviewers": {}}"#]
            ),
            want.outcome
        );
        fake.assert_files(&want);
    }

    #[test]
    fn agents_04_no_intake() {
        let (fake, want) = case!("agents-04-no-intake");
        assert_eq!(
            run_case(
                &fake,
                &["agents", "001-add-a-thing", r#"{"reviewers": {}}"#]
            ),
            want.outcome
        );
        fake.assert_files(&want);
    }

    #[test]
    fn agents_06_spec_is_stored_with_python_formatting() {
        let (fake, want) = case!("agents-06-spec-formattin");
        let spec = r#"{"reviewers":{"code":"sonnet","ux":"sonnet"},"note":"ñ","n":[1,2.50,1e3]}"#;
        assert_eq!(
            run_case(&fake, &["agents", "001-add-a-thing", spec]),
            want.outcome
        );
        fake.assert_files(&want);
    }

    #[test]
    fn agents_07_by_issue_number() {
        let (fake, want) = case!("agents-07-by-issue-numbe");
        assert_eq!(
            run_case(
                &fake,
                &["agents", "1", r#"{"reviewers": {"code": "opus"}}"#]
            ),
            want.outcome
        );
        fake.assert_files(&want);
    }

    #[test]
    fn agents_08_unknown_job() {
        let (fake, want) = case!("agents-08-job-missing");
        assert_eq!(run_case(&fake, &["agents", "999-nope", "{}"]), want.outcome);
        fake.assert_files(&want);
    }

    // ---- accepted divergences (MIGRATION.md DIV-JOB-*) -------------------
    // v2 ends these with a CPython traceback. Rust keeps v2's exit code, stdout
    // and file effects, and names the cause in one `job:` line.

    fn assert_diagnostic(out: &Outcome, stderr: &str) {
        assert_eq!(out.code, 1);
        assert_eq!(out.stdout, "");
        assert_eq!(out.stderr, stderr);
    }

    #[test]
    fn div_job_01_missing_reviews_fails_after_saving_like_v2() {
        let (fake, want) = case!("sync-15-keys-appended-in");
        let out = run_case(&fake, &["sync", "001-add-a-thing"]);
        assert_diagnostic(
            &out,
            "job: factory/jobs/001-add-a-thing/job.json has no reviews\n",
        );
        assert_eq!(out.code, want.outcome.code);
        fake.assert_files(&want);
    }

    #[test]
    fn div_job_02_invalid_job_json_is_named() {
        let (fake, want) = case!("sync-16-job-json-invalid");
        let out = run_case(&fake, &["sync", "001-add-a-thing"]);
        assert_diagnostic(
            &out,
            "job: factory/jobs/001-add-a-thing/job.json is not valid JSON (line 1, column 3)\n",
        );
        fake.assert_files(&want);
    }

    #[test]
    fn div_job_03_missing_config_is_named() {
        let (fake, want) = case!("sync-17-config-missing");
        assert_diagnostic(
            &Outcome::die("job", &load_config(&fake).unwrap_err()),
            "job: factory/config.json is missing\n",
        );
        fake.assert_files(&want);
    }

    #[test]
    fn div_job_04_invalid_agent_set_is_named_without_saving() {
        let (fake, want) = case!("agents-05-invalid-json");
        let out = run_case(&fake, &["agents", "001-add-a-thing", "{nope"]);
        assert_diagnostic(
            &out,
            "job: the agent set is not valid JSON (line 1, column 2)\n",
        );
        fake.assert_files(&want);
    }

    #[test]
    fn div_job_05_missing_config_is_checked_before_the_arguments() {
        let (fake, _) = case!("example-001");
        assert_eq!(Command::parse(&args(&["$@"])), None);
        assert_diagnostic(
            &Outcome::die("job", &load_config(&fake).unwrap_err()),
            "job: factory/config.json is missing\n",
        );
    }

    // ---- unrecorded v2 crash paths -----------------------------------------

    #[test]
    fn config_problems_are_named() {
        for (config, want) in [
            (
                "{ nope",
                "factory/config.json is not valid JSON (line 1, column 3)",
            ),
            ("[]", "factory/config.json has no labels"),
            (
                r#"{"labels": {}}"#,
                "factory/config.json has no base_branch",
            ),
            (
                r#"{"labels": {}, "base_branch": "main"}"#,
                "factory/config.json has no concurrency",
            ),
        ] {
            let fake = Fake::new().config(config);
            assert_eq!(load_config(&fake), Err(want.to_string()));
        }
        let unreadable = Fake::new().config_error(io::ErrorKind::PermissionDenied);
        assert_eq!(
            load_config(&unreadable),
            Err("cannot read factory/config.json".into())
        );
    }

    #[test]
    fn missing_prices_are_named() {
        let fake = Fake::new().job(
            "001-a",
            r#"{"id": "001-a", "stage": "intake", "round": 0, "reviews": {}}"#,
        );
        let config = serde_json::json!({"labels": {}, "base_branch": "main", "concurrency": 3});
        let out = run(
            &Command::Sync {
                job: "001-a".into(),
            },
            &config,
            &fake,
        );
        assert_diagnostic(
            &out,
            "job: factory/config.json has no prices_usd_per_mtok\n",
        );
    }

    #[test]
    fn job_json_shape_problems_are_named() {
        for (json, want) in [
            ("[]", "factory/jobs/001-a/job.json is not a JSON object"),
            (
                r#"{"stage": "intake"}"#,
                "factory/jobs/001-a/job.json has no id",
            ),
            (
                r#"{"id": 7}"#,
                "factory/jobs/001-a/job.json has an id that is not text",
            ),
            (
                r#"{"id": "001-a"}"#,
                "factory/jobs/001-a/job.json has no round",
            ),
        ] {
            let fake = Fake::new().job("001-a", json);
            let out = run_case(&fake, &["sync", "001-a"]);
            assert_diagnostic(&out, &format!("job: {want}\n"));
        }
    }

    #[test]
    fn io_failures_are_named() {
        let job = r#"{"id": "001-a", "stage": "intake", "round": 0, "reviews": {}}"#;
        let fake = Fake::new().job("001-a", job).fail_writes();
        assert_diagnostic(
            &run_case(&fake, &["sync", "001-a"]),
            "job: cannot write factory/jobs/001-a/job.json\n",
        );
        let fake = Fake::new().job("001-a", job).fail_locks();
        assert_diagnostic(
            &run_case(&fake, &["sync", "001-a"]),
            "job: cannot lock factory/jobs/001-a/.lock\n",
        );
        let fake = Fake::new().job("001-a", job).fail_transcripts();
        assert_diagnostic(
            &run_case(&fake, &["sync", "001-a"]),
            "job: cannot read Claude Code transcripts\n",
        );
        let fake = Fake::new().job("001-a", job).fail_job_list();
        assert_diagnostic(
            &run_case(&fake, &["sync", "5"]),
            "job: cannot list factory/jobs\n",
        );
        let fake = Fake::new().job("001-a", job).fail_reads();
        assert_diagnostic(
            &run_case(&fake, &["sync", "001-a"]),
            "job: cannot read factory/jobs/001-a/job.json\n",
        );
    }

    #[test]
    fn agents_outside_intake_stage_never_reads_intake() {
        let fake = Fake::new().job("001-a", r#"{"id": "001-a", "stage": "spec", "round": 0}"#);
        let out = run_case(&fake, &["agents", "001-a", "{}"]);
        assert_diagnostic(
            &out,
            "job: 001-a: route only an intake job that intake ACCEPTed\n",
        );
    }

    #[test]
    fn issue_lookup_needs_every_job_to_have_an_issue() {
        let fake = Fake::new().job("001-a", r#"{"id": "001-a"}"#);
        let out = run_case(&fake, &["sync", "1"]);
        assert_diagnostic(&out, "job: factory/jobs/001-a/job.json has no issue\n");
    }

    #[test]
    fn huge_issue_numbers_match_no_job() {
        let fake = Fake::new().job("001-a", r#"{"id": "001-a", "issue": 1}"#);
        let out = run_case(&fake, &["sync", "99999999999999999999999999"]);
        assert_diagnostic(&out, "job: no job for issue #99999999999999999999999999\n");
    }
}
