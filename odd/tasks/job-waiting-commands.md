# Feature: port `job summary`, `job feedback`, `job reject`

Objective: the Rust `job` answers `summary [<job>...]`, `feedback <job>` and `reject <job>` with parity against v2 (oracle c467603d), including the `advance` branches they use (needs-human → build, needs-info/needs-human → rejected).
Branch: `feat/job-waiting-commands` (stacked on `feat/gate`). Delivery: `ask-on-risk`. Test runner: `cargo test` (unit tests with doubles); parity via `rust-anything app replay`.

## Specs

- **S1** (user, L1): "si", answering "¿Sigo con el próximo slice (`job summary`, `feedback` y `reject`, que introducen el doble de `gh`)?"
- **S2** (standing migration rules): "La v2 original es el oráculo de comportamiento." "No modificar nunca los archivos del oráculo." "Usar dobles de `gh`, repositorios Git temporales y fixtures aislados." "No realizar operaciones contra GitHub real durante las pruebas." "No normalizar salidas salvo timestamps, UUID o valores realmente variables y comprobados ejecutando dos veces el mismo caso." "No usar comodines amplios que puedan esconder divergencias." "Ejecutar cada caso original al menos dos veces." "Primero demostrar que el replay del propio original pasa 100%." "Ejecutar `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`."
- **S3** (user): "tdd , 95% coverage , separar en modulos createes cuando sea necesario , no sobre ingenierai seamos inteligentes , test unitarios con mocks , no hagas integracion"
- **S4** (Owner decision, 2026-11-18): CPython tracebacks are accepted divergences (exit code, stdout and file effects kept; one stable `job:` line).
- **S5** (design, judgement call): `gh` and `git` are PATH doubles that append every call (argv, cwd) to `doubles.log` in the sandbox, so replay compares the GitHub/git side effects call by call; answers come from the case's `doubles/<tool>/<key>.{stdout,stderr,code}`. The `timestamps` normalizer is used only in a separate harness for `reject` (the only command here writing `now()`), after two recordings prove the value varies; unit tests pin its exact format with an injected clock. `summary` of unfinished jobs (elapsed from `now()`) is unit-tested only, not recorded.

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S1 S2 S5 | Record summary/feedback (tests/parity/job) and reject (tests/parity/job-reject) with logging gh/git doubles; twice; original replay 100% | inline | done | job 45 cases (33 old unchanged byte for byte), job-reject 7; variance of `ended` proven by two unnormalized recordings; normalized recordings identical; originals 100% |
| T2 | S1 S3 | `summary` (table, elapsed with injected clock), RED→GREEN | inline | done | summary: all recorded cases + unfinished-job clock test; RED→GREEN |
| T3 | S1 S3 S4 | Command layer (run/gh/comment/label/operator) and `advance` for needs-human→build and →rejected, RED→GREEN | inline | done | shell + advance (two branches, legal-transition table, unported refusal); found and fixed config read order (max_rounds before labels) |
| T4 | S1 S3 S4 | `feedback` and `reject` commands, argv ownership, RED→GREEN | inline | done | feedback/reject + argv ownership; recorded cases green, DIV-JOB-06 |
| T5 | S2 S4 | Replay all job/route/gate harnesses vs Rust, gates, coverage, MIGRATION.md | inline | done | job 39/45, job-reject 7/7, route 18/22, gate 22/29; 208 tests; coverage 97.43%; flaky 0/300 |

Route evidence: one sequential behavior, no parallel units; no writer delegated.

## Log

- L1 (2026-11-18, user): "si"
- L2: scope finding before T1: `feedback` and `reject` go through `advance()` (mtime check, next_round, labels, comments, `gh pr close`, `git push --delete`, worktree removal, `ended = now()`), so the slice is larger than "short" estimated earlier; still within 5 tasks.
- L3 (T1): rust-anything inserts `[..]` on its own where its two internal runs differ (e.g. `15:39:[..]`), so an unnormalized time-variant case would fail once the minute changes; hence `--normalize timestamps` in job-reject only. It also turns fixture `started` values into `[..]`; unit tests pin both. Fixture fix: waiting jobs need a worktree path (v2 `Path(None)` crashes on reject; real jobs always have one from `create`).
- L4 (T2–T5): RED 45 failing (`migration/rust-anything/job-reject/red-cargo-test.txt`), GREEN 208. Test-fixture fixes (not code): two summary unit fixtures lacked `agents`. The old dispatch test used `summary` as an unowned argv; now `advance`.
