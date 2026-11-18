//! In-memory [`Host`] for unit tests, plus `case!` to load a v2 case recorded
//! by `rust-anything app record` under tests/parity/gate.
//!
//! `git` answers like the PATH double in tests/parity/gate/launch.sh, from the
//! case's `git-double/` files. `checks.sh` is "run" by reading the fixture
//! that migration/rust-anything/gate/make_inputs.py generates: two
//! `printf '%s' $'...'` lines (stdout, then stderr) and `exit N`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

use super::{Host, Proc};
use crate::Outcome;

pub struct Fake {
    /// Case input files by sandbox-relative path.
    pub files: BTreeMap<String, String>,
    stdin: String,
    /// Host calls that fail with an I/O error: "config", "git", "checks".
    pub fail: Vec<&'static str>,
    writes: RefCell<BTreeMap<String, String>>,
}

/// What a recorded case expects.
pub struct Expected {
    pub outcome: Outcome,
    /// Files below `factory/` the case created or changed, by path relative
    /// to `factory/`.
    pub writes: BTreeMap<String, String>,
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "missing")
}

/// Undoes the bash `$'...'` quoting of make_inputs.py's `bash_text`.
fn ansi_c(quoted: &str) -> String {
    let inner = quoted
        .strip_prefix("$'")
        .and_then(|q| q.strip_suffix('\''))
        .unwrap_or(quoted);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

impl Fake {
    pub fn new(files: BTreeMap<String, String>, stdin: String) -> Self {
        Fake {
            files,
            stdin,
            fail: Vec::new(),
            writes: RefCell::new(BTreeMap::new()),
        }
    }

    fn file(&self, path: &str) -> io::Result<String> {
        self.files.get(path).cloned().ok_or_else(not_found)
    }

    pub fn assert_writes(&self, want: &Expected) {
        let mut writes = self.writes.borrow().clone();
        writes.retain(|path, text| self.files.get(&format!("factory/{path}")) != Some(text));
        assert_eq!(writes, want.writes, "files written below factory/");
    }
}

impl Host for Fake {
    fn read_config(&self) -> io::Result<String> {
        if self.fail.contains(&"config") {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        }
        self.file("factory/config.json")
    }

    fn read(&self, path: &str) -> io::Result<String> {
        self.file(&format!("factory/{path}"))
    }

    fn write(&self, path: &str, text: &str) -> io::Result<()> {
        let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let prefix = format!("factory/{dir}/");
        if !self.files.keys().any(|k| k.starts_with(&prefix)) {
            return Err(not_found());
        }
        self.writes
            .borrow_mut()
            .insert(path.to_string(), text.to_string());
        Ok(())
    }

    fn git(&self, worktree: Option<&str>, args: &[&str]) -> io::Result<Proc> {
        if self.fail.contains(&"git") {
            return Err(not_found());
        }
        let cmd = args.first().copied().unwrap_or_default();
        let answer = |suffix: &str| {
            self.files
                .get(&format!("git-double/{cmd}.{suffix}"))
                .cloned()
        };
        let Some(want) = answer("args") else {
            return Ok(Proc {
                code: 98,
                stdout: String::new(),
                stderr: format!("git double: no answer for: {}\n", args.join(" ")),
            });
        };
        let got: String = args.iter().map(|a| format!("{a}\n")).collect();
        let cwd = self
            .files
            .get("git-double/cwd")
            .map(|c| c.trim_end().to_string());
        if got != want || worktree.map(str::to_string) != cwd {
            return Ok(Proc {
                code: 99,
                stdout: String::new(),
                stderr: "git double: unexpected call\n".into(),
            });
        }
        Ok(Proc {
            code: answer("code")
                .and_then(|c| c.trim().parse().ok())
                .unwrap_or(0),
            stdout: answer("stdout").unwrap_or_default(),
            stderr: answer("stderr").unwrap_or_default(),
        })
    }

    fn checks(&self, _worktree: Option<&str>) -> io::Result<(Proc, u64)> {
        if self.fail.contains(&"checks") {
            return Err(not_found());
        }
        let Some(script) = self.files.get("factory/checks.sh") else {
            let stderr = "bash: [CWD]/factory/checks.sh: No such file or directory\n".to_string();
            return Ok((
                Proc {
                    code: 127,
                    stdout: String::new(),
                    stderr,
                },
                0,
            ));
        };
        let mut proc = Proc {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        for line in script.lines() {
            if let Some(rest) = line.strip_prefix("printf '%s' ") {
                match rest.strip_suffix(" >&2") {
                    Some(quoted) => proc.stderr = ansi_c(quoted),
                    None => proc.stdout = ansi_c(rest),
                }
            } else if let Some(code) = line.strip_prefix("exit ") {
                proc.code = code.parse().unwrap_or(0);
            }
        }
        Ok((proc, 0))
    }

    fn stdin(&self) -> io::Result<String> {
        Ok(self.stdin.clone())
    }

    fn read_user_file(&self, path: &str) -> io::Result<String> {
        self.file(path)
    }
}

/// `case!("gate-01-pass")` → `(Fake, Expected)` from
/// tests/parity/gate/rust/app-cases, read at test time.
macro_rules! case {
    ($name:literal) => {{
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/parity/gate/rust/app-cases/",
            $name
        );
        let tree = |suffix: &str| -> std::collections::BTreeMap<String, String> {
            $crate::testing::read_tree(&format!("{dir}{suffix}"))
                .into_iter()
                .collect()
        };
        let read = |suffix: &str| std::fs::read_to_string(format!("{dir}{suffix}")).unwrap();
        let toml = read(".toml");
        let code = toml
            .lines()
            .find_map(|l| l.strip_prefix("status.code = "))
            .map_or(0, |c| c.parse().unwrap());
        let writes = tree(".out")
            .into_iter()
            .filter_map(|(p, t)| p.strip_prefix("factory/").map(|p| (p.to_string(), t)))
            .collect();
        let want = $crate::gate::fake::Expected {
            outcome: $crate::Outcome {
                code,
                stdout: read(".stdout"),
                stderr: read(".stderr"),
            },
            writes,
        };
        let stdin = std::fs::read_to_string(format!("{dir}.stdin")).unwrap_or_default();
        ($crate::gate::fake::Fake::new(tree(".in"), stdin), want)
    }};
}
pub(crate) use case;
