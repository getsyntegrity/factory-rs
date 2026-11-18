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
ported `job sync` and `job agents`; slice 3 ported `gate`; slice 4 ported
`job summary`, `job feedback` and `job reject`; slice 5 ported `job create`
and `job queue`.

Out of scope: aifactory Go, Gentle/MCP, the new ODD, living documentation, a
new front end, extra scanners, any product change. No push, PR, release or
real GitHub call; tests use fixtures, `gh`/`job` doubles and sandboxes.

## Components

| Component | Language | Size | Lines | Entry point | Status |
| --- | --- | ---: | ---: | --- | --- |
| `factory/bin/route` | Python 3 (no extension) | 1 798 B | 51 | `if __name__ == "__main__"` → `main(sys.argv[1:])` | **ported** (slice 1) |
| `factory/bin/job` | Python 3 (no extension) | 28 866 B | 718 | `if __name__ == "__main__"`, subcommands `queue create advance merge feedback reject agents sync summary` | `sync`, `agents` (slice 2), `summary`, `feedback`, `reject` (slice 4), `create`, `queue` (slice 5) **ported**; `advance`, `merge`, help and bad argv run the v2 original as `factory/bin/job.py` (fallback) |
| `factory/bin/gate` | Python 3 (no extension) | 4 842 B | 121 | `if __name__ == "__main__"` | **ported** (slice 3) |
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

## Slice 3: `gate`

`gate <job>`, `gate <job> --merge` and `gate --hook` are all in Rust
(`src/gate/`, `src/bin/gate.rs`). There is no fallback: nothing in `gate`
still runs Python.

| File | Role |
| --- | --- |
| `src/gate/mod.rs` | argv, the gate run (round check, commit, protected paths, checks, `checks.{log,md}`), the SubagentStop hook |
| `src/gate/disk.rs` | real `Host`: files under `factory/`, `git`, `bash factory/checks.sh` timed in whole seconds, stdin, `~` expansion |
| `src/gate/fake.rs` | test-only `Host`; answers like the git double and reads the generated `checks.sh` fixtures |
| `src/py.rs` | adds `fnmatch` (POSIX: `*` crosses `/`, `?`, `[seq]`, `[!seq]`, ranges, unclosed `[` is literal) |
| `src/testing.rs` | test-only: `read_tree` for recorded cases, `process_lock` for tests that fork |

### Recording

The harness is `tests/parity/gate/`, with cases generated by
`migration/rust-anything/gate/make_inputs.py`.

`git` is a double that `launch.sh` puts first on PATH, outside the sandbox.
For `rev-parse` and `diff` it answers from the case's
`git-double/<cmd>.{stdout,stderr,code}`. It exits 99 unless its argv matches
`git-double/<cmd>.args` and its working directory is the job's worktree. A
real temporary repository was rejected: gate prints the commit SHA, and SHAs
depend on commit dates.

Jobs use the relative worktree `wt`, which v2 resolves against its working
directory.

The 29 cases cover:
- argv shapes;
- PASS and FAIL;
- protected paths for every `--name-status` letter, with the fnmatch forms;
- an 80-line tail with CRLF output;
- git failures, a different `base_branch` and a missing `checks.sh`;
- UTF-8 output;
- `--merge`;
- the hook (stop active, pass, fail, bad stdin, prompt without `Job: `);
- the tool-added `example-001`.

The first recording exposed a bug in the fixtures, not in v2: bash double
quotes printed `\n` literally. The fixtures now use `$'...'` quoting and were
re-recorded from scratch.

A second recording was identical, so the run time stays `0s` and no
normalizer was added. The original replays 29/29.

v2 behavior these cases pinned, which Rust reproduces:
- `die()` exits **2** and a traceback exits 1.
- Captured output is read with universal newlines, so CRLF becomes LF in
  `checks.log`.
- The log is stdout followed by stderr.
- Renames and copies report the old path. `T` (type change) counts; `A` is
  skipped.
- The merge summary always says "synced with the latest **main**", whatever
  `base_branch` is.
- The hook joins prompt blocks without filtering non-object blocks, which
  crash in v2.

### Result: PASS WITH 7 ACCEPTED DIAGNOSTIC DIVERGENCES

`gate` strict replay: **22/29**. The 7 cases that differ differ in stderr only:
exit code (1), stdout and files match v2.

| ID | Case | Cause | v2 (last traceback line) | Rust stderr |
| --- | --- | --- | --- | --- |
| DIV-GATE-01 | gate-11-job-missing | job missing | `FileNotFoundError: … factory/jobs/999-nope/job.json` | `gate: job 999-nope has no factory/jobs/999-nope/job.json` |
| DIV-GATE-02 | gate-12-round-dir-missin | round folder missing (checks already ran) | `FileNotFoundError: … round-1/checks.log` | `gate: cannot write factory/jobs/001-add-a-thing/round-1/checks.log` |
| DIV-GATE-03 | gate-13-config-missing | config missing | `FileNotFoundError: … factory/config.json` | `gate: factory/config.json is missing` |
| DIV-GATE-04 | hook-04-invalid-stdin | hook event not JSON | `JSONDecodeError: Expecting value: line 1 column 1 (char 0)` | `gate: the hook event on stdin is not valid JSON (line 1, column 2)` |
| DIV-GATE-05 | hook-05-no-job-in-prompt | prompt without `Job: ` | `IndexError: list index out of range` | `gate: the subagent prompt names no job ("Job: <id>.")` |
| DIV-GATE-06 | merge-03-dash-job | job `-weird` missing | `FileNotFoundError: … factory/jobs/-weird/job.json` | `gate: job -weird has no factory/jobs/-weird/job.json` |
| DIV-GATE-07 | example-001 | config missing (read before the argv) | `FileNotFoundError: … factory/config.json` | `gate: factory/config.json is missing` |

