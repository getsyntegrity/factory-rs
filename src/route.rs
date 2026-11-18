//! `route <job-id>`: map intake's risk and surfaces to the job's reviewer set.
//!
//! Port of v2 `factory/bin/route`. Reads `risk` and `surfaces` from the job's
//! intake (through `job sync`), looks them up in config.json's `routing` and
//! `surface_reviewers` tables, records the result with `job agents` and prints
//! the reviewer set.

use std::io;

use serde_json::{Map, Value, json};

use crate::py;

/// What `factory/bin/job` reported back. Its stdout is never shown by route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutput {
    pub success: bool,
    pub stderr: String,
}

/// Everything route touches outside its own logic.
pub trait Factory {
    /// Reads a file relative to the `factory/` directory.
    fn read(&self, path: &str) -> io::Result<String>;
    /// Runs `factory/bin/job` with `args`.
    ///
    /// FALLBACK (pending slice): the real implementation runs the Python `job`
    /// from v2 until `job` is ported.
    fn job(&self, args: &[&str]) -> io::Result<JobOutput>;
}

pub use crate::Outcome;

pub fn run(args: &[String], factory: &dyn Factory) -> Outcome {
    match route(args, factory) {
        Ok(stdout) => Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        },
        Err(msg) => Outcome {
            code: 1,
            stdout: String::new(),
            stderr: format!("route: {msg}\n"),
        },
    }
}

/// Why route stopped: the text after `route: ` on stderr.
///
/// Most messages are v2's own `die()` text. Where v2 crashes with a CPython
/// traceback instead (missing or invalid JSON files, a job that cannot run),
/// route prints one stable line naming the cause: the accepted divergences
/// DIV-ROUTE-01..04 in MIGRATION.md.
type Failure = String;

type Table = Map<String, Value>;

/// v2 reads config.json at import time, before looking at the arguments.
fn route(args: &[String], factory: &dyn Factory) -> Result<String, Failure> {
    let config = read_json(factory, "config.json", || {
        "factory/config.json is missing".to_string()
    })?;
    let [job_id] = args else {
        return Err("usage: route <job-id>".into());
    };

    call_job(factory, &["sync", job_id])?;
    let job_path = format!("jobs/{job_id}/job.json");
    let job = read_json(factory, &job_path, || {
        format!("job {job_id} has no factory/{job_path}")
    })?;
    let intake = match job.get("intake") {
        Some(Value::Object(intake)) if !intake.is_empty() => intake,
        _ => return Err(format!("{job_id} has no intake.md")),
    };

    let routing = table(&config, "routing")?;
    let risk = intake.get("risk").and_then(Value::as_str);
    let mut reviewers = match risk.and_then(|r| routing.get(r)) {
        Some(Value::Object(set)) if !set.is_empty() => set.clone(),
        _ => {
            return Err(format!(
                "risk {} isn't in config.json routing ({})",
                py::repr(risk),
                keys(routing)
            ));
        }
    };

    let surface_reviewers = table(&config, "surface_reviewers")?;
    let surfaces = intake
        .get("surfaces")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for surface in surfaces.iter().filter_map(Value::as_str) {
        let Some(Value::Object(extra)) = surface_reviewers.get(surface) else {
            return Err(format!(
                "surface {} isn't in config.json surface_reviewers ({})",
                py::repr(Some(surface)),
                keys(surface_reviewers)
            ));
        };
        reviewers.extend(extra.clone());
    }

    let agents = json!({ "reviewers": reviewers });
    call_job(factory, &["agents", job_id, &py::dumps(&agents)])?;

    let summary: Vec<String> = reviewers
        .iter()
        .map(|(role, model)| format!("{role} {}", text(model)))
        .collect();
    Ok(format!("{job_id}: reviewers {}\n", summary.join(", ")))
}

fn call_job(factory: &dyn Factory, args: &[&str]) -> Result<(), Failure> {
    let out = factory
        .job(args)
        .map_err(|_| "cannot run factory/bin/job".to_string())?;
    if out.success {
        Ok(())
    } else {
        Err(out.stderr.trim().to_string())
    }
}

/// Reads and parses `factory/<path>`; `missing` names the cause when the file
/// does not exist.
fn read_json(
    factory: &dyn Factory,
    path: &str,
    missing: impl FnOnce() -> String,
) -> Result<Value, Failure> {
    let text = factory.read(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => missing(),
        _ => format!("cannot read factory/{path}"),
    })?;
    serde_json::from_str(&text).map_err(|e| {
        format!(
            "factory/{path} is not valid JSON (line {}, column {})",
            e.line(),
            e.column()
        )
    })
}

