#!/usr/bin/env python3
"""Calls of factory/bin/* named in v2's skill and agent prompts, and how many
the Rust binaries answer (route, gate, and job sync/agents/summary/feedback/
reject/create/queue); the rest go to v2's job.py.

Usage: python3 -I migration/audit/measure_skill.py   (from the repo root)
"""
import re
from collections import Counter
from pathlib import Path

RUST_JOB = {"sync", "agents", "summary", "feedback", "reject", "create", "queue"}
calls = Counter()
for path in sorted(Path(".oracle/v2/.claude").rglob("*.md")):
    for exe, sub in re.findall(r"factory/bin/(job|gate|route)\b ?([a-z]*)", path.read_text()):
        calls[(exe, sub if exe == "job" else "")] += 1
rust = sum(n for (exe, sub), n in calls.items() if exe != "job" or sub in RUST_JOB)
total = sum(n for (exe, sub), n in calls.items() if not (exe == "job" and sub == ""))
for (exe, sub), n in calls.most_common():
    owner = "Rust" if exe != "job" or sub in RUST_JOB else ("(bare mention)" if sub == "" else "Python job.py")
    print(f"{n:3d}  {exe} {sub:<9} {owner}")
print(f"answered by Rust: {rust}/{total} ({100 * rust / total:.1f}%)")