In DIV-GATE-04 the JSON position comes from Rust's parser (column 2), while
v2 says column 1. The message is stable but not v2's number. Under the Owner
rule a traceback is not contract, so this is accepted with the divergence.

Unit tests `div_gate_01..07` check each case against its recording. Unit tests
for unrecorded crash paths (written after the code, not RED-first) name:
- job.json shape problems;
- an unreadable config;
- `git` or `bash` that cannot start;
- a diff line without a tab;
- `protected_paths` that is not a list;
- every transcript problem in the hook.

### Known unrecorded gaps

These inputs are not recorded, so there is no evidence either way:
- Invalid UTF-8 output from `checks.sh` or `git` is decoded lossily in Rust;
  v2 crashes on it.
- `~user/...` transcript paths are not expanded.
- Malformed fnmatch ranges such as `[b-a]` are not compared against v2.

### Evidence (`migration/rust-anything/gate/`)

| File | What |
| --- | --- |
| `make_inputs.py`, `record-*.txt`, `record.*` | cases and recording |
| `record-twice.*` | second recording: identical |
| `replay-original*.txt` | original: 29/29 |
| `red-cargo-test.txt` | RED: 29 gate tests failing on `todo!()` |
| `replay-rust*.txt`, `replay-rust-report.md` | Rust replay: 22/29 |
| `replay-case-*.txt` | each divergent case on its own |
| `gates.txt` | fmt, clippy, tests, build, coverage, flakiness |

### Gates (slice 3)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(163 unit tests) and `cargo build --release` pass on rustc 1.100.0. Line
coverage is 97.58%; `src/gate/mod.rs` is at 100%.

The repeated test runs surfaced a race in an older test. While
`job::disk::tests::lock_*` held a `.lock`, a fork in another test thread could
inherit the descriptor and keep the `flock` held. That test now takes
`process_lock`: before the fix, 1 run in 120 failed; after it, 0 in 400.

`route` (18/22) and `job` (28/33) replays are unchanged.

## Slice 4: `job summary`, `job feedback`, `job reject`

`feedback` and `reject` are not small. Both go through v2's `advance()`.
Two of its transitions are now in Rust: needs-human → build and
needs-info / needs-human → rejected. Every other legal transition is refused
with "not ported to Rust yet; use the v2 job" instead of being guessed. It is
unreachable from the commands the Rust job owns.

| File | Role |
| --- | --- |
| `src/job/summary.rs` | the summary table: widths, `$x.xx`, `elapsed()`, total |
| `src/job/waiting.rs` | `feedback` (feedback-<k>.md, → build) and `reject` (rejection.md, → rejected) |
| `src/job/advance.rs` | v2 `TRANSITIONS`, the two ported branches, terminal worktree removal, `ended` |
| `src/job/shell.rs` | v2 `run` / `gh` / `operator` / `comment` / `label` and the `Refused` message format |
| `src/job/clock.rs` | ISO 8601 parse, `%Y-%m-%dT%H:%M:%SZ`, floored minutes for `elapsed()` (no date crate) |

### Recording

`gh` and `git` are PATH doubles, set up in `tests/parity/job/launch.sh`.
Every call appends its working directory and argv to `doubles.log` **inside
the sandbox**. So replay compares the GitHub and git side effects call by
call: comment bodies, labels, `pr close`, `push --delete`, `worktree remove`,
`branch -D`. Each answer comes from the case's
`doubles/<tool>/<first two args>.{stdout,stderr,code}`.

Adding the doubles left all 33 earlier job cases byte-identical.

- **`summary` and `feedback`:** 12 new cases in `tests/parity/job`, now 45 in
  total.
- **`reject`:** 7 cases in `tests/parity/job-reject`, recorded with
  `--normalize timestamps`, because reject writes `ended = now()`.

The variance was proven first: two recordings without a normalizer differed
(`15:40:19` vs `15:40:20`). rust-anything also inserts `[..]` on its own where
its two internal runs differ (`15:39:[..]`), so an unnormalized case would
break once the minute changes. With the normalizer, two recordings were
identical.

The normalizer also turns the fixture `started` into `[..]`. A unit test pins
both exact values (`started` kept, `ended` = the injected clock,
`2026-11-18T03:04:05Z`).

`summary` of an unfinished job uses the current time. It is unit-tested with
an injected clock and is not recorded.

v2 behavior these cases pinned, which Rust reproduces:
- **`summary`**
  - syncs for display and never saves, but locks every job it loads;
  - prints `-` for a PR `0` (falsy);
  - pads columns by characters and right-strips each row;
  - floors `elapsed()` (`59m`, `1h05m`, `25h00m`).
