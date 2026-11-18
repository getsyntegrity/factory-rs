//! In-memory [`Workspace`] for unit tests, plus `case!` to load a v2 case
//! recorded by `rust-anything app record` under tests/parity/job (or
//! tests/parity/job-reject).
//!
//! `run` behaves like the `gh`/`git` doubles of tests/parity/job/launch.sh:
//! every call is appended to `doubles.log`, and answers come from the case's
//! `doubles/<tool>/<key>.{stdout,stderr,code}` files.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, SystemTime};

use super::Workspace;
use crate::{Outcome, Proc};

/// The absolute repository path the fake pretends to live at.
pub const REPO: &str = "/repo";

/// The fake's clock: 2026-11-18T03:04:05Z.
pub const NOW: i64 = 1_794_971_045_000_000;

#[derive(Default)]
pub struct Fake {
    config: Option<String>,
    config_error: Option<io::ErrorKind>,
    /// `factory/jobs/<path>` → text.
    files: BTreeMap<String, String>,
    transcripts: Vec<String>,
    fail: Vec<&'static str>,
    locks: RefCell<Vec<String>>,
    writes: RefCell<BTreeMap<String, String>>,
    /// Every input file of the case, by sandbox-relative path.
    sandbox: BTreeMap<String, String>,
    stdin: String,
    dirs: RefCell<Vec<String>>,
    /// Write order, as modification times later than every input file.
    clock: Cell<u64>,
    mtimes: RefCell<BTreeMap<String, u64>>,
}

/// What a recorded case expects.
pub struct Expected {
    pub outcome: Outcome,
    /// `factory/jobs/<id>/job.json` files the case created or changed.
    pub job_files: BTreeMap<String, String>,
    /// Ids whose `.lock` the case created.
    pub lock_ids: Vec<String>,
}

fn other(what: &'static str) -> io::Error {
    io::Error::other(what)
}

impl Fake {
    pub fn new() -> Self {
        Fake {
            config: Some(r#"{"labels": {}, "base_branch": "main", "concurrency": 3, "prices_usd_per_mtok": {}}"#.into()),
            ..Fake::default()
        }
    }

    pub fn config(mut self, text: &str) -> Self {
        self.config = Some(text.into());
        self
    }

    pub fn config_error(mut self, kind: io::ErrorKind) -> Self {
        self.config_error = Some(kind);
        self
    }

    pub fn job(mut self, id: &str, json: &str) -> Self {
        self.files.insert(format!("{id}/job.json"), json.into());
        self
    }

    pub fn fail_writes(mut self) -> Self {
        self.fail.push("write");
        self
    }

    pub fn fail_locks(mut self) -> Self {
        self.fail.push("lock");
        self
    }

    pub fn fail_transcripts(mut self) -> Self {
        self.fail.push("transcripts");
        self
    }

    pub fn fail_job_list(mut self) -> Self {
        self.fail.push("list");
        self
    }

    pub fn fail_run(mut self) -> Self {
        self.fail.push("run");
        self
    }

    pub fn fail_reads(mut self) -> Self {
        self.fail.push("read");
        self
    }

    pub fn stdin(mut self, text: &str) -> Self {
        self.stdin = text.into();
        self
    }

    pub fn file(mut self, path: &str, text: &str) -> Self {
        self.files.insert(path.into(), text.into());
        self
    }

    pub fn sandbox_file(mut self, path: &str, text: &str) -> Self {
        self.sandbox.insert(path.into(), text.into());
        self
    }

    pub fn dirs(&self) -> Vec<String> {
        self.dirs.borrow().clone()
    }

    pub fn written(&self, path: &str) -> Option<String> {
        self.writes.borrow().get(path).cloned()
    }

    /// Builds a fake from a recorded case's input files (`<case>.in/`).
    pub fn from_inputs(inputs: &[(&str, &str)]) -> Self {
        let mut fake = Fake::default();
        for (path, text) in inputs {
            fake.sandbox.insert(path.to_string(), text.to_string());
            if *path == "factory/config.json" {
                fake.config = Some(text.to_string());
            } else if let Some(rel) = path.strip_prefix("factory/jobs/") {
                fake.files.insert(rel.into(), text.to_string());
            } else if is_transcript(path) {
                fake.transcripts.push(text.replace("@REPO@", REPO));
            }
        }
        fake
    }

    pub fn locks(&self) -> Vec<String> {
        self.locks.borrow().clone()
    }

    /// The written files and `.lock` files match the recorded case. A `[..]`
    /// in a recorded line (rust-anything's normalizer) matches any text there.
    pub fn assert_files(&self, want: &Expected) {
        let mut writes = self.writes.borrow().clone();
        // A file written back byte-identical is not a change on disk.
        writes.retain(|path, text| self.files.get(path) != Some(text));
        let paths: Vec<&String> = writes.keys().collect();
        let want_paths: Vec<&String> = want.job_files.keys().collect();
        assert_eq!(paths, want_paths, "written files");
        for (path, text) in &writes {
            assert!(
                matches_recorded(text, &want.job_files[path]),
                "{path}:\n{text}\n---\n{}",
                want.job_files[path]
            );
        }
        let mut locks = self.locks();
        locks.sort();
        assert_eq!(locks, want.lock_ids, ".lock files");
    }

    fn check(&self, what: &'static str) -> io::Result<()> {
        if self.fail.contains(&what) {
            Err(other(what))
        } else {
            Ok(())
        }
    }
}

/// trycmd semantics for one file: line by line, `[..]` matches any text.
pub fn matches_recorded(actual: &str, recorded: &str) -> bool {
    let actual: Vec<&str> = actual.split('\n').collect();
    let recorded: Vec<&str> = recorded.split('\n').collect();
    actual.len() == recorded.len()
        && actual
            .iter()
            .zip(&recorded)
            .all(|(a, r)| match r.split_once("[..]") {
                None => a == r,
                Some((head, tail)) => {
                    a.len() >= head.len() + tail.len() && a.starts_with(head) && a.ends_with(tail)
                }
            })
}

/// What rust-anything's sandbox glob `*/subagents/agent-*.jsonl` matches
/// below `claude-config/projects/REPO/`; the real glob is tested in disk.rs.
fn is_transcript(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("claude-config/projects/REPO/") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    matches!(parts.as_slice(), [_, "subagents", file] if file.starts_with("agent-") && file.ends_with(".jsonl"))
}

impl Workspace for Fake {
    fn read_config(&self) -> io::Result<String> {
        if let Some(kind) = self.config_error {
            return Err(io::Error::new(kind, "config"));
        }
        self.config
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config"))
    }

