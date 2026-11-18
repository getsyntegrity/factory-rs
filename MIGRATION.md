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
`factory/bin/gate`, one slice at a time. Slice 1 ported `route`; slice 2
ported `job sync` and `job agents`.

Out of scope: aifactory Go, Gentle/MCP, the new ODD, living documentation, a
new front end, extra scanners, any product change. No push, PR, release or
real GitHub call; tests use fixtures, `gh`/`job` doubles and sandboxes.

## Components

| Component | Language | Size | Lines | Entry point | Status |
| --- | --- | ---: | ---: | --- | --- |
| `factory/bin/route` | Python 3 (no extension) | 1 798 B | 51 | `if __name__ == "__main__"` → `main(sys.argv[1:])` | **ported** (slice 1) |
| `factory/bin/job` | Python 3 (no extension) | 28 866 B | 718 | `if __name__ == "__main__"`, subcommands `queue create advance merge feedback reject agents sync summary` | `sync`, `agents` **ported** (slice 2); every other argv runs the v2 original as `factory/bin/job.py` (fallback) |
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

`Installation::job` runs the sibling `factory/bin/job`. Since slice 2 that
is the Rust `job`, which answers `sync` and `agents` itself, so `route` no
longer runs Python (proven by the `rust-tripwire` replay below).

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

## Slice 2: `job sync` and `job agents`

### Layout and fallback

The Rust `job` binary (`src/bin/job.rs`) owns exactly two argv shapes:
`sync <job>` and `agents <job> <json>`. Every other argv (`-h`, `queue`,
`create`, `advance`, `merge`, `feedback`, `reject`, `summary`, wrong argument
counts) replaces the process with v2's original Python job. That original is
installed beside the binary as `factory/bin/job.py`.

**FALLBACK, pending:** `job.py` stays until those subcommands are ported.

v2 loads `config.json` when the module is imported, before it reads any
argument. The Rust job does the same, for every argv, before it hands
anything to `job.py`. A missing or broken config therefore never reaches
Python (DIV-JOB-05).

Rejected design: calling `sync`/`agents` in-process from `route`. v2 takes the
job's `flock` once per process. A second lock on a new descriptor in the same
process would block, and the locking semantics would change.

| File | Role |
| --- | --- |
| `src/job/mod.rs` | `Workspace` trait, argv ownership, `run`, resolve/load/save |
| `src/job/sync.rs` | v2 `sync()`: intake header, gate, review verdicts |
| `src/job/cost.rs` | v2 `cost_usd()`: transcript pricing, `round(x, 2)` |
| `src/job/disk.rs` | real `Workspace` (files, `flock` on `.lock`, `job.tmp` → `job.json`, transcript glob) and `dispatch` |
| `src/job/fake.rs` | test-only in-memory `Workspace`; `case!` loads a recorded case |
| `src/py.rs` | adds Python `strip`, universal newlines, `splitlines`, float `repr`, `json.dumps(indent=2)`, `str()`, truthiness |
| `src/bin/job.rs` | binary: dispatch, or `exec` of `job.py` with the original argv bytes |

### Recording

The harness is `tests/parity/job/`, with cases generated by
`migration/rust-anything/job/make_inputs.py`. Each case runs through
`launch.sh original|rust`.

Claude Code transcript fixtures live under `claude-config/projects/REPO/`.
The launcher copies them outside the sandbox, renames `REPO` to the folder
name v2 derives from the sandbox path, and replaces `@REPO@` in their prompts.

```
rust-anything app record tests/parity/job --inputs tests/parity/job/inputs.jsonl --no-traps --name job -- sh launch.sh original
rust-anything app replay tests/parity/job                              # original: 33/33
rust-anything app replay tests/parity/job --build "sh launch.sh rust"  # Rust
```

There are 33 cases: 18 `sync`, 8 `agents`, 6 argv shapes for the fallback,
and the tool-added `example-001`. A second independent recording was
identical. `sync` and `agents` never call `gh` or `git`, so no doubles were
needed.

v2 behavior these cases pinned, which Rust reproduces:
- pathlib's `glob` in Python 3.12 matches hidden directories, so
  `.hidden/subagents/agent-*.jsonl` is priced.
- A non-directory at the session level is skipped.
- `round(x, 2)` rounds the exact binary value, ties to even, like Rust's `{:.2}`.
- `job.json` is written back as `json.dumps(indent=2)` + newline with
  `ensure_ascii`, and float `repr` (`1e+16`, `1e-05`, `100.0`, `-0.0`).
- New keys are appended and existing keys keep their position.
- `Path.read_text()` reads universal newlines, so a lone `\r` ends the first
  line of a review file.
- Header keys are stripped and upper-cased. A whitespace-only line ends the
  header. `RESCOPED` must be exactly `yes`.
- An issue number loads and locks every job, in sorted order.
- `job["agents"] and job["stage"] in …` and `stage != "intake" or …`
  short-circuit.

