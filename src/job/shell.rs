//! v2's command helpers: `run`, `gh`, `operator`, `comment`, `label`. Every
//! command runs in the repository root; a failing checked command becomes
//! v2's `Refused` message.

use std::cell::OnceCell;

use serde_json::Value;

use super::Workspace;
use crate::{Proc, py};

/// Marks the factory's own GitHub comments.
pub const MARKER: &str = "<!-- factory -->";

pub struct Shell<'a> {
    ws: &'a dyn Workspace,
    operator: OnceCell<String>,
}

impl<'a> Shell<'a> {
    pub fn new(ws: &'a dyn Workspace) -> Self {
        Shell {
            ws,
            operator: OnceCell::new(),
        }
    }

    /// v2 `run(cmd, check)`: output read with universal newlines; with
    /// `check`, a non-zero exit is refused as
    /// `` `<cmd>` failed: <stderr, else stdout> ``.
    pub fn run(&self, program: &str, args: &[&str], check: bool) -> Result<Proc, String> {
        let raw = self
            .ws
            .run(program, args)
            .map_err(|_| format!("cannot run {program}"))?;
        let proc = Proc {
            code: raw.code,
            stdout: py::universal_newlines(&raw.stdout),
            stderr: py::universal_newlines(&raw.stderr),
        };
        if check && proc.code != 0 {
            let output = if proc.stderr.is_empty() {
                &proc.stdout
            } else {
                &proc.stderr
            };
            let cmd: Vec<&str> = std::iter::once(program)
                .chain(args.iter().copied())
                .collect();
            return Err(format!("`{}` failed: {}", cmd.join(" "), py::strip(output)));
        }
        Ok(proc)
    }

    /// v2 `gh(*args)`: checked, returns stdout.
    pub fn gh(&self, args: &[&str]) -> Result<String, String> {
        Ok(self.run("gh", args, true)?.stdout)
    }

    /// v2 `gh_json(*args)`: stdout parsed as JSON.
    pub fn gh_json(&self, args: &[&str]) -> Result<Value, String> {
        let stdout = self.gh(args)?;
        serde_json::from_str(&stdout).map_err(|e| {
            format!(
                "`gh {}` did not print JSON (line {}, column {})",
                args.join(" "),
                e.line(),
                e.column()
            )
        })
    }

    /// v2 `operator()`: the GitHub login running the factory, asked once.
    pub fn operator(&self) -> Result<String, String> {
        if let Some(login) = self.operator.get() {
            return Ok(login.clone());
        }
        let login = py::strip(&self.gh(&["api", "user", "--jq", ".login"])?).to_string();
        Ok(self.operator.get_or_init(|| login).clone())
    }

    /// v2 `comment(kind, number, body)`.
    pub fn comment(&self, kind: &str, number: &str, body: &str) -> Result<(), String> {
        let body = format!("{MARKER}\n{body}");
        self.gh(&[kind, "comment", number, "--body", &body])
            .map(drop)
    }

    /// v2 `label(kind, number, add, remove)`; empty labels are left out.
    pub fn label(
        &self,
        kind: &str,
        number: &str,
        add: Option<&str>,
        remove: Option<&str>,
    ) -> Result<(), String> {
        let mut args = vec![kind, "edit", number];
        if let Some(add) = add.filter(|a| !a.is_empty()) {
            args.extend(["--add-label", add]);
        }
        if let Some(remove) = remove.filter(|r| !r.is_empty()) {
            args.extend(["--remove-label", remove]);
        }
        self.gh(&args).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::fake::Fake;

    fn log(fake: &Fake) -> String {
        fake.written("doubles.log").unwrap_or_default()
    }

    #[test]
    fn run_reads_universal_newlines_and_refuses_failures_with_stderr_then_stdout() {
        let fake = Fake::new()
            .sandbox_file("doubles/git/status_x.stdout", "a\r\nb\r")
            .sandbox_file("doubles/git/push_x.code", "1")
            .sandbox_file("doubles/git/push_x.stderr", "  denied\r\n")
            .sandbox_file("doubles/git/fetch_x.code", "2")
            .sandbox_file("doubles/git/fetch_x.stdout", "only stdout\n");
        let shell = Shell::new(&fake);
        assert_eq!(
            shell.run("git", &["status", "x"], true).unwrap().stdout,
            "a\nb\n"
        );
        assert_eq!(
            shell.run("git", &["push", "x"], true),
            Err("`git push x` failed: denied".into())
        );
        assert_eq!(shell.run("git", &["push", "x"], false).unwrap().code, 1);
        assert_eq!(
            shell.run("git", &["fetch", "x"], true),
            Err("`git fetch x` failed: only stdout".into())
        );
    }

    #[test]
    fn a_program_that_cannot_start_is_named() {
        let mut fake = Fake::new();
        fake = fake.fail_run();
        assert_eq!(Shell::new(&fake).gh(&["api"]), Err("cannot run gh".into()));
    }

    #[test]
    fn gh_json_parses_stdout_and_names_bad_json() {
        let fake = Fake::new()
            .sandbox_file("doubles/gh/pr_view.stdout", "{\"state\": \"MERGED\"}\n")
            .sandbox_file("doubles/gh/issue_list.stdout", "nope\n");
        let shell = Shell::new(&fake);
        assert_eq!(
            shell.gh_json(&["pr", "view", "7"]).unwrap()["state"],
            "MERGED"
        );
        assert_eq!(
            shell.gh_json(&["issue", "list"]),
            Err("`gh issue list` did not print JSON (line 1, column 2)".into())
        );
    }

    #[test]
    fn operator_is_asked_once_and_stripped() {
        let fake = Fake::new().sandbox_file("doubles/gh/api_user.stdout", " octo\n");
        let shell = Shell::new(&fake);
        assert_eq!(shell.operator(), Ok("octo".into()));
        assert_eq!(shell.operator(), Ok("octo".into()));
        assert_eq!(
            log(&fake),
            "$ gh (cwd .)\n  [api]\n  [user]\n  [--jq]\n  [.login]\n"
        );
    }

    #[test]
    fn comment_prefixes_the_marker() {
        let fake = Fake::new();
        Shell::new(&fake)
            .comment("issue", "3", "hi\nthere")
            .unwrap();
        assert_eq!(
            log(&fake),
            "$ gh (cwd .)\n  [issue]\n  [comment]\n  [3]\n  [--body]\n  [<!-- factory -->\nhi\nthere]\n"
        );
    }

    #[test]
    fn label_adds_and_removes() {
        let fake = Fake::new();
        let shell = Shell::new(&fake);
        shell.label("issue", "3", Some("a"), Some("b,c")).unwrap();
        shell.label("pr", "4", None, None).unwrap();
        assert_eq!(
            log(&fake),
            "$ gh (cwd .)\n  [issue]\n  [edit]\n  [3]\n  [--add-label]\n  [a]\n  [--remove-label]\n  [b,c]\n\
             $ gh (cwd .)\n  [pr]\n  [edit]\n  [4]\n"
        );
    }
}