- **`feedback`**
  - asks for the operator (`gh api user --jq .login`) before writing
    feedback-<k>.md;
  - if `gh issue edit` fails, keeps feedback-<k>.md and leaves job.json
    unsaved;
  - sets `round_limit = round + max_rounds` and opens `round-<n+1>/`;
  - prints the new round.
- **`reject`**
  - the issue comment repeats the "By @operator …" line, because the reason
    is rejection.md from its second line on;
  - `git push --delete` failures are ignored, `git worktree remove` failures
    are not.
- **Order and naming**
  - `CONFIG["max_rounds"]` is read before `LABELS["needs_human"]`.
  - The "last" feedback is chosen by name, so feedback-10 sorts before
    feedback-9.
- **Fixtures**
  - Jobs need a worktree path: `Path(None)` crashes on reject, and real jobs
    always have one from `job create`.

### Result: PASS WITH 6 ACCEPTED DIAGNOSTIC DIVERGENCES (job)

| Harness | Rust replay |
| --- | --- |
| `job` | **39/45** (the 5 earlier DIV-JOB plus DIV-JOB-06; stderr only) |
| `job-reject` | **7/7**, including every `gh`/`git` call |
| `route` | 18/22 (unchanged; tripwire also 18/22, no Python call) |
| `gate` | 22/29 (unchanged) |

| ID | Case | Cause | v2 (last traceback line) | Rust stderr | Files |
| --- | --- | --- | --- | --- | --- |
| DIV-JOB-06 | feedback-07-no-decision | decision.md missing | `FileNotFoundError: … decision.md` | `job: factory/jobs/001-add-a-thing/decision.md is missing` | feedback-1.md and the `gh api user` call happen first, as in v2 |

### Known unrecorded gaps

These inputs are not recorded, so there is no evidence either way:
- **Dates:** `fromisoformat` accepts more forms than Rust's parser (dates
  only, other separators). Rust names those as "not ISO 8601".
- **Feedback numbering:** many feedback files (`feedback-10` beside
  `feedback-9`) are not recorded. Their mtimes would come from case setup
  order, which is not controlled. Unit-tested only.
- **Undecodable output:** `gh` or `git` output that is not UTF-8 is decoded
  lossily in Rust; v2 crashes on it.

### Evidence (`migration/rust-anything/job-reject/`, `migration/rust-anything/job/`)

| File | What |
| --- | --- |
| `job/make_inputs.py` | all job cases, including summary, feedback and reject |
| `job/record-slice4.*` | the job re-recording with the doubles |
| `job-reject/unnormalized-twice.diff` | proof that `ended` varies between runs |
| `job-reject/record-*.txt`, `record.*`, `record-twice.*` | normalized recording, twice, identical |
| `job-reject/replay-original.txt` | the four originals: 100% |
| `job-reject/red-cargo-test.txt` | RED: 45 new tests failing on `todo!()` |
| `job-reject/replay-{job,job-reject,route,gate}.txt` | Rust replays |
| `job-reject/gates.txt` | fmt, clippy, tests, build, coverage, flakiness |

### Gates (slice 4)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(208 unit tests) and `cargo build --release` pass. Line coverage is 97.43%.
300 repeated `cargo test --lib` runs had no failure.

## Slice 5: `job create`, `job queue`

`queue` moves a job whose PR merged on GitHub to `merged` before it builds
the rows, so this slice also ports `advance`'s → merged branch and
`pr_state`. That leaves four transition branches in Rust: → merged,
→ rejected (from waiting), needs-human → build, and the legal-transition
table. The rest are still refused, unported.

| File | Role |
| --- | --- |
| `src/job/create.rs` | existing-job and slot checks, issue checks, folder numbering, the new job.json, issue.md |
| `src/job/queue.rs` | auto-merge, then `new` / `resume` / `wait` / `skip` / `hold` rows and concurrency slots |
| `src/job/issue.rs` | `fetch_issue`, `write_issue_md` (issue quoted as data), `quote`, `replied_since_question`, `slugify` |
| `src/job/advance.rs` | adds → merged and `pr_state` |
| `src/job/shell.rs` | adds `gh_json` |

`create <issue>` is owned only when the argument is ASCII digits. v2 also
accepts any `str.isdigit()` string (and then crashes in `int()` for some).
Those strings, and every wrong argv shape, still go to the fallback, which
then behaves exactly like v2.

### Recording

The cases are in `tests/parity/job-queue`, with the `timestamps` normalizer.
`create` writes `started = now()` and the auto-merge writes `ended = now()`.
Two unnormalized recordings differed only in those two fields; two
normalized recordings were identical.

The doubles now answer by the **first three arguments** when such files exist
(`doubles/gh/issue_view_5.*`), else by the first two. Every earlier case still
replays unchanged.

The recordings also caught fixture mistakes, not v2 bugs: an unanswered
`gh pr view 9`, and an issue that had to be answered under two numbers.

v2 behavior these cases pinned, which Rust reproduces:
- **`create`**
  - asks GitHub for the issue **twice**: with the argument as given
    (`gh issue view 005`), then with the number GitHub returned
    (`gh issue view 5`) inside `write_issue_md`;
  - numbers the new folder after the highest one whose first three
    characters are digits (`010-no-job-json` counts, `12x-…` and `abc-…` do
    not);
  - slugs the title with the first four `[a-z0-9]+` words of the lowercased
    title (the Kelvin sign lowercases to `k`) or `issue`;
  - writes the worktree as an absolute path below the repository (`[CWD]` in
    the recording).