### Result: PASS WITH 5 ACCEPTED DIAGNOSTIC DIVERGENCES

`job` strict replay: **28/33**. All 6 fallback cases pass byte for byte. The 5
cases that differ differ in stderr only.

`route` with the Rust `job` (`launch.sh rust-all`): **18/22**, the same 4
accepted route divergences. job.json and `.lock` files are identical to v2.

`route` with a `job.py` tripwire (`launch.sh rust-tripwire`) also gives
**18/22**, and no case ever ran the tripwire. A manual check confirms the
tripwire fires for an unowned argv (`job --help` exits 97). So `route` no
longer runs Python.

| ID | Case | Cause | v2 (last traceback line) | Rust stderr | Files |
| --- | --- | --- | --- | --- | --- |
| DIV-JOB-01 | sync-15-keys-appended-in | job.json has no `reviews` | `KeyError: 'reviews'` | `job: factory/jobs/001-add-a-thing/job.json has no reviews` | v2 saves job.json **before** crashing; Rust saves it too (identical) |
| DIV-JOB-02 | sync-16-job-json-invalid | job.json invalid | `JSONDecodeError: Expecting property name …: line 1 column 3 (char 2)` | `job: factory/jobs/001-add-a-thing/job.json is not valid JSON (line 1, column 3)` | `.lock` created, as in v2 |
| DIV-JOB-03 | sync-17-config-missing | config missing | `FileNotFoundError: … factory/config.json` | `job: factory/config.json is missing` | none |
| DIV-JOB-04 | agents-05-invalid-json | agent set invalid | `JSONDecodeError: …: line 1 column 2 (char 1)` | `job: the agent set is not valid JSON (line 1, column 2)` | `.lock` created, job.json not saved, as in v2 |
| DIV-JOB-05 | example-001 | config missing (argv `$@`, not owned) | `FileNotFoundError: … factory/config.json` | `job: factory/config.json is missing` | none |

DIV-JOB-01 differs from the route divergences. There, route wrote nothing of
its own. Here the Owner rule ("ninguna modificación de archivos") is applied
as "no file effect beyond v2's": v2 itself writes job.json and then crashes,
and replay shows the Rust job.json is byte-identical.

Unit tests `div_job_01..05` in `src/job/mod.rs` check exit 1, empty stdout,
the exact stderr line, and that the job.json and `.lock` effects match the
recording.

Other v2 crash paths get the same diagnostic style, with unit tests only:
- config not valid JSON, or missing `labels`, `base_branch`, `concurrency` or
  `prices_usd_per_mtok`;
- job.json not an object, or missing `id`, `round` or `issue`;
- a broken transcript line;
- a model price missing a token kind;
- I/O failures (read, list, lock, write).

### Known unrecorded gaps

These inputs are not recorded, so there is no evidence either way:
- **Issue lookup:** an issue number matches integer `issue` values only. v2
  would also match `2.0` or `true`, and its `isdigit()` accepts non-ASCII
  digits.
- **Agent set:** `NaN`/`Infinity` are rejected by Rust; v2's `json.loads`
  accepts them.
- **Transcripts:** invalid UTF-8 inside a non-matching transcript fails in
  Rust. v2 only reads the first line of such a file.
- **Shapes:** v2 accepts a list as `agents.reviewers`; Rust names it as a
  missing table.

### Evidence (`migration/rust-anything/job/`)

| File | What |
| --- | --- |
| `make_inputs.py`, `record-*.txt`, `record.*` | cases and recording |
| `record-twice.*` | second recording: identical |
| `replay-original*.txt` | original vs its recording: 33/33 |
| `red-cargo-test.txt` | RED: 49 new unit tests failing on `todo!()` stubs |
| `replay-rust*.txt`, `replay-rust-final-report.md` | Rust replay: 28/33 |
| `replay-case-*.txt` | each divergent case on its own |
| `gates.txt` | fmt, clippy, tests, build, coverage, flakiness |
| `../route/replay-rust-all.txt`, `../route/replay-rust-tripwire.txt` | route with the Rust job, and with the tripwire |

There is no RED evidence from replaying a stub `job` binary. RED was observed
at the unit level, where the tests consume the same recorded cases.

### Gates (slice 2)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(117 unit tests) and `cargo build --release` pass on rustc 1.100.0. Line
coverage is 97.74%; only the two `src/bin/*.rs` shells are uncovered. 120
repeated `cargo test --lib` runs had no failure.

## Next slice

Port `gate` (121 lines) or `job summary` (read-only, already recorded as a
fallback case). `summary` shows `elapsed()`, which uses the current time
for unfinished jobs, so its cases must use finished jobs or a time
normalizer proven by two runs. Leave `advance`, `merge`,
`create` and `queue` for last: they call `gh` and `git` and will need doubles.
