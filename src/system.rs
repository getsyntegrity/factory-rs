//! The real filesystem and process side of [`Factory`].

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::route::{self, Factory, JobOutput, Outcome};

/// The `route` executable: its arguments (without the program name) and its
/// own path, as `std::env` reports them.
pub fn route_main(args: impl IntoIterator<Item = OsString>, exe: io::Result<PathBuf>) -> Outcome {
    let args: Vec<String> = args
        .into_iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    match exe {
        Ok(exe) => route::run(&args, &Installation::from_executable(&exe)),
        Err(e) => Outcome {
            code: 1,
            stdout: String::new(),
            stderr: format!("route: {e}\n"),
        },
    }
}

/// A v2 installation: the `factory/` directory that holds `bin/`, `config.json`
/// and `jobs/`.
pub struct Installation {
    factory: PathBuf,
}

impl Installation {
    /// v2 locates `factory/` as the parent of the running executable's `bin/`
    /// directory (`Path(__file__).resolve().parents[1]`).
    pub fn from_executable(exe: &Path) -> Self {
        let factory = exe
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("/"));
        Installation {
            factory: factory.to_path_buf(),
        }
    }
}

impl Factory for Installation {
    fn read(&self, path: &str) -> io::Result<String> {
        fs::read_to_string(self.factory.join(path))
    }

    /// FALLBACK (pending `job` slice): runs the sibling `factory/bin/job`, which
    /// is still v2's Python original.
    fn job(&self, args: &[&str]) -> io::Result<JobOutput> {
        let out = Command::new(self.factory.join("bin/job"))
            .args(args)
            .output()?;
        Ok(JobOutput {
            success: out.status.success(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::MutexGuard;

    /// A temporary install. Tests that write or run executables hold
    /// [`crate::testing::process_lock`].
    struct Fixture {
        dir: tempfile::TempDir,
        _serial: MutexGuard<'static, ()>,
    }

    impl Fixture {
        fn empty() -> Self {
            let _serial = crate::testing::process_lock();
            Fixture {
                dir: tempfile::tempdir().unwrap(),
                _serial,
            }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }
    }

    fn factory_with_job(script: &str) -> Fixture {
        let fixture = Fixture::empty();
        let bin = fixture.path().join("factory/bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("job"), script).unwrap();
        fs::set_permissions(bin.join("job"), fs::Permissions::from_mode(0o755)).unwrap();
        fixture
    }

    fn installation(fixture: &Fixture) -> Installation {
        Installation::from_executable(&fixture.path().join("factory/bin/route"))
    }

    #[test]
    fn route_main_runs_route_against_the_executables_installation() {
        let dir = factory_with_job("#!/bin/sh\n");
        fs::write(dir.path().join("factory/config.json"), "{}").unwrap();
        let exe = dir.path().join("factory/bin/route");
        let out = route_main([], Ok(exe));
        assert_eq!(
            out,
            Outcome {
                code: 1,
                stdout: String::new(),
                stderr: "route: usage: route <job-id>\n".into()
            }
        );
    }

    #[test]
    fn route_main_passes_non_utf8_args_lossily() {
        use std::os::unix::ffi::OsStringExt;
        let dir = factory_with_job("#!/bin/sh\necho \"job: no job $2\" >&2\nexit 1\n");
        fs::write(dir.path().join("factory/config.json"), "{}").unwrap();
        let exe = dir.path().join("factory/bin/route");
        let out = route_main([OsString::from_vec(b"a\xffb".to_vec())], Ok(exe));
        assert_eq!(out.code, 1);
        assert!(
            out.stderr.starts_with("route: job: no job a"),
            "{}",
            out.stderr
        );
    }

    #[test]
    fn route_main_reports_an_unknown_executable_path() {
        let out = route_main([], Err(io::Error::other("no exe")));
        assert_eq!(
            out,
            Outcome {
                code: 1,
                stdout: String::new(),
                stderr: "route: no exe\n".into()
            }
        );
    }

    #[test]
    fn factory_is_the_parent_of_the_executables_bin_directory() {
        let inst = Installation::from_executable(Path::new("/srv/app/factory/bin/route"));
        assert_eq!(inst.factory, PathBuf::from("/srv/app/factory"));
    }

    #[test]
    fn read_resolves_paths_inside_factory() {
        let dir = factory_with_job("#!/bin/sh\n");
        fs::write(dir.path().join("factory/config.json"), "{}\n").unwrap();
        assert_eq!(installation(&dir).read("config.json").unwrap(), "{}\n");
    }

    #[test]
    fn read_reports_missing_files() {
        let dir = factory_with_job("#!/bin/sh\n");
        let err = installation(&dir).read("jobs/x/job.json").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn job_passes_args_and_captures_stderr_but_not_stdout() {
        let dir = factory_with_job("#!/bin/sh\necho out\necho \"err $1|$2\" >&2\nexit 0\n");
        let out = installation(&dir).job(&["sync", "a b"]).unwrap();
        assert_eq!(
            out,
            JobOutput {
                success: true,
                stderr: "err sync|a b\n".into()
            }
        );
    }

    #[test]
    fn job_nonzero_exit_is_a_failure() {
        let dir = factory_with_job("#!/bin/sh\necho boom >&2\nexit 4\n");
        let out = installation(&dir).job(&["agents"]).unwrap();
        assert_eq!(
            out,
            JobOutput {
                success: false,
                stderr: "boom\n".into()
            }
        );
    }

    #[test]
    fn job_that_cannot_start_is_an_error() {
        let dir = Fixture::empty();
        let err = installation(&dir).job(&["sync"]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