- **issue.md** quotes the body and the comments of the author and the
  operator only (`mallory` is dropped), credits marker comments to
  "factory", and shows an empty body as `(empty)`.
- **`queue`**
  - asks `gh pr view` for every non-terminal job with a PR, and **again**
    inside `advance` for a merged one;
  - drops the needs-human label when a needs-human job merges;
  - removes the worktree and the branch of a merged job;
  - a reply counts after the factory's last question, and building
    `{author, operator()}` always asks for the operator;
  - replied and new issues take the free slots in that order, and the rest
    are `hold`.

### Result: PASS WITH 1 ACCEPTED DIAGNOSTIC DIVERGENCE (job-queue)

| Harness | Rust replay |
| --- | --- |
| `job-queue` | **15/16** (DIV-JOB-07; stderr only) |
| `job` | 39/45 (unchanged) |
| `job-reject` | 7/7 (unchanged) |
| `route` | 18/22 (unchanged; tripwire 18/22) |
| `gate` | 22/29 (unchanged) |

| ID | Case | Cause | v2 (last traceback line) | Rust stderr |
| --- | --- | --- | --- | --- |
| DIV-JOB-07 | queue-06-invalid-json | `gh issue list` printed something that is not JSON | `JSONDecodeError: Expecting value: line 1 column 1 (char 0)` | `` job: `gh issue list --label factory --state open --limit 200 --json number,title` did not print JSON (line 1, column 2) `` |

### Known unrecorded gaps

These inputs are not recorded, so there is no evidence either way:
- **Issue numbers:** Python's `==` treats `5`, `5.0` and `True` as the same
  issue. Rust compares JSON values, so only identical numbers match.
- **Slugs:** `str.lower()` and Rust's `to_lowercase()` agree on everything
  tested, but not on every Unicode special case.

### Evidence (`migration/rust-anything/job-queue/`)

| File | What |
| --- | --- |
| `unnormalized-twice.diff` | only `started`/`ended` vary |
| `record-*.txt`, `record.*`, `record-twice.*` | normalized recording, twice, identical |
| `replay-original.txt` | all five originals: 100% |
| `red-cargo-test.txt` | RED: 27 new tests failing on `todo!()` |
| `replay-*.txt`, `replay-job-queue-report.md` | Rust replays |
| `gates.txt` | fmt, clippy, tests, build, coverage, flakiness |

### Gates (slice 5)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(236 unit tests) and `cargo build --release` pass. Line coverage is 97.46%.
300 repeated runs had no failure.

## Verification before porting the rest (advance, merge)

Requested before any new port: an oracle for everything still in Python, a
hybrid end-to-end replay, a lock check across languages, a stderr-consumer
audit and a manual plan for real GitHub. No production code changed here.

### Oracle for what is still Python (`tests/parity/job-advance`)

There are 70 cases, generated by
`migration/rust-anything/job-advance/make_inputs.py`. They are recorded with
the `timestamps` normalizer, because two unnormalized recordings differed
only in `ended` (`migration/rust-anything/job-advance/unnormalized-twice.diff`).
Two normalized recordings were identical, and the original replays 70/70.

| Transition / command | Cases | Includes |
| --- | ---: | --- |
| intake → needs-info | 3 | wrong decision, no intake.md |
| intake → rejected | 2 | wrong decision |
| intake → spec | 5 | not routed, `git fetch` fails, body without sections, `gh issue comment` fails |
| intake → (illegal) | 1 | |
| needs-info → intake | 4 | second time (intake-2.md), concurrency reached, **crash**: no intake.md |
| spec → build | 2 | no spec.md |
| build → gate (`git`) | 11 | not READY, no build.md, dirty, no commits, not pushed, no remote branch, no open PR, PR known, `git status` fails, **crash**: `rev-list` not a number |
| build → needs-human (`section`, `escalate`, needs-human comment) | 5 | no Blocked section, draft PR (→ ready), ready PR, not BLOCKED |
| gate → review / build / needs-human (`git`, `escalate`) | 5 / 2 / 3 | not run, other commit, failed, last round, too early, `rev-parse` fails |
| review → approve / build / needs-human (`check_reviews_done`, `escalate`) | 4 / 3 / 1 | missing review, invalid verdict, CHANGES, last round |
| approve → merging / needs-human (needs-human comment) | 3 / 2 | escalated, no decision.md, **crash**: one-line decision.md |
| other `job advance` refusals | 4 | → merged without a merged PR, → build without feedback, → rejected without rejection.md, terminal stage |
| `job merge` (`git`, `escalate`, gate `--merge`) | 10 | already merged, happy path, draft PR, conflict, gate fails, push fails, wrong stage, fetch fails, `gh pr merge` fails, from approve |

To support `merge`, two things changed in the harness:
- **`gate` installed beside `job`.** `merge` runs `factory/bin/gate`, so the
  job harness installs the matching gate: v2's in `original` mode, the Rust
  one in `rust` mode.
