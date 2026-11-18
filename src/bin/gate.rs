//! `gate` binary: process I/O around [`factory_rs::gate::run`].

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use factory_rs::Outcome;
use factory_rs::gate::{self, disk::Disk};

fn main() -> ExitCode {
    let args: Vec<String> = env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let outcome = match env::current_exe() {
        Ok(exe) => gate::run(
            &args,
            &Disk::new(&exe, env::var_os("HOME").map(PathBuf::from)),
        ),
        Err(e) => Outcome::die("gate", &e.to_string()),
    };
    let _ = io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = io::stderr().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code as u8)
}
