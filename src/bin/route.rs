//! `route` binary: process I/O around [`factory_rs::system::route_main`].

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let outcome = factory_rs::system::route_main(env::args_os().skip(1), env::current_exe());
    // A failed write (closed pipe) has nowhere to be reported; the exit code still is.
    let _ = io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = io::stderr().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code as u8)
}