- **Doubles answer by call number.** `<key>.<n>.*` lets a PR change state
  during a case, so it can merge.

Every earlier harness still replays 100% against v2: 22 + 45 + 7 + 16 + 29 =
119 cases.

Replay of these cases with today's dispatch (Rust `job` handing advance and
merge to `job.py`, which runs the Rust `gate`) gives **67/70**. The 3 that
differ are v2 crashes. They differ in stderr only: the traceback names
`factory/bin/job.py` instead of `factory/bin/job`. Those are HYB-01..03 below.

### Hybrid end-to-end replay (`tests/parity/e2e`)

`tests/parity/e2e/scenarios/*.sh` run real command sequences. Between
commands they write what the agents would write (intake.md, spec.md,
build.md, reviews, decision.md). The scenarios are recorded 100% on v2 and
replayed with today's real dispatch (`launch.sh hybrid`): Rust `route`,
`gate` and `job`, with v2's `job.py` for advance and merge.

| Scenario | Steps | By Rust | By Python | Hybrid replay |
| --- | ---: | ---: | ---: | --- |
| `full-cycle`: create → route → spec → build → gate → review → sync → approve → queue → merge → summary | 12 | 6 | 6 | identical |
| `waiting-loop`: create → needs-info → queue (reply) → intake → route → spec → build → needs-human → feedback → needs-human → reject → summary | 12 | 6 | 6 | identical |
| `rework-rounds`: gate fails → round 2 → review CHANGES → round 3 → review CHANGES at the limit → needs-human | 16 | 6 | 10 | identical |

"Identical" covers every step's stdout and exit code, every job.json, every
round file, and **90 gh/git calls** in `doubles.log`. The hybrid replay ran
5 times: 3/3 each time
(`migration/rust-anything/e2e/replay-hybrid-repeat.txt`). Which language
answered each step comes from a `job.py` wrapper that logs every call
(`migration/rust-anything/e2e/hybrid-trace.txt`).

The scenarios hand job.json across languages in both directions:
- **Rust → Python:** `route` (Rust job sync/agents) writes job.json and
  `advance spec` (Python) reads it. `feedback` (Rust) writes, then
  `advance needs-human` (Python) reads.
- **Python → Rust:** `advance gate` (Python) writes, then `gate` (Rust)
  reads it and writes checks.md, which `advance review` (Python) reads.
  `advance review` (Python) writes, `job sync` (Rust) rewrites, and
  `advance approve` (Python) reads.

### Lock compatibility (`tests/parity/e2e/concurrency.sh`)

Both jobs lock `factory/jobs/<id>/.lock` with `flock(2)`: Python with
`fcntl.flock`, Rust with `File::lock`.

The script holds the lock with a real `job feedback`. It loads the job, and
so locks it, before it reads stdin, in both languages. Feeding stdin from a
FIFO keeps the lock held for 2 seconds while the other language's `job sync`
runs. The latest run (`migration/rust-anything/e2e/concurrency.txt`):

| Check | Result |
| --- | --- |
| Python holds, Rust `job sync` waits | waited 2.01 s, exit 0, PASS |
| Rust holds, Python `job sync` waits | waited 2.02 s, exit 0, PASS |
| Control: lock-free read while held (must not wait) | 0.00 s, PASS (both holders) |
| Race: Rust and Python `job sync` started together | 300 runs, 0 failures (both exit 0, job.json valid, same stdout) |

### Lock primitives (both sides)

| | Python (v2 `factory/bin/job`) | Rust (`src/job/disk.rs`) |
| --- | --- | --- |
| Code | `fcntl.flock(_locks[job_id], fcntl.LOCK_EX)` on `open(job_dir(job_id) / ".lock", "w")` (module `fcntl`, `load()`, job:110–116) | `File::create(jobs/<id>/.lock)?` then `file.lock()?`: `std::fs::File::lock` from the **standard library**, no crate (`Disk::lock`, disk.rs:125–126) |
| File | `factory/jobs/<id>/.lock`, created or truncated | the same file |
| Syscalls (observed) | `openat(…/.lock, O_WRONLY\|O_CREAT\|O_TRUNC\|O_CLOEXEC, 0666)`, then `flock(3, LOCK_EX)` | **identical**: `openat(…, O_WRONLY\|O_CREAT\|O_TRUNC\|O_CLOEXEC, 0666)`, then `flock(3, LOCK_EX)` |
| Held until | the process exits (the file object stays in `_locks`) | the process exits (the `File` stays in `Disk::locks`) |
| Kind | **advisory**: it only excludes other `flock` users. A process that writes `job.json` without locking is not stopped. | advisory, the same |

The syscalls come from tracing a real `job sync` of each language on one job,
using `strace -f -e trace=openat,flock,fcntl factory/bin/job sync 001-a` (and
`python3 factory/bin/job.py sync 001-a`) in a scratch factory.

**Where the lock tests ran.** The 300 races and the hold-and-wait tests ran on
Linux Mint 22.3, kernel `Linux 7.0.0-38-generic x86_64` (`uname -srmo`), on a
local filesystem.

**Does compatibility depend on the OS? Yes, in principle.** Both sides
exclude each other only because both end up calling `flock(2)`. That was
observed on Linux only.
- Python's `fcntl.flock` calls `flock(2)` wherever it exists.
- Rust's `File::lock` is documented as `flock` with `LOCK_EX` "on Unix". The
  Rust standard library may implement it differently on some platforms, so
  check the syscall rather than assume it.
