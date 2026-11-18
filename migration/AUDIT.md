# Audit: Software Factory v2 → Rust (2026-11-18)

Every number below comes from one fresh run (2026-11-18) of
`sh migration/audit/run_audit.sh` (full output:
`migration/audit/run_audit.out`). The sections A1–A10 below are the sections
of that run. Nothing recorded earlier was trusted without running it again.

## Verdict

1. **Real use in the skill: Rust answers 17 of the 35 calls (48.6%).** The
   35 are the `factory/bin/*` calls named in v2's skill and agent prompts.
   The 18 still in Python are 15× `job advance` and 3× `job merge`. In the
   three end-to-end lifecycles, Rust answered 18 of 40 steps; the other 22
   were `advance` and `merge`, handed to v2's `job.py`.
   *Verified by:* A3 `python3 -I migration/audit/measure_skill.py`, and A10
   `sh tests/parity/e2e/trace.sh`.
2. **Recorded behavior**, as four separate measures:
   - **(a) v2 against its own recordings: 192/192.** Every harness, the
     original against itself (A4).
   - **(b) Rust on the ported commands: 101/119 byte for byte, plus 18
     accepted divergences.** The divergences are DIV-ROUTE-01..04,
     DIV-GATE-01..07 and DIV-JOB-01..07. They differ in stderr only: one
     `prog: <cause>` line where v2 crashes with a traceback (A5, A6).
   - **(c) Hybrid against the oracle of what is still Python: 67/70, plus
     HYB-01..03.** The three differ **only in the path in the traceback**
     (`factory/bin/job.py` instead of `factory/bin/job`) of three v2 crashes
     inside the fallback. Exit code, stdout and files are identical (A5, A6).
   - **(d) Whole lifecycles, hybrid against all-v2: 3/3** (A5 e2e).
3. **Lines: 474 of v2's 623 executable lines (76.1%) have a Rust
   counterpart.** `route` and `gate` are at 100%, `job` at 71.3%, and `job
   advance` at 25%. Not ported: `section`, `escalate`, `git`,
   `check_reviews_done`, `cmd_merge`.
   *Verified by:* A8 `python3 -I migration/audit/measure_lines.py`.

## Claims and the command that verifies each

| # | Claim | Result in `run_audit.out` | Command |
| --- | --- | --- | --- |
| A1 | The oracle is v2 at `c467603d`, byte for byte, and was never modified | `oracle == source at c467603d`; 0 uncommitted changes in `.oracle/v2` | `git -C .oracle/src-repo archive c467603d… \| tar -x` + `diff -r -x .git … .oracle/v2`; `git -C .oracle/v2 status --short` |
| A2 | Gates pass | fmt OK; clippy clean; **236 tests passed**; release build OK; **97.46%** line coverage | `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`; `cargo build --release`; `cargo llvm-cov --summary-only` |
| A3 | Skill usage: Rust answers 17/35 | per-call table, then `answered by Rust: 17/35 (48.6%)` | `python3 -I migration/audit/measure_skill.py` |
| A4 | Every harness reproduces v2 against its own recordings | route 22/22, job 45/45, job-reject 7/7, job-queue 16/16, gate 29/29, job-advance 70/70, e2e 3/3 (**192/192**) | `rust-anything app replay tests/parity/<harness>` |
| A5 | Rust and hybrid replay | route 18/22, job 39/45, job-reject 7/7, job-queue 15/16, gate 22/29 (**101/119**); job-advance hybrid 67/70; e2e hybrid 3/3; route with the `job.py` tripwire 18/22 | `rust-anything app replay tests/parity/<h> --build "sh launch.sh rust"` (e2e: `"sh launch.sh hybrid"`; tripwire: `"sh launch.sh rust-tripwire"`) |
| A6 | Every differing case differs in stderr only, and only by the one-line diagnostic or the `job.py` traceback path | differing fields: only `stderr` (4, 6, 0, 1, 7, 3); added lines other than a diagnostic or `job.py` traceback: **0** in every harness | the replay's `- <case>: <fields>` lines, and `grep -E '^\+ ' …/replay-report.md` |
| A7 | Rust and Python job locks exclude each other | Python holds → Rust waits 2.01 s; Rust holds → Python waits 2.01 s; lock-free control 0.00 s; **100 races, 0 failures** | `sh tests/parity/e2e/concurrency.sh 100` |
| A8 | 474/623 v2 lines ported | `ALL: 474/623 (76.1%)`; `job advance: 31/126 (25%)` | `python3 -I migration/audit/measure_lines.py` |
| A9 | Git state | 6 local branches, the 5 slice branches pushed and tracking `origin`, `feat/verification-gaps` local | `git branch -vv`; `git status --short` |
| A10 | Who answers each e2e step | full-cycle 12 steps: 6 Rust / 6 Python; waiting-loop 12: 6 / 6; rework-rounds 16: 6 / 10 | `sh tests/parity/e2e/trace.sh` |
| A11 | `gate --hook` exit codes match v2 case by case | 0/0, 0/0, 2/2, 1/1, 1/1 (v2/Rust) for hook-01..05 | each recorded case's input through `tests/parity/gate/launch.sh rust --hook`, against the recorded `status.code` |

Within this session, the concurrency script also ran with 300 races
(`migration/rust-anything/e2e/concurrency.txt`) and the e2e hybrid replay ran
5 times, 3/3 each (`migration/rust-anything/e2e/replay-hybrid-repeat.txt`).
Those are earlier runs, kept as evidence; the table above is the fresh one.

## Recorded cases by harness