fn table<'a>(config: &'a Value, key: &str) -> Result<&'a Table, Failure> {
    config
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("factory/config.json has no {key} table"))
}

fn keys(table: &Table) -> String {
    table
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Python's `str()` of a config value: strings bare, anything else as JSON.
fn text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const CONFIG: &str = include_str!(
        "../tests/parity/route/rust/app-cases/05-low-no-surfaces.in/factory/config.json"
    );
    const ID: &str = "001-add-a-thing";

    /// In-memory factory: files by path, scripted `job` answers, recorded calls.
    type AgentsReply = Box<dyn Fn(&[&str]) -> JobOutput>;

    struct Fake {
        files: HashMap<String, String>,
        sync: io::Result<JobOutput>,
        agents: AgentsReply,
        calls: RefCell<Vec<Vec<String>>>,
    }

    fn ok() -> JobOutput {
        JobOutput {
            success: true,
            stderr: String::new(),
        }
    }

    fn failed(stderr: &str) -> JobOutput {
        JobOutput {
            success: false,
            stderr: stderr.to_string(),
        }
    }

    impl Fake {
        fn new() -> Self {
            Fake {
                files: HashMap::from([("config.json".to_string(), CONFIG.to_string())]),
                sync: Ok(ok()),
                agents: Box::new(|_| ok()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn job_json(mut self, id: &str, intake: &str) -> Self {
            let json = format!(r#"{{"id": "{id}", "stage": "intake", "intake": {intake}}}"#);
            self.files.insert(format!("jobs/{id}/job.json"), json);
            self
        }

        fn intake(self, risk: &str, surfaces: &str) -> Self {
            let intake = format!(
                r#"{{"decision": "ACCEPT", "rescoped": false, "risk": {risk}, "surfaces": {surfaces}}}"#
            );
            self.job_json(ID, &intake)
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.borrow().clone()
        }
    }

    impl Factory for Fake {
        fn read(&self, path: &str) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No such file or directory"))
        }

        fn job(&self, args: &[&str]) -> io::Result<JobOutput> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|a| a.to_string()).collect());
            match args[0] {
                "sync" => match &self.sync {
                    Ok(out) => Ok(out.clone()),
                    Err(e) => Err(io::Error::new(e.kind(), e.to_string())),
                },
                _ => Ok((self.agents)(args)),
            }
        }
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    /// Exit code, stdout and stderr recorded from v2 by `rust-anything app record`.
    macro_rules! recorded {
        ($case:literal) => {{
            let toml = include_str!(concat!(
                "../tests/parity/route/rust/app-cases/",
                $case,
                ".toml"
            ));
            let code = toml
                .lines()
                .find_map(|l| l.strip_prefix("status.code = "))
                .map(|c| c.parse::<i32>().unwrap())
                .unwrap_or(0);
            Outcome {
                code,
                stdout: include_str!(concat!(
                    "../tests/parity/route/rust/app-cases/",
                    $case,
                    ".stdout"
                ))
                .to_string(),
                stderr: include_str!(concat!(
                    "../tests/parity/route/rust/app-cases/",
                    $case,
                    ".stderr"
                ))
                .to_string(),
            }
        }};
    }

    #[test]
    fn case_01_no_args_prints_usage() {
        let fake = Fake::new();
        assert_eq!(run(&args(&[]), &fake), recorded!("01-no-args"));
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn case_02_two_args_prints_usage() {
        let fake = Fake::new();
        assert_eq!(run(&args(&[ID, "extra"]), &fake), recorded!("02-two-args"));
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn case_03_sync_error_is_reported_stripped() {
        let fake = Fake {
            sync: Ok(failed("job: no job 999-no-such-job\n")),
            ..Fake::new()
        };
        assert_eq!(
            run(&args(&["999-no-such-job"]), &fake),
            recorded!("03-job-missing")
        );
        assert_eq!(fake.calls(), vec![args(&["sync", "999-no-such-job"])]);
    }

    #[test]
    fn case_04_null_intake_is_refused() {
        let fake = Fake::new().job_json(ID, "null");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("04-no-intake"));
        assert_eq!(fake.calls().len(), 1);
    }

    #[test]
    fn case_05_low_risk_routes_code_haiku() {
        let fake = Fake::new().intake(r#""low""#, "[]");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("05-low-no-surfaces"));
        assert_eq!(
            fake.calls(),
            vec![
                args(&["sync", ID]),
                args(&["agents", ID, r#"{"reviewers": {"code": "haiku"}}"#])
            ]
        );
    }

    #[test]
    fn case_06_medium_risk_routes_code_sonnet() {
        let fake = Fake::new().intake(r#""medium""#, "[]");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("06-medium-no-surfaces"));
    }

    #[test]
    fn case_07_high_risk_routes_code_sonnet() {
        let fake = Fake::new().intake(r#""high""#, "[]");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("07-high-no-surfaces"));
    }

    #[test]
    fn case_08_ui_surface_adds_ux_and_ui() {
        let fake = Fake::new().intake(r#""low""#, r#"["ui"]"#);
        assert_eq!(run(&args(&[ID]), &fake), recorded!("08-surface-ui"));
    }

    #[test]
    fn case_09_security_surface_adds_security() {
        let fake = Fake::new().intake(r#""medium""#, r#"["security"]"#);
        assert_eq!(run(&args(&[ID]), &fake), recorded!("09-surface-security"));
    }

    #[test]
    fn case_10_surfaces_add_in_order() {
        let fake = Fake::new().intake(r#""high""#, r#"["ui", "security"]"#);
        assert_eq!(
            run(&args(&[ID]), &fake),
            recorded!("10-surfaces-ui-security")
        );
    }

    #[test]
    fn case_11_unknown_risk_lists_routing_keys() {
        let fake = Fake::new().intake(r#""extreme""#, "[]");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("11-unknown-risk"));
        assert_eq!(fake.calls().len(), 1);
    }

    #[test]
    fn case_12_unknown_surface_lists_surface_keys() {
        let fake = Fake::new().intake(r#""low""#, r#"["ui", "api"]"#);
        assert_eq!(run(&args(&[ID]), &fake), recorded!("12-unknown-surface"));
        assert_eq!(fake.calls().len(), 1);
    }

    #[test]
    fn case_13_sync_failure_stops_before_reading_job() {
        let fake = Fake {
            sync: Ok(failed("double: sync sync 001-add-a-thing\nsync exploded\n")),
            ..Fake::new()
        };
        assert_eq!(run(&args(&[ID]), &fake), recorded!("13-sync-fails"));
    }

    #[test]
    fn case_14_agents_failure_is_reported_with_python_json_argument() {
        let fake = Fake {
            agents: Box::new(|a| failed(&format!("agents refused: [{}] [{}]\n", a[1], a[2]))),
            ..Fake::new().intake(r#""high""#, r#"["ui", "security"]"#)
        };
        assert_eq!(run(&args(&[ID]), &fake), recorded!("14-agents-fails"));
    }

    #[test]
    fn case_15a_unresolvable_id_reports_job_error() {
        let fake = Fake {
            sync: Ok(failed("job: no job bad id/../x\n")),
            ..Fake::new()
        };
        assert_eq!(
            run(&args(&["bad id/../x"]), &fake),
            recorded!("15a-invalid-chars-id")
        );
    }

    #[test]
    fn case_17_agents_refusal_from_job_is_reported() {
        let fake = Fake {
            agents: Box::new(|_| {
                failed("job: 001-add-a-thing: route only an intake job that intake ACCEPTed\n")
            }),
            ..Fake::new().intake(r#""low""#, "[]")
        };
        assert_eq!(run(&args(&[ID]), &fake), recorded!("17-intake-rejected"));
    }

    #[test]
    fn case_18_missing_risk_is_shown_as_none() {
        let fake = Fake::new().intake("null", "[]");
        assert_eq!(
            run(&args(&[ID]), &fake),
            recorded!("18-intake-without-risk")
        );
    }

    #[test]
    fn case_19_risk_is_shown_with_python_repr() {
        let fake = Fake::new().intake(r#""it's""#, "[]");
        assert_eq!(run(&args(&[ID]), &fake), recorded!("19-quote-in-risk"));
    }

    /// Accepted divergences (MIGRATION.md, DIV-ROUTE-01..04): v2 ends these
    /// with a CPython traceback. Rust keeps exit 1, empty stdout and no write
    /// of its own (no `job agents`), and names the cause in one stable line.
    fn assert_diagnostic(out: &Outcome, fake: &Fake, stderr: &str) {
        assert_eq!(out.code, 1);
        assert_eq!(out.stdout, "");
        assert!(out.stderr.starts_with("route: "), "{}", out.stderr);
        assert_eq!(out.stderr, stderr);
        assert!(fake.calls().iter().all(|c| c[0] != "agents"));
    }

    #[test]
    fn div_route_01_missing_config_is_named() {
        let mut fake = Fake::new().intake(r#""low""#, "[]");
        fake.files.remove("config.json");
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(&out, &fake, "route: factory/config.json is missing\n");
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn div_route_02_invalid_config_is_named_with_its_position() {
        let mut fake = Fake::new().intake(r#""low""#, "[]");
        fake.files
            .insert("config.json".into(), "{ not json\n".into());
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(
            &out,
            &fake,
            "route: factory/config.json is not valid JSON (line 1, column 3)\n",
        );
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn div_route_03_id_without_job_folder_is_named() {
        let fake = Fake::new().intake(r#""low""#, "[]");
        let out = run(&args(&["1"]), &fake);
        assert_diagnostic(&out, &fake, "route: job 1 has no factory/jobs/1/job.json\n");
        assert_eq!(fake.calls(), vec![args(&["sync", "1"])]);
    }

    #[test]
    fn div_route_04_missing_config_wins_over_bad_arguments() {
        let mut fake = Fake::new();
        fake.files.remove("config.json");
        let out = run(&args(&["$@"]), &fake);
        assert_diagnostic(&out, &fake, "route: factory/config.json is missing\n");
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn unreadable_config_is_named() {
        let fake = Fake::new();
        let out = run(&args(&[]), &Unreadable(&fake));
        assert_diagnostic(&out, &fake, "route: cannot read factory/config.json\n");
    }

    #[test]
    fn invalid_job_json_is_named_with_its_position() {
        let mut fake = Fake::new();
        fake.files
            .insert(format!("jobs/{ID}/job.json"), "{\n  oops".into());
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(
            &out,
            &fake,
            "route: factory/jobs/001-add-a-thing/job.json is not valid JSON (line 2, column 3)\n",
        );
    }

    #[test]
    fn config_without_tables_is_named() {
        let mut fake = Fake::new().intake(r#""low""#, r#"["ui"]"#);
        fake.files.insert("config.json".into(), "{}".into());
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(
            &out,
            &fake,
            "route: factory/config.json has no routing table\n",
        );
        fake.files.insert(
            "config.json".into(),
            r#"{"routing": {"low": {"code": "haiku"}}}"#.into(),
        );
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(
            &out,
            &fake,
            "route: factory/config.json has no surface_reviewers table\n",
        );
    }

    #[test]
    fn job_that_cannot_start_is_named() {
        let fake = Fake {
            sync: Err(io::Error::new(io::ErrorKind::NotFound, "gone")),
            ..Fake::new()
        };
        let out = run(&args(&[ID]), &fake);
        assert_diagnostic(&out, &fake, "route: cannot run factory/bin/job\n");
    }

    /// Wraps a fake so every read fails with a permission error.
    struct Unreadable<'a>(&'a Fake);

    impl Factory for Unreadable<'_> {
        fn read(&self, _: &str) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        }

        fn job(&self, args: &[&str]) -> io::Result<JobOutput> {
            self.0.job(args)
        }
    }

    #[test]
    fn empty_intake_object_counts_as_no_intake() {
        let fake = Fake::new().job_json(ID, "{}");
        let out = run(&args(&[ID]), &fake);
        assert_eq!(out.stderr, "route: 001-add-a-thing has no intake.md\n");
        assert_eq!(out.code, 1);
    }

    #[test]
    fn empty_routing_entry_counts_as_unknown_risk() {
        let mut fake = Fake::new().intake(r#""low""#, "[]");
        fake.files.insert(
            "config.json".into(),
            r#"{"routing": {"low": {}, "high": {"code": "opus"}}, "surface_reviewers": {}}"#.into(),
        );
        let out = run(&args(&[ID]), &fake);
        assert_eq!(
            out.stderr,
            "route: risk 'low' isn't in config.json routing (low, high)\n"
        );
        assert_eq!(out.code, 1);
    }

    #[test]
    fn surface_overrides_keep_first_position_and_take_new_model() {
        let mut fake = Fake::new().intake(r#""low""#, r#"["deep"]"#);
        fake.files.insert(
            "config.json".into(),
            r#"{"routing": {"low": {"code": "haiku", "ux": "haiku"}}, "surface_reviewers": {"deep": {"code": "opus"}}}"#
                .into(),
        );
        let out = run(&args(&[ID]), &fake);
        assert_eq!(
            out.stdout,
            "001-add-a-thing: reviewers code opus, ux haiku\n"
        );
        assert_eq!(out.code, 0);
    }
}