- On network filesystems (NFS, SMB) `flock` behaves differently than on a
  local disk.

To repeat the check on another OS or filesystem, from the factory-rs root
after `cargo build --release` (needs `python3`, `bc`, `mkfifo`):

```sh
sh tests/parity/e2e/concurrency.sh 300                 # hold-and-wait both ways, lock-free control, 300 races
strace -f -e trace=openat,flock,fcntl factory/bin/job sync <id> 2>&1 | grep -E 'flock|F_SETLK|F_OFD'   # Linux, in an installed factory
sudo dtruss -f factory/bin/job sync <id> 2>&1 | grep -E 'flock|fcntl'                                # macOS, same
```

Run the trace commands once with the Rust `factory/bin/job` and once with
`python3 factory/bin/job.py`. The first command must print `PASS` for all four
hold-and-wait lines and `failures: 0`. If the second shows `fcntl(F_SETLK…)` or `F_OFD_SETLK` on one
side and `flock` on the other, the languages do not exclude each other on
that system.

### Hybrid-only differences

| ID | Case | What differs | Why |
| --- | --- | --- | --- |
| HYB-01 | job-advance `approve-05-needs-human-one-line-decision` | traceback path `factory/bin/job.py` vs `factory/bin/job` | v2 crash (IndexError) inside the fallback |
| HYB-02 | job-advance `build-11-rev-list-not-a-number` | same | v2 crash (ValueError) inside the fallback |
| HYB-03 | job-advance `info-04-no-intake-md` | same | v2 crash (FileNotFoundError) inside the fallback |

These are not accepted divergences: they only exist while `job.py` is the
fallback, and they disappear once those transitions are ported. The Owner
rule on tracebacks will apply to them then.

### `gate --hook`: event, matcher and fail-open (Owner decision pending)

**Where it is registered.** It is registered in one place only: the YAML
frontmatter of `.claude/agents/builder.md` (oracle, lines 6–10), as a `Stop`
hook with no matcher:

```yaml
hooks:
  Stop:
    - hooks:
        - type: command
          command: '"$CLAUDE_PROJECT_DIR"/factory/bin/gate --hook'
```

No `settings.json` exists under `.claude/`. SKILL.md and the other agents do
not register hooks; `grep -rn hook .claude` finds only builder.md.

**What Claude Code does with it.** These statements come from the official
docs, read in this session through a docs-lookup subagent, at
https://code.claude.com/docs/en/hooks and
https://code.claude.com/docs/en/sub-agents. Quotes are paraphrased where the
tool returned excerpts; check the pages before relying on the details.
- **Event.** A `Stop` hook in a subagent's frontmatter runs as
  **`SubagentStop`** when that subagent finishes.
- **Matcher.** None is given. For `SubagentStop`, an omitted matcher matches
  every agent type. `Stop` itself has no matcher support.
- **Exit codes:**
  - **0:** the subagent stops. stderr is never shown to Claude.
  - **2:** blocks. The subagent does not stop, and stderr reaches the
    subagent as its next instruction.
  - **Any other code (for example 1): a non-blocking error, "the action goes
    ahead"**, so the subagent stops. Where stderr goes is not documented for
    `Stop` or `SubagentStop`.
- **A hook that cannot start** (missing or non-executable `gate`) is a
  non-blocking error. `onFailure: "block"` has no effect on `SubagentStop`.
- **Untrusted folder.** If the project folder is not trusted, Claude Code
  skips frontmatter hooks; the builder still runs, without the gate.
- **`stop_hook_active`** is true when the subagent is already continuing
  because of a stop hook. The gate exits 0 then: "one nudge per stop".

**Per case.** The v2 exit codes come from the recordings. The Rust exit codes
come from running each recorded case's input through
`tests/parity/gate/launch.sh rust --hook`, rerun for this table.

| Case | Cause | v2 exit | Rust exit | Effect on `SubagentStop` | Fail-open? |
| --- | --- | ---: | ---: | --- | --- |
| hook-01-stop-hook-active | already continuing because of this hook | 0 | 0 | does not block; the builder stops | by design ("one nudge per stop") |
| hook-02-checks-pass | checks pass | 0 | 0 | does not block; the builder stops | no |
| hook-03-checks-fail | checks fail (exit 5) | 2 | 2 | **blocks**; stderr (message + last 40 log lines) goes to the builder as its next instruction | no |
| hook-04-invalid-stdin (**DIV-GATE-04**) | the hook event on stdin is not JSON | 1 (traceback) | 1 (one line) | does not block; the builder stops; stderr destination not documented | **YES: the gate could not evaluate and the stop goes ahead** |
| hook-05-no-job-in-prompt (**DIV-GATE-05**) | the subagent prompt has no `Job: <id>.` | 1 (traceback) | 1 (one line) | does not block; the builder stops | **YES** |
| argv-05-hook-with-extra | `gate --hook x` | 2 (usage) | 2 (usage) | would block with the usage text. Not reachable from the registered command, which passes no extra argument. | no |
| *(unrecorded)* config.json missing or invalid | | 1 (crash at import) | 1 | does not block | **YES** |
| *(unrecorded)* the prompt's job has no job.json | | 1 (crash) | 1 | does not block | **YES** |
| *(unrecorded)* transcript unreadable or not JSON | | 1 (crash) | 1 | does not block | **YES** |
| *(unrecorded)* `factory/checks.sh` missing | `bash` exits 127, so the checks "fail" | 2 | 2 | blocks | no (fail-closed) |
| *(unrecorded)* `factory/bin/gate` missing or not executable | the hook cannot start | (no exit) | (no exit) | non-blocking; the builder stops | **YES** |
| *(unrecorded)* project folder not trusted | the hook is skipped | — | — | the builder stops | **YES** |