| Harness | What it covers | Cases | Against v2 | Rust / hybrid |
| --- | --- | ---: | ---: | ---: |
| `route` | route | 22 | 22 | 18 + DIV-ROUTE-01..04 |
| `job` | sync, agents, summary, feedback, fallback argv | 45 | 45 | 39 + DIV-JOB-01..06 |
| `job-reject` | reject | 7 | 7 | 7 |
| `job-queue` | create, queue | 16 | 16 | 15 + DIV-JOB-07 |
| `gate` | gate, `--merge`, `--hook` | 29 | 29 | 22 + DIV-GATE-01..07 |
| `job-advance` | everything still Python: advance transitions, merge, section, escalate, check_reviews_done, git | 70 | 70 | 67 + HYB-01..03 (hybrid) |
| `e2e` | three whole lifecycles | 3 | 3 | 3 (hybrid) |
| **Total** | | **192** | **192** | |

The per-transition counts of `job-advance` are in MIGRATION.md, "Oracle for
what is still Python". Regenerate them with
`python3 migration/rust-anything/job-advance/make_inputs.py`, which prints
them.

## Riesgos del modo híbrido

The results of step 3 of the brief: today's real dispatch (Rust `route`,
`gate`, `job`; v2's `job.py` for advance and merge) against an all-v2 run.

| Risk | Result | Evidence |
| --- | --- | --- |
| Rust and Python write job.json differently and the next command misreads it | **Not observed.** Three lifecycles hand job.json across languages both ways and end identical to all-v2 in every job.json, round file, stdout line and exit code, and in 90 gh/git calls. | A5 e2e 3/3; A10; MIGRATION.md "Hybrid end-to-end replay" |
| The two languages' locks do not exclude each other | **Not observed.** Each waits for the other's flock (2.01 s on a 2 s hold), the lock-free control does not wait, and 100 races (300 earlier) had 0 failures. | A7 |
| `job merge` (Python) runs the Rust `gate --merge` and reads its output differently | **Not observed** in the 10 merge cases and in full-cycle. The gate line and merge-checks files are identical. | A5 job-advance (merge-01..10 all pass); e2e full-cycle |
| Crashes inside the fallback print a different traceback | **Present, stderr only:** HYB-01..03 name `factory/bin/job.py` instead of `factory/bin/job`. The exit code (1), stdout and files are identical. | A5/A6 job-advance |
| The skill or the agents read stderr text that changed | **No consumer depends on it.** Decisions use exit codes and stdout (`GATE:` lines, queue rows, job ids), and those are identical. stderr is read as a human-readable reason only. | MIGRATION.md "Who reads stderr" |
| **`gate --hook` fails open** when it cannot evaluate (bad input, missing job or config, gate missing, untrusted folder): exit 1 or no run lets the builder stop without the checks | **Present in v2 and unchanged in Rust**: exit codes match case by case. The real gate still runs at `advance gate` and fails the round. **Owner decision pending:** "fail-closed (exit 2) cuando el gate no puede evaluar". | A11; MIGRATION.md "`gate --hook`: event, matcher and fail-open" |
| Python is still required | **Yes**, for `job advance`, `job merge`, `-h` and wrong argv. `route` and `gate` run without Python. | A5 tripwire 18/22 |
| The doubles answer differently from real GitHub | **Not testable here:** gh and git are doubles in every harness. MIGRATION.md has a manual procedure against a throwaway repository, to run before `job.py` is deleted. | MIGRATION.md "Merge validation against real GitHub" |

### `gate --hook` per case (builder `SubagentStop` hook)

The hook is registered as `Stop` in `.claude/agents/builder.md`; Claude Code
runs it as **`SubagentStop`**, with no matcher. Effects follow the Claude Code
hooks docs: exit 2 blocks and sends stderr to the builder; any other non-zero
exit is a non-blocking error and the builder stops (see MIGRATION.md for the
sources and the unrecorded paths).

| Case | Cause | v2 exit | Rust exit | Effect | Fail-open |
| --- | --- | ---: | ---: | --- | --- |
| hook-01 | `stop_hook_active` (already nudged) | 0 | 0 | does not block | by design ("one nudge per stop") |
| hook-02 | checks pass | 0 | 0 | does not block | no |
| hook-03 | checks fail | 2 | 2 | **blocks**, stderr to the builder | no |
| hook-04 / DIV-GATE-04 | hook input not JSON | 1 | 1 | non-blocking; the builder stops | **yes** |
| hook-05 / DIV-GATE-05 | prompt names no job | 1 | 1 | non-blocking; the builder stops | **yes** |
| unrecorded | config or job.json missing, transcript unreadable | 1 | 1 | non-blocking | **yes** |
| unrecorded | gate missing or not executable; untrusted folder | — | — | the hook does not run | **yes** |

DIV-GATE-01, 02, 03, 06 and 07 are not hook runs (`gate <job>` or
`--merge`). Under `job merge`, any non-zero gate exit sends the job to
needs-human (fail-closed). Run by the orchestrator, they print no `GATE:`
line.

## Open items

- **Merge against real GitHub.** The manual procedure in MIGRATION.md (run A
  now as a baseline, run B after merge is ported, with real gh answers
  captured as fixtures) has not been run (Owner action).
- **Fail-closed hook.** Owner decision pending (see above).
- **Stacked PRs.** The bodies and the `gh pr create` commands are in
  `migration/prs/`; opening them is an Owner action.
- **Remaining port.** Four slices: advance intake, advance build,
  gate/review/approve, merge. The oracle for all of them is recorded
  (`tests/parity/job-advance`, 70 cases) and replays 100% against v2.
- **HYB-01..03.** They go away when those transitions are ported. The Owner
  rule on tracebacks will apply to the Rust versions.
- **TDD deviations.** The harness and scenario scripts are test
  infrastructure, verified by running them rather than RED-first. No
  production code changed in this verification step (MIGRATION.md,
  "Deviations from strict TDD in this step").
