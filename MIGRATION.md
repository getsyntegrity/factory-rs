# Software Factory v2 → Rust

## Goal

Port Software Factory v2 to Rust with **verifiable behavioral parity**. The
frozen v2 original is the oracle: a Rust piece is done only when
`rust-anything app replay` runs it against cases recorded from v2 and every
case matches (exit code, stdout, stderr and the files the run creates or
changes). Rust may be written by hand where `rust-anything` does not generate
it, but it must still pass replay.

## Oracle

| Field | Value |
| --- | --- |
| Source URL | https://github.com/zazencodes/zazencodes-season-3 |
| Path in source | `src/software-factory-claude-code-v2` |
| Frozen commit (`origin/main`) | `c467603d6d62392cfaae0c07d2f701e99e111443` (committed 2026-11-18T01:33:27-05:00) |
| Frozen on | 2026-11-18 |
| Local snapshot | `.oracle/v2` (git-ignored; its own repo on `main`, commit `6a7c1f8add1ec6f2e7134ba48440f20e6b10c76c` "oracle: freeze software factory v2") |
| rust-anything | 1.5.1 (built with rustc 1.97 through `cargo +1.97 install rust-anything`, because 1.5.1 requires rustc ≥ 1.97; the project itself builds with the default rustc 1.96.0) |
| Python | 3.12.3 |

The oracle is never edited. `.oracle/v2` had no changes after every record and
replay run (`git -C .oracle/v2 status --short` is empty). It is not the Go
implementation (`arch/factoryd-go`), which this migration does not use.

To rebuild the snapshot: clone the source URL, check out the frozen commit and
copy `src/software-factory-claude-code-v2` to `.oracle/v2`, then
`git init -b main && git add . && git commit -m "oracle: freeze software factory v2"`.

## Scope

In scope: the v2 executables `factory/bin/route`, `factory/bin/job`,
`factory/bin/gate`, one slice at a time. This slice ports **`route` only**.

Out of scope: aifactory Go, Gentle/MCP, the new ODD, living documentation, a
new front end, extra scanners, any product change. No push, PR, release or
real GitHub call; tests use fixtures, `gh`/`job` doubles and sandboxes.

## Components

| Component | Language | Size | Lines | Entry point | Status |
| --- | --- | ---: | ---: | --- | --- |
| `factory/bin/route` | Python 3 (no extension) | 1 798 B | 51 | `if __name__ == "__main__"` → `main(sys.argv[1:])` | **ported** (this slice) |
| `factory/bin/job` | Python 3 (no extension) | 28 866 B | 718 | `if __name__ == "__main__"`, subcommands `queue create advance merge feedback reject agents sync summary` | Python original; called by Rust `route` (fallback) |
| `factory/bin/gate` | Python 3 (no extension) | 4 842 B | 121 | `if __name__ == "__main__"` | Python original |
| `factory/checks.sh` | Bash | 621 B | 18 | script | not started |
| `factory/config.json` | JSON data | 1 122 B | 31 | read by all three binaries | data, kept as is |

Also observed, not executables: `.claude/agents/*.md`, `.claude/skills/factory/SKILL.md`
(agent prompts), `factory/dashboard.html`, `examples/jobs/*` (sample job
folders), docs. External programs used by v2: `git`, `gh`, `claude`
transcripts under `CLAUDE_CONFIG_DIR`.

## rust-anything analysis

Evidence: `migration/rust-anything/analyze/`.

1. `rust-anything analyze .oracle/v2 --whole-app --lang python` found
   **0 source files**: the tool only picks Python files by extension, so it
   silently skipped `job`, `gate` and `route` (`plan.md`, `plan.json`).
2. Workaround: `.oracle/analyze-view` is a git-ignored copy of the snapshot
   where the three executables are renamed `*.py`, byte-identical (checked with
   `sha256sum -c`). `rust-anything analyze .oracle/analyze-view --whole-app --lang python`
   found the 3 entry points (shape **cli**), one unit `factory/bin/` (727 lines,
   readiness 18, status caution) and decision D1 "port every entry point as a
   CLI" (`plan-view.md`, `plan-view.json`).

Tool limitations seen: extensionless scripts are not detected; `checks.sh` is
not listed; "Every dependency: None" although v2 shells out to `git`, `gh`
and reads Claude Code transcripts; the three executables are grouped into one
unit, so the per-unit `bridge/record/diff/gate` loop does not fit a CLI split
by executable. This migration therefore uses the whole-app flow
(`app record` / `app replay`). Risks it flagged for later slices: hash
iteration order, string slicing, a runtime-built regex in `job` (`section()`),
floor division in `elapsed()`.