DIV-GATE-01, 02, 03, 06 and 07 are not hook runs. They are `gate <job>` and
`gate <job> --merge`, which the orchestrator runs directly and `job merge`
runs as a subprocess. No hook event is involved:

| Case | Invoked by | v2 / Rust exit | Effect |
| --- | --- | --- | --- |
| DIV-GATE-01 job missing; DIV-GATE-03 and DIV-GATE-07 config missing; DIV-GATE-06 job missing (with `--merge`) | orchestrator (Bash), or `job merge` | 1 / 1 | **Orchestrator:** no `GATE:` line and exit 1. SKILL.md defines only PASS and FAIL, and its general rule is "When a script refuses… Fix the cause or stop and tell the user". **`job merge`:** any non-zero gate exit means `fail()`, so the job goes to needs-human (fail-closed). |
| DIV-GATE-02 round folder missing (checks already ran) | the same | 1 / 1 | the same; no checks.md is written |

**The fail-open risk.** Whenever `gate --hook` cannot evaluate (unreadable
input, no job, no config, the gate missing, an untrusted folder), the builder
is allowed to stop without the checks being run. This was true in v2 and is
**unchanged in Rust**, where exit codes match case by case. It is only a
nudge: the real gate still runs (`job advance <id> gate` and then `gate <id>`)
and fails the round. But the builder loses the early feedback, and the gate
round is spent.

**Owner decision pending:** "fail-closed (exit 2) cuando el gate no puede
evaluar".
- **What it would change:** the error cases above, in the `--hook` path only,
  would exit 2 with their one-line cause. The builder would then keep working
  with that message instead of stopping.
- **Trade-off:** if `stop_hook_active` cannot be read (stdin is not JSON), the
  hook cannot tell a first stop from a retry. It would block every time, until
  Claude Code's limit on consecutive stop-hook continuations ends the turn.
  The docs state that limit (eight) for `Stop`; for `SubagentStop` it is not
  stated.
- **Not done.** Parity with v2 holds until the Owner decides.

### Who reads stderr of `factory/bin/*` (audit, nothing changed)

I searched every file under `.claude/` of the oracle (`skills/factory/SKILL.md`
and the eight agents), plus README.md, INSTALL.md and DESIGN.md:

```
grep -rn -i 'stderr\|exit\|refus\|error\|fail\|prints\|output\|says\|message\|reason\|hook' .claude README.md INSTALL.md DESIGN.md
```

| Where | What is read | Stream | Touched by an accepted divergence? |
| --- | --- | --- | --- |
| SKILL.md:11–13 "When a script refuses, its message says why. Fix the cause or stop and tell the user; never work around it." | any refusal message | stderr (`die`) | **Not changed in meaning.** The 18 divergences are v2 crashes, not refusals. Where v2 printed a traceback, Rust prints the cause in one line. The orchestrator reads the text, it does not parse it. |
| SKILL.md:116–117 "If `advance` refuses … resume the builder once with the refusal" | `job advance` refusal text, passed to the builder | stderr | No: `advance` is still v2. In the hybrid, only the three crash tracebacks change their path (HYB-01..03). |
| SKILL.md:123, 138–139 "If it refuses because this was the last round" | the text `round N of M: go to needs-human` | stderr | No: `advance` is still v2, and that refusal is byte-identical in the hybrid replay. |
| SKILL.md:120 "It prints `GATE: PASS` or `GATE: FAIL`" | gate's verdict line | **stdout** | No: stdout is identical in every gate case, divergences included (stdout is empty in both). |
| SKILL.md:144–146 merge "moves the job to needs-human and says why" | merge's result line | stdout | No: `merge` is still v2. |
| SKILL.md:164 `job create` "refuse[s] when no slot is free" | the concurrency refusal | stderr | No: Rust prints v2's exact text (job-queue `create-04` replays byte for byte). |
| SKILL.md:158, 167 queue rows; "`job create <n>` prints the job id" | rows, job id | stdout | No: identical. |
| builder.md:6–10 Stop hook `factory/bin/gate --hook` | stderr and exit code | stderr | In Claude Code, exit 2 blocks the stop and feeds stderr to the builder; that path (`hook-03`) is byte-identical. DIV-GATE-04/05 are exit 1. With exit 1, Claude Code shows stderr to the user, not the model, so the model's input does not change. (Hook semantics as documented by Claude Code; not re-verified in this session.) |
| builder.md:34, approver.md:41 | `round-N/checks.md`, `checks.log` | files | No: the files are identical in every gate case. |

