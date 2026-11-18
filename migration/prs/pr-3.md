Stacked on #2 — review and merge in order.

## What

`job create` and `job queue` in Rust, with issue.md rendering and the `advance` → merged branch for jobs whose PR merged.

Details, recorded cases and accepted divergences: [MIGRATION.md — this slice](https://github.com/getsyntegrity/factory-rs/blob/feat/job-create-queue/MIGRATION.md#slice-5-job-create-job-queue).

## Parity (rust-anything replay against v2 @ c467603d)

job-queue 15/16 (DIV-JOB-07 added); all other harnesses unchanged. Every failing case differs in stderr only (a CPython traceback in v2, one `prog: <cause>` line in Rust), per the Owner decision recorded in MIGRATION.md.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass on this branch.

## Not in this PR

No change to v2 (the oracle is read-only); no real GitHub call in tests (gh/git are doubles).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
