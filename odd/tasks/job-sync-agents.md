# Feature: port `job sync` and `job agents` to Rust

Objective: a Rust `job` binary that answers `job sync <job>` and `job agents <job> <json>` with parity against v2 (oracle c467603d), so `route` no longer needs Python for them.
Branch: `feat/job-sync-agents` (from `develop`). Delivery: `ask-on-risk`. Test runner: `cargo test` (unit tests with doubles, no integration tests), parity via `rust-anything app replay`.

## Specs

- **S1** (user, L1): "arrancá el próximo slice: job sync y agents". The previous slice named it: "Port `job` … Start with `sync` and `agents`, the two subcommands `route` calls, so the `route → job` fallback can go away. The cases will need `gh`/`git` doubles. Also watch `job`'s file lock and the `CLAUDE_CONFIG_DIR` transcript pricing."
- **S2** (standing rules from the migration brief, still in force): "La v2 original es el oráculo de comportamiento." "No modificar nunca los archivos del oráculo." "No realizar operaciones contra GitHub real durante las pruebas." "Un caso desconocido, no grabado o no reproducible no equivale a PASS." "No normalizar salidas salvo timestamps, UUID o valores realmente variables y comprobados ejecutando dos veces el mismo caso." "Ejecutar cada caso original al menos dos veces." "Primero demostrar que el replay del propio original pasa 100%." "Crear tests Rust que consuman los casos grabados." "No debilitar los expectations para obtener verde." "No usar `unwrap`, `expect` o `panic!` para entradas o errores que v2 maneja normalmente." "Ejecutar `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`." "No ejecutar `app lift`."
- **S3** (user): "tdd , 95% coverage , separar en modulos createes cuando sea necesario , no sobre ingenierai seamos inteligentes , test unitarios con mocks , no hagas integracion"
- **S4** (Owner decision, 2026-11-18): "los tracebacks de CPython no forman parte del contrato de compatibilidad de Factory v2." For invalid cases Rust keeps "exit code 1; stdout vacío; ninguna modificación de archivos; un mensaje estable, breve y específico en stderr" and must not "imitar rutas, líneas ni frames de Python; imprimir un traceback; delegar nuevamente a Python; ocultar los … casos; declararlos como paridad byte a byte." Record them as accepted divergences with their own IDs; report "PASS WITH N ACCEPTED DIAGNOSTIC DIVERGENCES".
- **S5** (design decision, judgement call): Rust `job` owns exactly `sync <job>` and `agents <job> <json>`. Any other argv runs the v2 original installed beside it as `factory/bin/job.py` (explicit fallback). Rejected: calling sync/agents in-process from `route`, because v2 takes the job's `flock` once per process and an in-process second lock on a new descriptor would block.

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S1 S2 | Record v2 `job sync`/`job agents` cases (harness `tests/parity/job`, transcript fixtures, twice, original replay 100%) | inline | done | 33 cases; second recording identical; original replay 33/33; oracle clean |
| T2 | S1 S2 S3 S4 | Rust job store + `sync` (header, reviews, gate, cost from transcripts, lock, save as Python `json.dumps(indent=2)`), RED→GREEN | inline | done | RED 49 failing (`migration/rust-anything/job/red-cargo-test.txt`), GREEN 106/106 unit tests incl. every recorded sync/agents case |
| T3 | S1 S3 S4 S5 | `agents` + `job` binary dispatch with `job.py` fallback, RED→GREEN | inline | pending | |
| T4 | S2 S4 S5 | Replay job cases and route cases against Rust (route + job), gates, coverage, MIGRATION.md | inline | pending | |

Route evidence: every task is one sequential unit of the same behavior, no parallel units, so no writer is delegated.

## Log

- L1 (2026-11-18, user): "arrancá el próximo slice: job sync y agents"
- L2: Rust toolchain found at 1.100.0 (was 1.96.0 at the start of the route slice; not changed by this session's commands in this turn). Gates will run on 1.100.0.
- L3 (T1): 33 cases recorded (32 inputs + tool-added example-001). Findings: pathlib glob in Python 3.12 includes hidden dirs (`.hidden/subagents` is priced); sync-15 saves job.json and then crashes with KeyError 'reviews' (crash after the write); header parsing stops at a whitespace-only line; tracebacks in agents-05, sync-15, sync-16, sync-17, example-001.
- L4 (T2): sync, cost and the agents rules ported; unit tests load the recorded cases' inputs and expected stdout/stderr/exit/job.json. Faithfulness fixes found while porting: `job["agents"] and job["stage"] in ...` and `stage != "intake" or ...` short-circuit, `j["issue"]` is a hard key access.