Conclusion: no prompt parses stderr text of the 18 divergent cases. The
orchestrator reads stderr only as a human-readable reason, and in those cases
Rust gives a shorter, clearer one. Decisions depend on exit codes and stdout
lines (`GATE:`, queue rows, job ids), and those are identical.

### Deviations from strict TDD in this step

- **Not RED-first:** the harness and scenario scripts
  (`tests/parity/doubles.sh`, `tests/parity/e2e/*`, the job-advance
  generator) are test infrastructure, not production code. They were checked
  by running them: original replays 100%, a second recording identical, and a
  control in the lock check.
- **No production code changed.**

## Merge validation against real GitHub (manual, run by the Owner)

**Purpose:** before `job.py` is deleted, confirm on a real GitHub repository
that `job merge` (and the transitions around it) behave as the doubles
claim. The doubles reproduce the calls; they cannot prove GitHub's answers.
The agent must not run any of this.

### Setup (once)

1. **Create a throwaway private repository** under an account you control,
   for example `<you>/factory-merge-check`, with `gh repo create --private`.
   Use nothing that matters.
2. **Commit v2 to it.** Clone it, copy `.oracle/v2/factory` and
   `.oracle/v2/.claude` into it, write a `factory/checks.sh` that runs a
   one-line test, commit to `main` and push.
3. **Create the labels** with `gh label create`: `factory`,
   `factory:needs-info`, `factory:needs-human`.
4. **Install the build under test** in `factory/bin/` of the clone:
   - **hybrid run (today):** `cargo build --release` in factory-rs, copy
     `target/release/{route,job,gate}` there, and copy v2's job to
     `factory/bin/job.py`.
   - **reference run:** v2's own `factory/bin/{route,job,gate}`.

### Run (for each of: reference, hybrid)

1. **Open an issue** labeled `factory`, with a body that names a tiny change.
   `factory/bin/job queue` must print one `new` row. Then run
   `factory/bin/job create <n>` and note the id.
2. **Play the agents by hand**, between the commands, the way
   `tests/parity/e2e/scenarios/full-cycle.sh` does:
   - write `intake.md` (`DECISION: ACCEPT`), then
     `route <id>` → `job advance <id> spec`;
   - write `spec.md` → `job advance <id> build`;
   - in `.worktrees/<id>`: commit the change, `git push -u origin factory/<id>`,
     then `gh pr create --draft --body "Closes #<n>"`;
   - write `build.md` (`STATUS: READY`) → `job advance <id> gate`
     → `gate <id>` → `job advance <id> review`;
   - write one `round-1/review-<r>.md` per reviewer (`VERDICT: PASS`)
     → `job advance <id> approve`;
   - write `decision.md` (`APPROVE`).
3. **Merge:** `factory/bin/job merge <id>`. Expected:
   - **output:** `<id>: merged PR #<pr>`, exit 0;
   - **job.json:** `stage` = `merged`, `ended` set;
   - **on GitHub:** the PR is merged with the configured `merge_method`, not
     a draft first; the issue is closed by the PR; the remote branch is
     deleted (or "already deleted", which merge ignores);
   - **locally:** the worktree and the local branch are gone.
4. **Failure paths**, one fresh issue each:
   - **conflict:** push a commit to `main` that conflicts with the branch,
     then `job merge`. Expected: `needs-human: The branch conflicts with main`,
     the PR is marked ready, a `<!-- factory -->` comment with the decision is
     posted on the PR, the issue gets `factory:needs-human`, and `main` is
     untouched (the merge was aborted).
   - **gate fails after sync:** make `checks.sh` on `main` fail, then
     `job merge`. Expected: `needs-human: The gate failed after syncing with
     main`, and `round-N/merge-checks.md` says `GATE: FAIL`.
   - **already merged on GitHub:** merge the PR in the web UI, then
     `job merge`. Expected: `merged PR #<pr>` without any push.
5. **Record and compare.** Save from each run: `job.json`, `round-*/`,
   `decision.md`, the PR (`gh pr view <pr> --json state,isDraft,comments,mergeCommit`)
   and the issue (`gh issue view <n> --json state,labels,comments`). The
   hybrid run must match the reference run, except for numbers, URLs,
   timestamps and SHAs.

### Cleanup

Run `gh repo delete <you>/factory-merge-check --yes`, and remove the local
clone.

### When it passes

Record the date, the factory-rs commit, the `gh --version` and the
comparison in this file. Rerun the procedure once `merge` itself is ported to
Rust, with the Rust job and no `job.py`, before deleting `job.py`.

## Next slice

Only `advance` (most of its transitions) and `merge` remain in Python:

- **Intake:** intake → needs-info / rejected / spec, and needs-info → intake.
  This needs comments, labels, `require_slot`, the intake.md rename, and
  `git fetch` + `git worktree add` for spec.
- **Build:** spec → build, build → gate (`STATUS: READY`, clean and pushed
  branch, PR lookup), build → needs-human (`escalate`).
- **Gate, review and approve:** gate → review / build / needs-human; review →
  approve / build / needs-human (`check_reviews_done`); approve → merging /
  needs-human; the needs-human comment and PR ready.
- **`merge`:** merging → merged with `gate --merge`.

These are four slices. After them, `job.py` can be removed and `app lift`
applies.
