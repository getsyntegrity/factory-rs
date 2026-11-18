//! The real filesystem side of [`Workspace`], and the `job` executable's
//! dispatch between the Rust commands and the v2 fallback.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use super::{Command, Workspace, load_config, run};
use crate::Outcome;

/// What the `job` executable does with its argv.
#[derive(Debug, PartialEq, Eq)]
pub enum Dispatch {
    /// Answered in Rust.
    Done(Outcome),
    /// FALLBACK (pending later slices): hand the argv to v2's Python job,
    /// installed beside this binary as `factory/bin/job.py`.
    Fallback,
}

/// Decides and, for owned commands, runs.
pub fn dispatch(args: &[String], ws: &dyn Workspace) -> Dispatch {
    let config = match load_config(ws) {
        Ok(config) => config,
        Err(msg) => return Dispatch::Done(Outcome::die("job", &msg)),
    };
    match Command::parse(args) {
        Some(command) => Dispatch::Done(run(&command, &config, ws)),
        None => Dispatch::Fallback,
    }
}

/// A v2 checkout: the repository holding `factory/`.
pub struct Disk {
    repo: PathBuf,
    claude_config: Option<PathBuf>,
    locks: RefCell<HashMap<String, File>>,
}

impl Disk {
    /// v2 `REPO = Path(__file__).resolve().parents[2]`, and its Claude Code
    /// config directory: `$CLAUDE_CONFIG_DIR`, else `~/.claude`.
    pub fn new(exe: &Path, claude_config_dir: Option<PathBuf>, home: Option<PathBuf>) -> Self {
        let repo = exe
            .ancestors()
            .nth(3)
            .unwrap_or(Path::new("/"))
            .to_path_buf();
        Disk {
            repo,
            claude_config: claude_config_dir.or_else(|| home.map(|h| h.join(".claude"))),
            locks: RefCell::new(HashMap::new()),
        }
    }

    /// v2's Python job, run for every argv the Rust job does not own.
    pub fn fallback(&self) -> PathBuf {
        self.repo.join("factory/bin/job.py")
    }

    fn jobs(&self) -> PathBuf {
        self.repo.join("factory/jobs")
    }
}

fn not_found_as_none<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// `dir`'s entries in directory order (Python's `scandir` order); none when
/// `dir` does not exist or is not a directory, as a glob step skips it.
fn entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let list = match fs::read_dir(dir) {
        Ok(list) => list,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(Vec::new());
        }
        Err(e) => return Err(e),
    };
    list.map(|e| e.map(|e| e.path())).collect()
}

impl Workspace for Disk {
    fn read_config(&self) -> io::Result<String> {
        fs::read_to_string(self.repo.join("factory/config.json"))
    }

    fn read(&self, path: &str) -> io::Result<Option<String>> {
        not_found_as_none(fs::read_to_string(self.jobs().join(path)))
    }

    fn has_job(&self, id: &str) -> bool {
        self.jobs().join(id).join("job.json").exists()
    }

    fn job_ids(&self) -> io::Result<Vec<String>> {
        let mut ids: Vec<String> = entries(&self.jobs())?
            .into_iter()
            .filter(|p| p.join("job.json").exists())
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        ids.sort();
        Ok(ids)
    }

    fn lock(&self, id: &str) -> io::Result<()> {
        if self.locks.borrow().contains_key(id) {
            return Ok(());
        }
        let file = File::create(self.jobs().join(id).join(".lock"))?;
        file.lock()?;
        self.locks.borrow_mut().insert(id.to_string(), file);
        Ok(())
    }

    fn write_job(&self, id: &str, text: &str) -> io::Result<()> {
        let dir = self.jobs().join(id);
        fs::write(dir.join("job.tmp"), text)?;
        fs::rename(dir.join("job.tmp"), dir.join("job.json"))
    }

    fn job_folder(&self, id: &str) -> String {
        self.jobs().join(id).to_string_lossy().into_owned()
    }