    fn read(&self, path: &str) -> io::Result<Option<String>> {
        self.check("read")?;
        let written = self.writes.borrow().get(path).cloned();
        Ok(written.or_else(|| self.files.get(path).cloned()))
    }

    fn has_job(&self, id: &str) -> bool {
        self.files.contains_key(&format!("{id}/job.json"))
    }

    fn job_ids(&self) -> io::Result<Vec<String>> {
        self.check("list")?;
        Ok(self
            .files
            .keys()
            .filter_map(|p| p.strip_suffix("/job.json"))
            .map(str::to_string)
            .collect())
    }

    fn lock(&self, id: &str) -> io::Result<()> {
        self.check("lock")?;
        let mut locks = self.locks.borrow_mut();
        if !locks.iter().any(|l| l == id) {
            locks.push(id.to_string());
        }
        Ok(())
    }

    fn write_job(&self, id: &str, text: &str) -> io::Result<()> {
        self.check("write")?;
        self.writes
            .borrow_mut()
            .insert(format!("{id}/job.json"), text.to_string());
        Ok(())
    }

    fn job_folder(&self, id: &str) -> String {
        format!("{REPO}/factory/jobs/{id}")
    }

    fn transcripts(&self) -> io::Result<Vec<String>> {
        self.check("transcripts")?;
        Ok(self.transcripts.clone())
    }

    fn stdin(&self) -> io::Result<String> {
        Ok(self.stdin.clone())
    }

    fn write(&self, path: &str, text: &str) -> io::Result<()> {
        self.check("write")?;
        self.clock.set(self.clock.get() + 1);
        self.mtimes
            .borrow_mut()
            .insert(path.into(), self.clock.get());
        self.writes.borrow_mut().insert(path.into(), text.into());
        Ok(())
    }

