# Feature: port `job create` and `job queue`

Objective: the Rust `job` answers `create <issue>` and `queue` with parity against v2 (oracle c467603d), including issue.md rendering and the `advance` → merged branch that `queue` uses for jobs whose PR merged.
Branch: `feat/job-create-queue` (stacked on `feat/job-waiting-commands`). Delivery: `ask-on-risk`. Test runner: `cargo test` (unit tests with doubles); parity via `rust-anything app replay`.

## Specs

- **S1** (user, L1): "si", answering "¿Sigo con `create` y `queue`?"
- **S2** (standing migration rules): "La v2 original es el oráculo de comportamiento." "No modificar nunca los archivos del oráculo." "Usar dobles de `gh`, repositorios Git temporales y fixtures aislados." "No realizar operaciones contra GitHub real durante las pruebas." "No normalizar salidas salvo timestamps, UUID o valores realmente variables y comprobados ejecutando dos veces el mismo caso." "No usar comodines amplios que puedan esconder divergencias." "Ejecutar cada caso original al menos dos veces." "Primero demostrar que el replay del propio original pasa 100%." "Ejecutar `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`."
- **S3** (user): "tdd , 95% coverage , separar en modulos createes cuando sea necesario , no sobre ingenierai seamos inteligentes , test unitarios con mocks , no hagas integracion"
- **S4** (Owner decision, 2026-11-18): CPython tracebacks are accepted divergences (exit code, stdout and file effects kept; one stable `job:` line).
- **S5** (design, judgement calls): the gh/git doubles answer from `doubles/<tool>/<first three args>.*` when present, else `<first two args>.*` (backward compatible), so one case can answer `gh issue view` per issue number. create/queue cases live in `tests/parity/job-queue` with the `timestamps` normalizer (create writes `started = now()`, queue's auto-merge writes `ended = now()`), after two unnormalized recordings prove the variance. `create <arg>` is owned only when the argument is ASCII digits; other `isdigit()` strings keep going to the v2 fallback, which then behaves exactly like v2.

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S1 S2 S5 | Record create/queue cases (tests/parity/job-queue), double key extension, variance proof, twice, original replay 100% | inline | done | 16 cases; two unnormalized recordings differ only in started/ended; normalized recordings identical; all five originals 100% |
| T2 | S1 S3 | Issue rendering: `fetch_issue`, `write_issue_md`, `quote`, `replied_since_question`, `slugify`, RED→GREEN | inline | done | issue rendering green; found operator() must be asked even when the author matches |
| T3 | S1 S3 S4 | `create`: existing-job and slot checks, issue checks, id numbering, job.json, RED→GREEN | inline | done | create: all recorded cases green |
| T4 | S1 S3 S4 | `queue`: auto-merge (`pr_state`, advance → merged), rows, slots, RED→GREEN | inline | done | queue + advance → merged + pr_state green; DIV-JOB-07 |
| T5 | S2 S4 | Replay every harness vs Rust, gates, coverage, MIGRATION.md | inline | done | job-queue 15/16, job 39/45, job-reject 7/7, route 18/22, gate 22/29; 236 tests; coverage 97.46%; flaky 0/300 |

Route evidence: one sequential behavior, no parallel units; no writer delegated.

## Log

- L1 (2026-11-18, user): "si"
- L2: scope finding before T1: `queue` runs `advance(sync(j), "merged")` for jobs whose PR merged, so this slice also ports the → merged branch and `pr_state`.
- L3 (T1): fixture fixes found by recording (not v2 bugs): `queue` asks `pr view` for every non-terminal job with a PR (needed an answer for PR 9); `create 005` asks `gh issue view 005` first (argument as given) and then `gh issue view 5` (the number GitHub returned) inside write_issue_md. The double now answers by the first three arguments when such files exist. The normalizer also hides fixture comment `createdAt` values in issue.md; unit tests pin them.
- L4 (T2–T5): RED 27 failing (`migration/rust-anything/job-queue/red-cargo-test.txt`), GREEN 236. A test pinned that v2 always asks for the operator when a comment follows a factory question (set literal `{author, operator()}`), a gh call my first version skipped.
