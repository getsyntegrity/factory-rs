Stacked on #0 — review and merge in order.

## What

`gate <job>`, `gate <job> --merge` and `gate --hook` in Rust, with Python `fnmatch` semantics for protected paths.

Details, recorded cases and accepted divergences: [MIGRATION.md — this slice](https://github.com/getsyntegrity/factory-rs/blob/feat/gate/MIGRATION.md#slice-3-gate).

## Parity (rust-anything replay against v2 @ c467603d)

gate 22/29 with 7 accepted diagnostic divergences (DIV-GATE-01..07). Also fixes a 1-in-120 flaky test (fork inheriting a flock descriptor). Every failing case differs in stderr only (a CPython traceback in v2, one `prog: <cause>` line in Rust), per the Owner decision recorded in MIGRATION.md.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass on this branch.

## Not in this PR

No change to v2 (the oracle is read-only); no real GitHub call in tests (gh/git are doubles).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
