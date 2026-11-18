# Feature: verification follow-ups before slice 1

Objective: place feat/verification-gaps in the stack, analyse `gate --hook` fail-open behaviour, extend the real-GitHub merge procedure (two runs, captured fixtures), document the lock primitives, and split AUDIT.md's numbers — without changing production behaviour.
Branch: `feat/verification-gaps` (local commits after the step-1 push). Follow-up spec: `odd/tasks/advance-intake.md` (slice 1, step 6 of the brief).

## Specs

- **S1** (user): "No cambies comportamiento de producción en los pasos 1–5: cualquier hallazgo se documenta, no se corrige."
- **S2** (step 1): "Pusheá feat/verification-gaps por SSH." "Agregá a migration/prs/ un PR para esta rama, ubicado ANTES de los 4 slices pendientes (que entre a develop primero). Si hoy está apilada sobre otra rama, documentá base y orden final de la pila en migration/prs/README.md. No abras PRs ni ejecutes gh."
- **S3** (step 2): "Determiná en qué evento(s) de Claude Code está registrado gate --hook (settings de .claude/, SKILL.md, agentes) y qué matcher usa." "Para cada caso grabado de gate --hook (incluidas DIV-GATE-01…07), tabulá: causa, exit code en v2, exit code en Rust, y efecto según el evento: bloquea / no bloquea / stderr a Claude / stderr solo al usuario." "Marcá explícitamente cada caso donde un error del gate termina dejando pasar la acción (fail-open). Escribilo en MIGRATION.md como decisión pendiente de Owner: "fail-closed (exit 2) cuando el gate no puede evaluar". NO lo cambies: la paridad con v2 se mantiene hasta que yo decida."
- **S4** (step 3): "Actualizá el procedimiento en MIGRATION.md para que corra DOS veces: a) ahora, como línea base (v2 vs híbrido, donde merge sigue en Python); b) después del slice 4, con merge en Rust." "Agregá al procedimiento la captura de las respuestas reales de gh (stdout, stderr, exit code de cada invocación) a un directorio de fixtures, con el formato que usan los dobles, y los pasos para incorporarlas como fixtures de los dobles y re-correr el replay." "Indicá qué datos hay que sanear antes de commitear las fixtures (tokens, URLs privadas, usuarios). No ejecutes nada de esto."
- **S5** (step 4): "Documentá qué primitiva de lock usa cada lado (Python: módulo y llamada exacta; Rust: crate y llamada exacta), sobre qué archivo, y si es advisory." "Indicá en qué SO corrió la prueba de 300 carreras y si la interoperabilidad depende del SO. Si depende, dejá en MIGRATION.md el comando para repetir la prueba en otro SO."
- **S6** (step 5): "Separá en líneas distintas: (a) v2 contra sus grabaciones: 192/192; (b) Rust en comandos portados: 101/119 + 18 divergencias; (c) híbrido contra el oráculo: 67/70 + HYB-01…03, con la aclaración de que difieren solo en la ruta de job.py en el traceback; (d) ciclos 3/3." "Agregá la tabla del paso 2 y el riesgo fail-open en "Riesgos del modo híbrido"." "Re-corré run_audit.sh y actualizá run_audit.out."

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S2 | Push feat/verification-gaps; PR 4 and the final stack order in migration/prs/ | inline | done | pushed (`git branch -vv`); `git merge-base --is-ancestor feat/job-create-queue feat/verification-gaps` exits 0 |
| T2 | S1 S3 | gate --hook event/matcher and per-case effect table, fail-open marked, Owner decision recorded | inline | done | MIGRATION.md "gate --hook: event, matcher and fail-open"; exit codes re-run with launch.sh rust --hook; docs read via a claude-code-guide subagent |
| T3 | S1 S4 | Merge procedure: two runs, gh capture to doubles-format fixtures, sanitising list | inline | pending | |
| T4 | S1 S5 | Lock primitives (verified with strace), OS, repeat command | inline | done | MIGRATION.md "Lock primitives": identical openat+flock(LOCK_EX) observed on both; Linux Mint 22.3 / 7.0.0-38 |
| T5 | S6 | AUDIT.md split lines, hook table + fail-open risk, re-run run_audit.sh | inline | pending | |

## Log

- L1 (2026-11-18, user): steps 1–6 brief quoted in S1–S6; step 6 tracked in odd/tasks/advance-intake.md.
- L2: "los 4 slices pendientes" read as the four future slices (advance intake, build, gate/review/approve, merge); PR 4 goes after PRs 0–3, which it depends on, and before those slices.
- L3: after the step-1 push, later commits on feat/verification-gaps stay local (the brief allows no other remote operation).
- L4 (T2): Claude Code docs (via subagent): frontmatter Stop → SubagentStop, no matcher = all, exit 2 blocks, other non-zero = non-blocking, a hook that cannot start is non-blocking, untrusted folder skips frontmatter hooks. Fail-open cases: DIV-GATE-04/05 and unrecorded error paths; not changed (S1).
