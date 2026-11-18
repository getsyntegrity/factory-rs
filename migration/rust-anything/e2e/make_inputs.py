#!/usr/bin/env python3
"""Generate tests/parity/e2e/inputs.jsonl: fixtures for the end-to-end
scenarios in tests/parity/e2e/scenarios/. A case's only argument is the
scenario name; the scenario runs the real command sequence.

Recorded with the `timestamps` normalizer (job.json `started`/`ended`).

Usage: python3 migration/rust-anything/e2e/make_inputs.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONFIG = (ROOT / ".oracle/v2/factory/config.json").read_text()
OUT = ROOT / "tests/parity/e2e/inputs.jsonl"
ID = "001-add-dark-mode-toggle"
SHA = "3333333333333333333333333333333333333333"
MARKER = "<!-- factory -->"


def answer(tool, key, stdout=None, code=None, stderr=None):
    fs = {}
    if stdout is not None:
        fs[f"doubles/{tool}/{key}.stdout"] = stdout
    if code is not None:
        fs[f"doubles/{tool}/{key}.code"] = f"{code}\n"
    if stderr is not None:
        fs[f"doubles/{tool}/{key}.stderr"] = stderr
    return fs


ISSUE = json.dumps({
    "number": 5, "title": "Add dark mode toggle", "body": "Please add a toggle.",
    "author": {"login": "alice"}, "labels": [{"name": "factory"}], "state": "OPEN",
    "url": "https://github.com/acme/app/issues/5",
    "comments": [
        {"author": {"login": "factory-operator"}, "body": MARKER + "\nWhich page?", "createdAt": "2026-11-18T01:00:00Z"},
        {"author": {"login": "alice"}, "body": "The settings page.", "createdAt": "2026-11-18T02:00:00Z"},
    ],
}) + "\n"

CHECKS = """#!/usr/bin/env bash
# Runs in the job's worktree (.worktrees/<id>); fails while the scenario
# leaves .fail-checks at the repository root.
if [ -e ../../.fail-checks ]; then
  echo "tests failed"
  exit 1
fi
echo "all checks passed"
"""

COMMON = {
    "factory/config.json": CONFIG,
    "factory/checks.sh": CHECKS,
    **answer("gh", "api_user", "factory-operator\n"),
    **answer("gh", "issue_view_5", ISSUE),
    **answer("gh", "issue_list", json.dumps([{"number": 5, "title": "Add dark mode toggle"}]) + "\n"),
    **answer("gh", "pr_list", json.dumps([{"number": 7, "url": "https://github.com/acme/app/pull/7"}]) + "\n"),
    **answer("git", "status_--porcelain", ""),
    **answer("git", "rev-list_--count", "1\n"),
    **answer("git", "rev-parse_HEAD", SHA + "\n"),
    **answer("git", f"rev-parse_origin_factory_{ID}", SHA + "\n"),
    **answer("git", "diff_--name-status", ""),
    **answer("git", "push_origin_--delete", code=1, stderr="error: unable to delete: remote ref does not exist\n"),
}


def pr_view(*states, draft=False):
    return {f"doubles/gh/pr_view_7.{n}.stdout": json.dumps({"state": s, "isDraft": draft}) + "\n"
            for n, s in enumerate(states, 1)}


CASES = [
    ("full-cycle", {**COMMON, **pr_view("OPEN", "OPEN", "OPEN", "MERGED", draft=True)}),
    ("waiting-loop", COMMON),
    ("rework-rounds", {**COMMON, **answer("gh", "pr_view_7", json.dumps({"state": "OPEN", "isDraft": True}) + "\n")}),
]


def main():
    with OUT.open("w") as f:
        for name, fs in CASES:
            f.write(json.dumps({"name": name, "args": [name], "stdin": "", "env": {}, "files": fs}) + "\n")
    print(f"wrote {len(CASES)} scenarios to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
