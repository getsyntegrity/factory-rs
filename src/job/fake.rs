//! In-memory [`Workspace`] for unit tests, plus `case!` to load a v2 case
//! recorded by `rust-anything app record` under tests/parity/job.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

use super::Workspace;
use crate::Outcome;

/// The absolute repository path the fake pretends to live at.
pub const REPO: &str = "/repo";

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

    pub fn fail_reads(mut self) -> Self {
        self.fail.push("read");
        self
    }

    /// Builds a fake from a recorded case's input files (`<case>.in/`).
    pub fn from_inputs(inputs: &[(&str, &str)]) -> Self {
        let mut fake = Fake::default();
        for (path, text) in inputs {
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

    /// The job.json writes and `.lock` files match the recorded case.
    pub fn assert_files(&self, want: &Expected) {
        let mut writes = self.writes.borrow().clone();
        // A job.json written back byte-identical is not a change on disk.
        writes.retain(|path, text| self.files.get(path) != Some(text));
        assert_eq!(writes, want.job_files, "job.json files");
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
        Ok(self.files.get(path).cloned())
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
/// tests/parity/job/rust/app-cases, read at test time.
macro_rules! case {
    ($name:literal) => {{
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/parity/job/rust/app-cases/",
            $name
        );
        let files = |suffix: &str| $crate::job::fake::read_tree(&format!("{dir}{suffix}"));
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
        );
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

/// Every file below `dir` as (relative path, text); empty when `dir` is absent.
pub fn read_tree(dir: &str) -> Vec<(String, String)> {
    fn walk(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let base = std::path::Path::new(dir);
    let mut out = Vec::new();
    walk(base, base, &mut out);
    out.sort();
    out
}
