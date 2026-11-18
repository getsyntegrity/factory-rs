First PR of the stack (base `develop`).

## What

The Rust `job` answers `job sync` and `job agents`; every other argv execs v2's Python job as `factory/bin/job.py` after the import-time config check.

Details, recorded cases and accepted divergences: [MIGRATION.md — this slice](https://github.com/getsyntegrity/factory-rs/blob/feat/job-sync-agents/MIGRATION.md#slice-2-job-sync-and-job-agents).

## Parity (rust-anything replay against v2 @ c467603d)

job 28/33 with 5 accepted diagnostic divergences (DIV-JOB-01..05); route with the Rust job 18/22, and a `job.py` tripwire shows route no longer runs Python. Every failing case differs in stderr only (a CPython traceback in v2, one `prog: <cause>` line in Rust), per the Owner decision recorded in MIGRATION.md.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass on this branch.

## Not in this PR

No change to v2 (the oracle is read-only); no real GitHub call in tests (gh/git are doubles).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
