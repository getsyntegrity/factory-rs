#!/usr/bin/env python3
"""Lines of v2 executable code (function bodies of job, gate, route; blank
and comment lines excluded) with a Rust counterpart.

Usage: python3 -I migration/audit/measure_lines.py   (from the repo root)
"""
import ast
from pathlib import Path

V2 = Path(".oracle/v2/factory/bin")
# job functions fully ported to Rust (src/job/*), by v2 name.
PORTED = {"die", "now", "run", "gh", "gh_json", "operator", "job_dir", "load", "save", "all_jobs", "running",
          "require_slot", "resolve", "first_line", "header", "round_dir", "reviewers", "quote", "fetch_issue",
          "write_issue_md", "replied_since_question", "transcripts_dir", "prompt_text", "cost_usd", "sync",
          "require", "pr_state", "next_round", "slugify", "cmd_create", "cmd_queue", "comment", "label",
          "cmd_feedback", "cmd_reject", "cmd_agents", "elapsed", "cmd_summary"}
# advance: line ranges ported (legality + setup, → merged, waiting → rejected,
# needs-human → build, terminal worktree removal and stage/ended).
ADVANCE_PORTED = [(367, 373), (375, 379), (381, 391), (474, 479), (500, 505)]
MAIN_UNPORTED = (692, 697)  # the `advance` and `merge` dispatch branches


def code(src, a, b):
    return sum(1 for l in src.splitlines()[a - 1:b] if l.strip() and not l.strip().startswith("#"))


def functions(path):
    src = path.read_text()
    return src, {n.name: (n.lineno, n.end_lineno) for n in ast.parse(src).body if isinstance(n, ast.FunctionDef)}


src, fns = functions(V2 / "job")
total = done = 0
unported = []
for name, (a, b) in fns.items():
    n = code(src, a, b)
    total += n
    if name in PORTED:
        d = n
    elif name == "advance":
        d = sum(code(src, x, y) for x, y in ADVANCE_PORTED)
        print(f"job advance: {d}/{n} ({100 * d / n:.0f}%)")
    elif name == "main":
        d = n - code(src, *MAIN_UNPORTED)
    else:
        d = 0
        unported.append(name)
    done += d
print(f"job: {done}/{total} ({100 * done / total:.1f}%); not ported: {', '.join(unported)}")
all_done, all_total = done, total
for exe in ("gate", "route"):
    s, f = functions(V2 / exe)
    n = sum(code(s, a, b) for a, b in f.values())
    all_done += n
    all_total += n
    print(f"{exe}: {n}/{n} (100%)")
print(f"ALL: {all_done}/{all_total} ({100 * all_done / all_total:.1f}%)")