## How `route` is recorded and replayed

The rust-anything "repo" for `route` is `tests/parity/route/`, so recordings
land in `tests/parity/route/rust/app-cases/` and never in the oracle.

Each case runs in a rust-anything sandbox (a temporary working directory
filled with the case's `files`). v2's `route` locates `factory/` from its own
path, not the working directory, so `tests/parity/route/launch.sh` installs the
build under test at `./factory/bin/route` in the sandbox, installs the oracle's
Python `job` beside it (unless the case ships a `job` double), points
`CLAUDE_CONFIG_DIR` at a directory that does not exist (so `job sync` never
reads real transcripts), runs it, and removes what it installed. The original
and the Rust build go through the same launcher: `sh launch.sh original` and
`sh launch.sh rust`.

Cases come from `tests/parity/route/inputs.jsonl`, generated by
`migration/rust-anything/route/make_inputs.py` from the oracle's config.

```
rust-anything app record tests/parity/route --inputs tests/parity/route/inputs.jsonl --no-traps --name route -- sh launch.sh original
rust-anything app replay tests/parity/route                                  # the original
cargo build --release --bin route
rust-anything app replay tests/parity/route --build "sh launch.sh rust"      # the Rust build
```

`--no-traps` keeps the generated trap inputs out: they run without fixtures,
so every one would only repeat the missing-config case. rust-anything added
one case on its own, `example-001` (arguments `["$@"]`, taken from the
launcher), which is kept.

Normalization: only rust-anything's built-in `[CWD]` for the sandbox path. No
`--normalize` was added. A second, independent recording of all cases was
identical (`migration/rust-anything/route/record-twice.txt`), and
rust-anything itself runs every case twice while recording.

### Recorded cases (22)

| Case | What it covers | v2 result |
| --- | --- | --- |
| 01-no-args | no argument | usage, exit 1 |
| 02-two-args | two arguments | usage, exit 1 |
| 03-job-missing | unknown job id | `route: job: no job …`, exit 1 |
| 04-no-intake | `intake` is null | `route: <id> has no intake.md`, exit 1 |
| 05/06/07 | risk low / medium / high, no surfaces | reviewer line, job.json updated |
| 08 | surface `ui` | `code haiku, ux sonnet, ui haiku` |
| 09 | surface `security` | `code sonnet, security opus` |
| 10 | surfaces `ui, security` | four reviewers in order |
| 11 | unknown risk | `route: risk 'extreme' isn't in config.json routing (low, medium, high)` |
| 12 | unknown surface | `route: surface 'api' isn't in … (ui, security)` |
| 13-sync-fails | `job` double: `sync` fails | job's stderr, stripped |
| 14-agents-fails | `job` double: `agents` fails and echoes its JSON argument | pins Python `json.dumps` format |
| 15a | id with spaces and `/../` | `route: job: no job bad id/../x` |
| 15b | issue number as id (`1`) | **Python traceback** (FileNotFoundError) |
| 16a | config.json missing | **Python traceback** (FileNotFoundError) |
| 16b | config.json not JSON | **Python traceback** (JSONDecodeError) |
| 17 | intake REJECT | `job agents` refuses |
| 18 | intake without RISK | `risk None isn't …` |
| 19 | risk `it's` | `risk "it's" isn't …` (Python repr) |
| example-001 | added by rust-anything | **Python traceback** (config missing) |

## Result: PASS WITH 4 ACCEPTED DIAGNOSTIC DIVERGENCES

Strict replay of the Rust build: **18/22**. This is **not** 22/22 exact parity
and must never be reported as such. Every case v2 handles itself matches byte
for byte: exit code, stdout, stderr and files. The 4 remaining cases differ in
stderr only. The difference is accepted below by Owner decision (2026-11-18).

### Accepted divergences

v2 does not handle these inputs: CPython ends with an uncaught exception and
prints a traceback full of interpreter internals (`/usr/lib/python3.12/pathlib.py`,
`json/decoder.py`, line numbers, carets). **CPython tracebacks are not part of
the v2 compatibility contract.** For these cases, Rust keeps exit code 1,
empty stdout and no file write of its own (it never reaches `job agents`).
It prints one stable, short line on stderr that starts with `route:` and
names the cause. It does not imitate Python paths, lines or frames, does not
print a traceback, and does not fall back to Python.

| ID | Case | Cause | v2 stderr (last line of the traceback) | Rust stderr |
| --- | --- | --- | --- | --- |
| DIV-ROUTE-01 | 16a-config-missing | config missing | `FileNotFoundError: … '[CWD]/factory/config.json'` | `route: factory/config.json is missing` |
| DIV-ROUTE-02 | 16b-config-invalid | config invalid | `json.decoder.JSONDecodeError: Expecting property name enclosed in double quotes: line 1 column 3 (char 2)` | `route: factory/config.json is not valid JSON (line 1, column 3)` |
| DIV-ROUTE-03 | 15b-issue-number-id | invalid job (an issue number is not a job folder) | `FileNotFoundError: … '[CWD]/factory/jobs/1/job.json'` | `route: job 1 has no factory/jobs/1/job.json` |
| DIV-ROUTE-04 | example-001 | config missing (wins over the arguments, as in v2) | `FileNotFoundError: … '[CWD]/factory/config.json'` | `route: factory/config.json is missing` |

In DIV-ROUTE-03 the v2 `job sync 1` (still Python, the fallback) resolves
issue 1 and rewrites `factory/jobs/001-add-a-thing/job.json` and `.lock`.
Replay shows those files are identical to v2's. route itself writes nothing.

Proof:
- replay: the four cases fail on `stderr` only (`replay-rust-final.txt`,
  `replay-case-*.txt`), so status, stdout and files match.
- unit tests `div_route_01..04` in `src/route.rs` check 4/4: exit code 1, empty
  stdout, no `job agents` call (route's only write), stderr starts with
  `route:` and names the cause.

The same diagnostic style covers unrecorded crash paths of v2: an unreadable
config (`route: cannot read factory/config.json`), an invalid job.json, a
config without `routing`/`surface_reviewers`, and a `job` that cannot start
(`route: cannot run factory/bin/job`). These have unit tests only. They are
not recorded cases.

Rejected options (2026-11-18): copying the traceback text. It would match
only the recorded inputs and would tie the port to CPython 3.12 internals.
Delegating these errors to the Python route was also rejected: it would keep a
second Python dependency.

## `route` in Rust

| File | Role |
| --- | --- |
| `src/route.rs` | the logic: `run(args, &dyn Factory) -> Outcome` |
| `src/py.rs` | Python `repr()` and `json.dumps()` formats that v2 makes observable |
| `src/system.rs` | real `Factory` (files under `factory/`, the sibling `job` process) and `route_main` |
| `src/bin/route.rs` | binary: writes the `Outcome`, sets the exit code |

**Fallback, pending:** `Installation::job` runs `factory/bin/job`, which is
still v2's Python `job`. It goes away with the `job` slice.

Tests are unit tests with an in-memory `Factory` double. The route tests load
their expected exit code, stdout and stderr from the recorded case files with
`include_str!`, so a re-recording that changes v2 behavior breaks them.

## Evidence (`migration/rust-anything/route/`)

| File | What |
| --- | --- |
| `record-command.txt`, `record.*` | recording command and output |
| `record-twice.*` | second recording, diff against the first (empty) |
| `replay-original.stdout` | original vs its own recording: 22/22 |
| `red-cargo-test.txt` | RED: 36/36 unit tests fail on `todo!()` stubs |
| `red-replay.txt`, `red-replay-report.md` | RED: stub binary 0/22 |
| `replay-rust-pre-decision.txt` | Rust 18/22 before the Owner decision |
| `red-divergences.txt` | RED: the 8 diagnostic tests failing before the messages changed |
| `replay-rust-final.txt`, `replay-rust-final-report.md` | final Rust replay: 18/22, 4 accepted divergences |
| `replay-case-*.txt` | replay of each divergent case on its own |
| `replay-original-final.txt` | original after all work: 22/22 |
| `gates.txt` | fmt, clippy, test, build, coverage |
| `ra-gate.txt` | `rust-anything gate .`: does not apply (needs a `bridge` unit) |

## Gates

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(47 unit tests), `cargo build --release --bin route` all pass. Line coverage
is 98.20% (`cargo llvm-cov`). The only lines not covered are the 6 lines of
`src/bin/route.rs`; covering them would need an integration test, which this
slice does not use.

`rust-anything gate` checks a `bridge` unit (a PyO3/napi crate per library
unit) and needs its `rust-anything.json`, so it does not apply to a CLI
replayed whole-app. `app lift` was not run: `job` and `gate` are still the
Python originals.

## Next slice

Port `job` (718 lines). Start with `sync` and `agents`, the two subcommands
`route` calls, so the `route → job` fallback can go away. The cases will need
`gh`/`git` doubles. Also watch `job`'s file lock and the `CLAUDE_CONFIG_DIR`
transcript pricing.
