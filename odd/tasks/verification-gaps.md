# Feature: close verification gaps before porting the rest of advance and merge

Objective: before any new port, push the stacked branches (PRs opened by the user), record v2 oracle cases for everything still in Python, prove the hybrid Rust/Python dispatch end to end (including lock compatibility), audit stderr consumers, write the real-GitHub merge validation plan, and re-audit every claim with its command.
Branch: `feat/verification-gaps` (stacked on `feat/job-create-queue`). Delivery: `ask-on-risk`. Test runner: `cargo test`; parity via `rust-anything app replay`.

## Specs

- **S1** (user, L1): "Antes de seguir con los 4 slices, resolvé los huecos de verificación que siguen. No portes código nuevo hasta terminar los pasos 1–4."
- **S2** (user, step 1): "Pusheá feat/gate, feat/job-waiting-commands y feat/job-create-queue." "Abrí PRs apilados en orden (cada uno con base en el anterior), con descripción que enlace la sección correspondiente de MIGRATION.md. No mergees nada." Amended by the user's answer (L2): push over SSH; the user opens the PRs; no `gh` is run.
- **S3** (user, step 2): "Grabá casos de replay contra v2 (SHA c467603d, mismo mecanismo de .oracle/v2 y dobles) para TODAS las transiciones de advance que siguen en Python: intake→needs-info/rejected/spec, needs-info→intake, spec→build, build→gate/needs-human, gate→*, review→*, approve→*, y el bloque de comentarios de needs-human." "Grabá casos para merge y para section, escalate, check_reviews_done y git." "Incluí caminos de error y crash, no solo happy path. Reportá cantidad de casos por transición." "Verificá que los 5 harnesses originales siguen dando 119/119 y que los nuevos dan 100% contra v2."
- **S4** (user, step 3): "Armá un harness que ejecute un job de punta a punta (create → queue → advance … → gate → review → approve → merge) usando el dispatch REAL que usa la skill hoy (Rust donde está portado, Python donde no), contra dobles de gh/git." "Compará el resultado final (job.json, archivos de round, doubles.log, exit codes) contra la misma secuencia corrida 100% en v2." "Incluí al menos un caso donde Rust escribe job.json y Python lo lee en el paso siguiente, y viceversa." "Corré el harness con concurrencia (dos comandos sobre el mismo job) para verificar que los locks de Python y Rust son compatibles. Reportá corridas y fallos."
- **S5** (user, step 4): "Buscá en SKILL.md, los agentes y los prompts de .claude/ cualquier lugar donde se lea, parsee o se tome una decisión en base a stderr de factory/bin/*." "Si existe alguno, listalo y decime si las 18 divergencias aceptadas lo afectan. No cambies nada; solo reportá."
- **S6** (user, step 5): "Escribí en MIGRATION.md un procedimiento de validación end-to-end de merge contra un repo descartable con gh real, que voy a correr yo manualmente antes de eliminar job.py." "No toques ningún remoto real ni ejecutes gh real en esta sesión."
- **S7** (user, step 6): "Encabezá el veredicto con la métrica de uso real en la skill (llamadas atendidas por Rust / total), luego comportamiento grabado, y al final líneas." "Agregá una sección "Riesgos del modo híbrido" con los resultados del paso 3." "Para cada afirmación, indicá el comando que la verifica. Nada de lo anotado antes vale sin volver a correrlo."
- **S8** (user, rules): "TDD estricto en todo lo nuevo (RED antes de GREEN); si te desviás, anotalo en MIGRATION.md. cargo fmt --check, clippy -D warnings y cargo test tienen que pasar en cada commit." "Si algo de esto contradice una decisión anterior mía documentada, frená y preguntame."

## Tasks

| ID | Specs | Task | Route | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| T1 | S2 | Push the three branches over SSH; write the stacked PR titles, bases and bodies for the user | inline | done | pushed (git branch -vv shows origin tracking); migration/prs/ |
| T2 | S3 | Record v2 cases for every advance transition still in Python, merge, section/escalate/check_reviews_done/git, errors and crashes; originals 100% | inline | done | tests/parity/job-advance: 70 cases, 70/70 vs v2; earlier five 119/119; unnormalized twice differ only in `ended`; normalized twice identical |
| T3 | S4 | Hybrid end-to-end harness vs all-v2, cross-language job.json hand-offs, concurrent lock check | inline | done | tests/parity/e2e: 3 scenarios, 40 steps (18 Rust, 22 Python), hybrid 3/3 ×5; locks both directions PASS + control; 300 races, 0 failures |
| T4 | S5 S6 | stderr-consumer audit (report only) and the real-GitHub merge validation procedure in MIGRATION.md | inline | done | MIGRATION.md "Who reads stderr" (9 consumers, none affected in meaning) and "Merge validation against real GitHub" |
| T5 | S7 | Re-audited report with a verifying command per claim, "Riesgos del modo híbrido" | inline | pending | |

Route evidence: sequential verification work; no writer delegated.

## Log

- L1 (2026-11-18, user): the six-step verification brief quoted in S1–S8.
- L2 (2026-11-18, user answer): "Push SSH, PRs los abrís vos (Recomendado)" — chosen because step 1 contradicted MIGRATION.md:41 ("No push, PR, release or real GitHub call") and step 5 ("ni ejecutes gh real en esta sesión"), and PR creation would need the user's gh session (not authorized). Pushed over SSH: feat/gate, feat/job-waiting-commands, feat/job-create-queue.
- L3 (T2): the job harness now installs the matching `gate` (merge runs factory/bin/gate) and its doubles can answer per call number (`<key>.<n>.*`) so a PR can merge mid-case; both changes left every earlier case unchanged (replays 45/45, 7/7, 16/16). Hybrid replay of job-advance: 67/70; the 3 differences are v2 crashes whose traceback names `factory/bin/job.py` instead of `factory/bin/job` (stderr only).
- L4 (T3): scenario bug found by the first recording: build.md lives in the job folder, not in round-N/ (v2 `first_line(d / "build.md")`). The shared doubles moved to tests/parity/doubles.sh (sourced by the job and e2e launchers); every earlier harness replays unchanged. Harness scripts (shell) are the tests themselves and were not written RED-first; recorded in MIGRATION.md.