    /// v2 globs `<config>/projects/<repo>/*/subagents/agent-*.jsonl`, where
    /// `<repo>` is the repository path with every character outside
    /// `[A-Za-z0-9]` replaced by `-`. pathlib's `*` matches hidden names too.
    fn transcripts(&self) -> io::Result<Vec<String>> {
        let Some(config) = &self.claude_config else {
            return Ok(Vec::new());
        };
        let mangled: String = self
            .repo
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let mut texts = Vec::new();
        for session in entries(&config.join("projects").join(mangled))? {
            for path in entries(&session.join("subagents"))? {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if name.starts_with("agent-") && name.ends_with(".jsonl") && name.len() >= 12 {
                    texts.push(fs::read_to_string(&path)?);
                }
            }
        }
        Ok(texts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Repo {
        dir: tempfile::TempDir,
    }

    impl Repo {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("factory/bin")).unwrap();
            Repo { dir }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn write(&self, rel: &str, text: &str) -> &Self {
            let path = self.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
            self
        }

        fn disk(&self) -> Disk {
            Disk::new(
                &self.path().join("factory/bin/job"),
                Some(self.path().join("claude")),
                None,
            )
        }

        /// Where job looks for this repository's transcripts.
        fn transcripts_dir(&self) -> String {
            let mangled: String = self
                .path()
                .to_string_lossy()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect();
            format!("claude/projects/{mangled}")
        }
    }

    #[test]
    fn repo_is_two_levels_above_the_executables_bin() {
        let disk = Disk::new(
            Path::new("/srv/app/factory/bin/job"),
            None,
            Some("/home/u".into()),
        );
        assert_eq!(disk.repo, PathBuf::from("/srv/app"));
        assert_eq!(
            disk.fallback(),
            PathBuf::from("/srv/app/factory/bin/job.py")
        );
        assert_eq!(disk.claude_config, Some(PathBuf::from("/home/u/.claude")));
        assert_eq!(disk.job_folder("001-a"), "/srv/app/factory/jobs/001-a");
    }

    #[test]
    fn claude_config_dir_wins_over_home() {
        let disk = Disk::new(
            Path::new("/r/factory/bin/job"),
            Some("/cfg".into()),
            Some("/home/u".into()),
        );
        assert_eq!(disk.claude_config, Some(PathBuf::from("/cfg")));
        let none = Disk::new(Path::new("/r/factory/bin/job"), None, None);
        assert_eq!(none.claude_config, None);
    }

    #[test]
    fn reads_config_and_job_files() {
        let repo = Repo::new();
        repo.write("factory/config.json", "{}")
            .write("factory/jobs/001-a/intake.md", "RISK: low\n");
        let disk = repo.disk();
        assert_eq!(disk.read_config().unwrap(), "{}");
        assert_eq!(
            disk.read("001-a/intake.md").unwrap().as_deref(),
            Some("RISK: low\n")
        );
        assert_eq!(disk.read("001-a/missing.md").unwrap(), None);
        assert!(disk.read("001-a").is_err(), "a directory is not a file");
    }

    #[test]
    fn job_ids_are_sorted_folders_with_a_job_json() {
        let repo = Repo::new();
        assert_eq!(repo.disk().job_ids().unwrap(), Vec::<String>::new());
        repo.write("factory/jobs/b/job.json", "{}")
            .write("factory/jobs/a/job.json", "{}")
            .write("factory/jobs/c/intake.md", "")
            .write("factory/jobs/notes.txt", "");
        let disk = repo.disk();
        assert_eq!(disk.job_ids().unwrap(), ["a", "b"]);
        assert!(disk.has_job("a"));
        assert!(!disk.has_job("c"));
        assert!(!disk.has_job("bad id/../a"));
    }

    #[test]
    fn lock_creates_the_lock_file_once_and_holds_it() {
        let repo = Repo::new();
        repo.write("factory/jobs/a/job.json", "{}")
            .write("factory/jobs/a/.lock", "stale");
        let disk = repo.disk();
        disk.lock("a").unwrap();
        disk.lock("a").unwrap();
        assert_eq!(
            fs::read_to_string(repo.path().join("factory/jobs/a/.lock")).unwrap(),
            ""
        );
        let other = File::open(repo.path().join("factory/jobs/a/.lock")).unwrap();
        assert!(other.try_lock().is_err(), "held by the first disk");
        drop(disk);
        assert!(other.try_lock().is_ok(), "released with the disk");
    }

    #[test]
    fn write_job_replaces_job_json_through_job_tmp() {
        let repo = Repo::new();
        repo.write("factory/jobs/a/job.json", "old");
        repo.disk().write_job("a", "new\n").unwrap();
        assert_eq!(
            fs::read_to_string(repo.path().join("factory/jobs/a/job.json")).unwrap(),
            "new\n"
        );
        assert!(!repo.path().join("factory/jobs/a/job.tmp").exists());
    }

    #[test]
    fn transcripts_follow_v2_glob_including_hidden_sessions() {
        let repo = Repo::new();
        let t = repo.transcripts_dir();
        repo.write(&format!("{t}/s1/subagents/agent-1.jsonl"), "one")
            .write(&format!("{t}/.hidden/subagents/agent-h.jsonl"), "hidden")
            .write(&format!("{t}/s2/subagents/agent-.jsonl"), "bare")
            .write(&format!("{t}/s2/subagents/other.jsonl"), "no")
            .write(&format!("{t}/s2/subagents/agent-x.json"), "no")
            .write(&format!("{t}/s3/other/agent-x.jsonl"), "no")
            .write(&format!("{t}/agent-top.jsonl"), "no")
            .write("claude/projects/elsewhere/s1/subagents/agent-1.jsonl", "no");
        let mut found = repo.disk().transcripts().unwrap();
        found.sort();
        assert_eq!(found, ["bare", "hidden", "one"]);
    }

    #[test]
    fn no_transcripts_without_a_project_folder_or_config_dir() {
        let repo = Repo::new();
        assert!(repo.disk().transcripts().unwrap().is_empty());
        let disk = Disk::new(&repo.path().join("factory/bin/job"), None, None);
        assert!(disk.transcripts().unwrap().is_empty());
    }

    #[test]
    fn dispatch_checks_config_before_the_arguments() {
        let repo = Repo::new();
        let disk = repo.disk();
        assert_eq!(
            dispatch(&["--help".into()], &disk),
            Dispatch::Done(Outcome::die("job", "factory/config.json is missing"))
        );
    }

    #[test]
    fn dispatch_hands_unowned_argv_to_the_fallback() {
        let repo = Repo::new();
        repo.write(
            "factory/config.json",
            r#"{"labels": {}, "base_branch": "main", "concurrency": 3}"#,
        );
        assert_eq!(
            dispatch(&["summary".into()], &repo.disk()),
            Dispatch::Fallback
        );
    }

    #[test]
    fn dispatch_runs_owned_commands() {
        let repo = Repo::new();
        repo.write(
            "factory/config.json",
            r#"{"labels": {}, "base_branch": "main", "concurrency": 3}"#,
        );
        assert_eq!(
            dispatch(&["sync".into(), "x".into()], &repo.disk()),
            Dispatch::Done(Outcome::die("job", "no job x"))
        );
    }
}
