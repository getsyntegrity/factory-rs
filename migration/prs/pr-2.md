Stacked on #1 — review and merge in order.

## What

`job summary`, `job feedback` and `job reject` in Rust, with the `advance` branches they use (needs-human → build, waiting → rejected) and v2's gh/git command layer. gh/git doubles log every call so replay checks the GitHub side effects.

Details, recorded cases and accepted divergences: [MIGRATION.md — this slice](https://github.com/getsyntegrity/factory-rs/blob/feat/job-waiting-commands/MIGRATION.md#slice-4-job-summary-job-feedback-job-reject).

## Parity (rust-anything replay against v2 @ c467603d)

job 39/45 (DIV-JOB-06 added), job-reject 7/7 including every gh/git call. Every failing case differs in stderr only (a CPython traceback in v2, one `prog: <cause>` line in Rust), per the Owner decision recorded in MIGRATION.md.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass on this branch.

## Not in this PR

No change to v2 (the oracle is read-only); no real GitHub call in tests (gh/git are doubles).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