    fn list(&self, dir: &str) -> io::Result<Vec<String>> {
        let prefix = format!("{dir}/");
        let writes = self.writes.borrow();
        let mut names: Vec<String> = self
            .files
            .keys()
            .chain(writes.keys())
            .filter_map(|p| p.strip_prefix(&prefix))
            .filter(|rest| !rest.contains('/'))
            .map(str::to_string)
            .collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    fn mtime(&self, path: &str) -> io::Result<SystemTime> {
        let tick = match self.mtimes.borrow().get(path) {
            Some(tick) => *tick,
            None if self.files.contains_key(path) => 0,
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "missing")),
        };
        Ok(SystemTime::UNIX_EPOCH + Duration::from_nanos(tick))
    }

    fn mkdir(&self, path: &str) -> io::Result<()> {
        self.dirs.borrow_mut().push(path.into());
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        let prefix = format!("{path}/");
        self.sandbox
            .keys()
            .any(|p| p == path || p.starts_with(&prefix))
    }

    fn run(&self, program: &str, args: &[&str]) -> io::Result<Proc> {
        if self.fail.contains(&"run") {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no such program"));
        }
        let mut log = self
            .writes
            .borrow()
            .get("doubles.log")
            .cloned()
            .unwrap_or_default();
        log.push_str(&format!("$ {program} (cwd .)\n"));
        for a in args {
            log.push_str(&format!("  [{a}]\n"));
        }
        self.writes.borrow_mut().insert("doubles.log".into(), log);
        let key: String = format!(
            "{}_{}",
            args.first().unwrap_or(&""),
            args.get(1).unwrap_or(&"")
        )
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
        let answer = |suffix: &str| {
            self.sandbox
                .get(&format!("doubles/{program}/{key}.{suffix}"))
                .cloned()
        };
        Ok(Proc {
            code: answer("code")
                .and_then(|c| c.trim().parse().ok())
                .unwrap_or(0),
            stdout: answer("stdout").unwrap_or_default(),
            stderr: answer("stderr").unwrap_or_default(),
        })
    }

    fn now(&self) -> i64 {
        NOW
    }
}

/// Reads the recorded files of one case into `Expected`.
pub fn expected(
    code: Option<&str>,
    stdout: &str,
    stderr: &str,
    outputs: &[(&str, &str)],
) -> Expected {
    let mut job_files = BTreeMap::new();
    let mut lock_ids = Vec::new();
    for (path, text) in outputs {
        let rel = path.strip_prefix("factory/jobs/").unwrap_or(path);
        if let Some(id) = rel.strip_suffix("/.lock") {
            lock_ids.push(id.to_string());
        } else {
            job_files.insert(rel.to_string(), text.to_string());
        }
    }
    lock_ids.sort();
    Expected {
        outcome: Outcome {
            code: code.map_or(0, |c| c.parse().unwrap_or(-1)),
            stdout: stdout.into(),
            stderr: stderr.into(),
        },
        job_files,
        lock_ids,
    }
}

/// `case!("sync-01-fresh-job")` → `(Fake, Expected)` from
/// tests/parity/job/rust/app-cases; `case!("job-reject", "reject-01-...")`
/// from another harness. Read at test time.
macro_rules! case {
    ($name:literal) => {
        $crate::job::fake::case!("job", $name)
    };
    ($harness:literal, $name:literal) => {{
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/parity/",
            $harness,
            "/rust/app-cases/",
            $name
        );
        let files = |suffix: &str| $crate::testing::read_tree(&format!("{dir}{suffix}"));
        let inputs = files(".in");
        let outputs = files(".out");
        let toml = std::fs::read_to_string(format!("{dir}.toml")).unwrap();
        let code = toml
            .lines()
            .find_map(|l| l.strip_prefix("status.code = "))
            .map(str::to_string);
        let read = |suffix: &str| std::fs::read_to_string(format!("{dir}{suffix}")).unwrap();
        let fake = $crate::job::fake::Fake::from_inputs(
            &inputs
                .iter()
                .map(|(p, t)| (p.as_str(), t.as_str()))
                .collect::<Vec<_>>(),
        )
        .stdin(&std::fs::read_to_string(format!("{dir}.stdin")).unwrap_or_default());
        let want = $crate::job::fake::expected(
            code.as_deref(),
            &read(".stdout"),
            &read(".stderr"),
            &outputs
                .iter()
                .map(|(p, t)| (p.as_str(), t.as_str()))
                .collect::<Vec<_>>(),
        );
        (fake, want)
    }};
}
pub(crate) use case;
