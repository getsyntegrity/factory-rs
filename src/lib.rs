//! Rust port of Software Factory v2 (see MIGRATION.md).

pub mod gate;
pub mod job;
pub mod py;
pub mod route;
pub mod system;

#[cfg(test)]
mod testing;

/// The process result a ported command must reproduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    /// v2's `die(msg)`: `<prog>: <msg>` on stderr, exit 1.
    pub fn die(prog: &str, msg: &str) -> Self {
        Outcome {
            code: 1,
            stdout: String::new(),
            stderr: format!("{prog}: {msg}\n"),
        }
    }
}
