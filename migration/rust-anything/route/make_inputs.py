#!/usr/bin/env python3
"""Generate tests/parity/route/inputs.jsonl, the recorded cases for `route`.

Each line follows the `rust-anything app record --inputs` schema (v1.5.1):
{"name", "args": [...], "stdin", "env": {...}, "files": {path: text}}.
Fixtures are built from the frozen oracle's factory/config.json.

Usage: python3 migration/rust-anything/route/make_inputs.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONFIG = (ROOT / ".oracle/v2/factory/config.json").read_text()
OUT = ROOT / "tests/parity/route/inputs.jsonl"

JOB_ID = "001-add-a-thing"


def job_json(job_id=JOB_ID, issue=1, intake=None):
    job = {
        "id": job_id,
        "issue": issue,
        "title": "Add a thing",
        "issue_url": None,
        "pr": None,
        "pr_url": None,
        "branch": f"factory/{job_id}",
        "worktree": None,
        "stage": "intake",
        "round": 0,
        "round_limit": 3,
        "intake": intake,
        "agents": None,
        "gate": None,
        "reviews": {},
        "cost_usd": 0.0,
        "started": "2026-11-18T00:00:00Z",
        "ended": None,
    }
    return json.dumps(job, indent=2) + "\n"


def intake_md(risk="low", surfaces="none", decision="ACCEPT"):
    lines = [f"DECISION: {decision}"]
    if risk is not None:
        lines.append(f"RISK: {risk}")
    lines += [f"SURFACES: {surfaces}", "RESCOPED: no", "", "## Summary", "- fixture", ""]
    return "\n".join(lines)


def files(intake=True, config=CONFIG, job_id=JOB_ID, issue=1, **intake_args):
    fs = {}
    if config is not None:
        fs["factory/config.json"] = config
    fs[f"factory/jobs/{job_id}/job.json"] = job_json(job_id, issue)
    if intake:
        fs[f"factory/jobs/{job_id}/intake.md"] = intake_md(**intake_args)
    return fs


SYNC_FAILS = """#!/bin/sh
# Double for factory/bin/job: `sync` fails.
echo "double: sync $*" >&2
echo "sync exploded" >&2
exit 4
"""

AGENTS_FAILS = """#!/bin/sh
# Double for factory/bin/job: `sync` passes, `agents` fails and echoes its argv.
case "$1" in
  sync) echo "$2: intake round 0 gate -"; exit 0 ;;
  agents) printf 'agents refused: [%s] [%s]\\n' "$2" "$3" >&2; exit 3 ;;
esac
exit 9
"""

CASES = [
    ("01-no-args", [], files()),
    ("02-two-args", [JOB_ID, "extra"], files()),
    ("03-job-missing", ["999-no-such-job"], files()),
    ("04-no-intake", [JOB_ID], files(intake=False)),
    ("05-low-no-surfaces", [JOB_ID], files(risk="low")),
    ("06-medium-no-surfaces", [JOB_ID], files(risk="medium")),
    ("07-high-no-surfaces", [JOB_ID], files(risk="high")),
    ("08-surface-ui", [JOB_ID], files(risk="low", surfaces="ui")),
    ("09-surface-security", [JOB_ID], files(risk="medium", surfaces="security")),
    ("10-surfaces-ui-security", [JOB_ID], files(risk="high", surfaces="ui, security")),
    ("11-unknown-risk", [JOB_ID], files(risk="extreme")),
    ("12-unknown-surface", [JOB_ID], files(risk="low", surfaces="ui, api")),
    ("13-sync-fails", [JOB_ID], {**files(), "factory/bin/job": SYNC_FAILS}),
    ("14-agents-fails", [JOB_ID], {
        **files(risk="high", surfaces="ui, security"),
        "factory/jobs/001-add-a-thing/job.json": job_json(intake={
            "decision": "ACCEPT", "rescoped": False, "risk": "high",
            "surfaces": ["ui", "security"]}),
        "factory/bin/job": AGENTS_FAILS,
    }),
    ("15a-invalid-chars-id", ["bad id/../x"], files()),
    ("15b-issue-number-id", ["1"], files()),
    ("16a-config-missing", [JOB_ID], files(config=None)),
    ("16b-config-invalid", [JOB_ID], files(config="{ not json\n")),
    ("17-intake-rejected", [JOB_ID], files(decision="REJECT")),
    ("18-intake-without-risk", [JOB_ID], files(risk=None)),
    ("19-quote-in-risk", [JOB_ID], files(risk="it's")),
]


def main():
    with OUT.open("w") as f:
        for name, args, fs in CASES:
            f.write(json.dumps({"name": name, "args": args, "stdin": "", "env": {}, "files": fs}) + "\n")
    print(f"wrote {len(CASES)} cases to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
