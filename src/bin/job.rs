//! `job` binary: `sync` and `agents` in Rust; every other argv replaces this
//! process with v2's Python job (`factory/bin/job.py`, explicit fallback).

use std::env;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use factory_rs::Outcome;
use factory_rs::job::disk::{Disk, Dispatch, dispatch};

fn main() -> ExitCode {
    let raw: Vec<_> = env::args_os().skip(1).collect();
    let args: Vec<String> = raw
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let outcome = match env::current_exe() {
        Err(e) => Outcome::die("job", &e.to_string()),
        Ok(exe) => {
            let disk = Disk::new(
                &exe,
                env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from),
                env::var_os("HOME").map(PathBuf::from),
            );
            match dispatch(&args, &disk) {
                Dispatch::Done(outcome) => outcome,
                Dispatch::Fallback => {
                    // exec only returns when it fails.
                    let _ = Command::new(disk.fallback()).args(&raw).exec();
                    Outcome::die("job", "cannot run factory/bin/job.py")
                }
            }
        }
    };
    let _ = io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = io::stderr().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code as u8)
}
