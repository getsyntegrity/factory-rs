# Feature: port `gate` to Rust

Objective: a Rust `gate` binary with parity against v2 `factory/bin/gate` (oracle c467603d): `gate <job>`, `gate <job> --merge` and `gate --hook`.
Branch: `feat/gate` (stacked on `feat/job-sync-agents`). Delivery: `ask-on-risk`. Test runner: `cargo test` (unit tests with doubles, no integration tests); parity via `rust-anything app replay`.

## Specs

- **S1** (user, L1): "si avanzar con la migracion", answering the proposal "¿Arranco con `gate`?" The proposed slice: "`gate` … cierra el set de binarios y destraba `merge`. Necesita el doble de `git`, un `checks.sh` falso y el normalizador de segundos."
- **S2** (standing migration rules): "La v2 original es el oráculo de comportamiento." "No modificar nunca los archivos del oráculo." "Usar dobles de `gh`, repositorios Git temporales y fixtures aislados." "No normalizar salidas salvo timestamps, UUID o valores realmente variables y comprobados ejecutando dos veces el mismo caso." "Ejecutar cada caso original al menos dos veces." "Primero demostrar que el replay del propio original pasa 100%." "No debilitar los expectations para obtener verde." "Ejecutar `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`." "No ejecutar `app lift`."
- **S3** (user): "tdd , 95% coverage , separar en modulos createes cuando sea necesario , no sobre ingenierai seamos inteligentes , test unitarios con mocks , no hagas integracion"
- **S4** (Owner decision, 2026-11-18): CPython tracebacks are not part of the v2 contract; Rust keeps v2's exit code, stdout and file effects, prints one stable `gate:` line, and each such case becomes an accepted divergence with its own ID (DIV-GATE-*).
- **S5** (design, judgement call): `git` is a PATH double that replays canned output per case and checks its argv; a real temporary repository was rejected because commit SHAs (printed by gate) depend on commit dates. The checks run time is printed as whole seconds; fixtures keep `checks.sh` well under one second, so no normalizer is added unless two recordings differ.

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S1 S2 S5 | Record v2 gate cases (harness `tests/parity/gate`, git double, checks.sh fixtures, hook stdin), twice, original replay 100% | inline | done | 29 cases; second recording identical (seconds stable at 0s, no normalizer); original 29/29; oracle clean |
| T2 | S1 S3 S4 | Rust gate core: argv, round check, protected paths (fnmatch), checks run, checks.md/log, merge variant, RED→GREEN | inline | pending | |
| T3 | S1 S3 S4 | `--hook`, binary, replay vs Rust, gates, coverage, MIGRATION.md | inline | pending | |

Route evidence: one sequential behavior, no parallel units, so no writer is delegated.

## Log

- L1 (2026-11-18, user): "si avanzar con la migracion"
- L2 (T1): first recording exposed a fixture bug (bash double quotes printed `\n` literally); fixed with `$'...'` quoting, verified by running the generated checks.sh, re-recorded from scratch. v2 facts pinned: captured output is read with universal newlines (CRLF in checks.log becomes LF); fnmatch `*` crosses `/`, `?`, `[ab]`, `[!x]`; renames/copies report the old path; `T` (type change) counts; `A` is skipped; bash's own error lands in checks.log with the sandbox path (rust-anything shows it as [CWD]). Tracebacks: example-001, gate-11, gate-12, gate-13, hook-04, hook-05, merge-03 (job `-weird` missing).
