//! The real filesystem and process side of [`Host`].

use std::fs;
use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

use super::{Host, Proc};

/// A v2 installation, found from the running executable like v2's
/// `FACTORY = Path(__file__).resolve().parents[1]`.
pub struct Disk {
    factory: PathBuf,
    home: Option<PathBuf>,
    git: PathBuf,
}

impl Disk {
    pub fn new(exe: &Path, home: Option<PathBuf>) -> Self {
        let factory = exe
            .ancestors()
            .nth(2)
            .unwrap_or(Path::new("/"))
            .to_path_buf();
        Disk {
            factory,
            home,
            git: PathBuf::from("git"),
        }
    }
}

/// Python's `returncode`: the exit code, or minus the signal number.
fn proc(out: Output) -> Proc {
    Proc {
        code: out
            .status
            .code()
            .or_else(|| out.status.signal().map(|s| -s))
            .unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn in_dir(mut command: Command, worktree: Option<&str>) -> io::Result<Output> {
    if let Some(dir) = worktree {
        command.current_dir(dir);
    }
    command.output()
}

impl Host for Disk {
    fn read_config(&self) -> io::Result<String> {
        fs::read_to_string(self.factory.join("config.json"))
    }

    fn read(&self, path: &str) -> io::Result<String> {
        fs::read_to_string(self.factory.join(path))
    }

    fn write(&self, path: &str, text: &str) -> io::Result<()> {
        fs::write(self.factory.join(path), text)
    }

    fn git(&self, worktree: Option<&str>, args: &[&str]) -> io::Result<Proc> {
        let mut command = Command::new(&self.git);
        command.args(args);
        in_dir(command, worktree).map(proc)
    }

    fn checks(&self, worktree: Option<&str>) -> io::Result<(Proc, u64)> {
        let start = Instant::now();
        let mut command = Command::new("bash");
        command.arg(self.factory.join("checks.sh"));
        let out = in_dir(command, worktree)?;
        Ok((proc(out), start.elapsed().as_secs()))
    }

    fn stdin(&self) -> io::Result<String> {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        Ok(text)
    }

    /// `Path(path).expanduser()` for `~` and `~/...`.
    fn read_user_file(&self, path: &str) -> io::Result<String> {
        let expanded = match (path.strip_prefix('~'), &self.home) {
            (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
                home.join(rest.trim_start_matches('/'))
            }
            _ => PathBuf::from(path),
        };
        fs::read_to_string(expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Install {
        dir: tempfile::TempDir,
        disk: Disk,
    }

    fn install() -> Install {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("factory/bin")).unwrap();
        fs::create_dir_all(dir.path().join("wt")).unwrap();
        let disk = Disk::new(
            &dir.path().join("factory/bin/gate"),
            Some(dir.path().join("home")),
        );
        Install { dir, disk }
    }

    #[test]
    fn factory_is_the_parent_of_the_executables_bin() {
        let disk = Disk::new(Path::new("/srv/app/factory/bin/gate"), None);
        assert_eq!(disk.factory, PathBuf::from("/srv/app/factory"));
    }

    #[test]
    fn reads_and_writes_below_factory() {
        let i = install();
        fs::write(i.dir.path().join("factory/config.json"), "{}").unwrap();
        assert_eq!(i.disk.read_config().unwrap(), "{}");
        i.disk.write("note.md", "hi\n").unwrap();
        assert_eq!(i.disk.read("note.md").unwrap(), "hi\n");
        assert!(
            i.disk.write("missing/dir/x.md", "").is_err(),
            "no folder is created"
        );
    }

    #[test]
    fn git_runs_in_the_worktree_and_reports_signals_as_negative_codes() {
        let _serial = crate::testing::process_lock();
        let mut i = install();
        let git = i.dir.path().join("git");
        fs::write(
            &git,
            "#!/bin/sh\necho \"$PWD|$*\"\necho err >&2\n[ \"$1\" = die ] && kill -9 $$\nexit 3\n",
        )
        .unwrap();
        fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
        i.disk.git = git;
        let wt = i.dir.path().join("wt");
        let out = i
            .disk
            .git(Some(wt.to_str().unwrap()), &["diff", "a b"])
            .unwrap();
        let cwd = fs::canonicalize(&wt).unwrap();
        assert_eq!(out.code, 3);
        assert_eq!(out.stdout, format!("{}|diff a b\n", cwd.display()));
        assert_eq!(out.stderr, "err\n");
        assert_eq!(i.disk.git(None, &["die"]).unwrap().code, -9);
    }

    #[test]
    fn git_that_cannot_start_is_an_error() {
        let _serial = crate::testing::process_lock();
        let mut i = install();
        i.disk.git = i.dir.path().join("no-such-git");
        assert!(i.disk.git(None, &["status"]).is_err());
    }

    #[test]
    fn checks_run_with_bash_in_the_worktree() {
        let _serial = crate::testing::process_lock();
        let i = install();
        fs::write(
            i.dir.path().join("factory/checks.sh"),
            "basename \"$PWD\"\necho oops >&2\nexit 4\n",
        )
        .unwrap();
        let (out, seconds) = i
            .disk
            .checks(Some(i.dir.path().join("wt").to_str().unwrap()))
            .unwrap();
        assert_eq!(
            (out.code, out.stdout.as_str(), out.stderr.as_str(), seconds),
            (4, "wt\n", "oops\n", 0)
        );
    }

    #[test]
    fn user_files_expand_the_home_directory() {
        let i = install();
        fs::create_dir_all(i.dir.path().join("home/t")).unwrap();
        fs::write(i.dir.path().join("home/t/a.jsonl"), "home").unwrap();
        fs::write(i.dir.path().join("plain.jsonl"), "plain").unwrap();
        assert_eq!(i.disk.read_user_file("~/t/a.jsonl").unwrap(), "home");
        let plain = i.dir.path().join("plain.jsonl");
        assert_eq!(
            i.disk.read_user_file(plain.to_str().unwrap()).unwrap(),
            "plain"
        );
        assert!(
            i.disk.read_user_file("~other/a.jsonl").is_err(),
            "~user is not expanded"
        );
    }
}
