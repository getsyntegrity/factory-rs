#!/usr/bin/env python3
"""Generate tests/parity/job-advance/inputs.jsonl: v2 oracle cases for every
part of `job` that is still Python: the `advance` transitions not ported yet,
`merge`, and the helpers only they use (`section`, `escalate`,
`check_reviews_done`, `git`).

Recorded with the `timestamps` normalizer (advance writes `ended = now()`
when a job enters a waiting or terminal stage). Helpers come from
migration/rust-anything/job/make_inputs.py.

Usage: python3 migration/rust-anything/job-advance/make_inputs.py
"""

import importlib.util
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("job_inputs", ROOT / "migration/rust-anything/job/make_inputs.py")
J = importlib.util.module_from_spec(spec)
spec.loader.exec_module(J)

OUT = ROOT / "tests/parity/job-advance/inputs.jsonl"
ID = J.ID
SHA = "1111111111111111111111111111111111111111"
OTHER = "2222222222222222222222222222222222222222"
CONFIG = J.CONFIG
REVIEWERS = J.REVIEWERS
ACCEPTED = J.ACCEPTED
OPERATOR = J.OPERATOR


def job(stage, round_=1, round_limit=3, pr=None, intake=ACCEPTED, agents=REVIEWERS, reviews=None, **extra):
    return J.job(stage=stage, round_=round_, intake=intake, agents=agents, pr=pr, worktree="wt",
                 round_limit=round_limit, reviews=reviews or {}, **extra)


def case(job_json, **files):
    fs = {**J.base(job_json), **OPERATOR, "wt/README.md": "worktree\n"}
    for path, text in files.items():
        fs[path if "/" in path and not path.startswith("round") else f"factory/jobs/{ID}/{path}"] = text
    return fs


def answer(tool, key, stdout=None, code=None, stderr=None):
    return J.answer(tool, key, stdout, code, stderr)


def jfile(name):
    return f"factory/jobs/{ID}/{name}"


INTAKE = {
    "NEEDS-INFO": "DECISION: NEEDS-INFO\nRISK: low\nSURFACES: none\n\n## Questions\n- Which page?\n",
    "REJECT": "DECISION: REJECT\nRISK: low\nSURFACES: none\n\n## Why\nOut of scope.\n",
    "ACCEPT": "DECISION: ACCEPT\nRISK: high\nSURFACES: ui, security\n\n## Summary\n- Add it.\n\n## Notes\nmore\n",
}

GIT_READY = {
    **answer("git", "status_--porcelain", ""),
    **answer("git", "rev-list_--count", "2\n"),
    **answer("git", "rev-parse_HEAD", SHA + "\n"),
    **answer("git", f"rev-parse_origin_factory_{ID}", SHA + "\n"),
    **answer("gh", "pr_list", json.dumps([{"number": 7, "url": "https://github.com/acme/app/pull/7"}]) + "\n"),
}

BUILD_READY = "STATUS: READY\n\n## Summary\n- done\n"
BUILD_BLOCKED = "STATUS: BLOCKED\n\n## Summary\n- tried\n\n## Blocked\nThe API key is missing.\n\n## Next\n- ask\n"


def checks_md(gate, commit=SHA):
    return f"GATE: {gate}\nCOMMIT: {commit}\n\n## Summary\n- x\n"


def reviews(**verdicts):
    return {f"round-1/review-{r}.md": f"VERDICT: {v}\n\nnotes\n" for r, v in verdicts.items()}


ALL_PASS = reviews(code="PASS", ux="PASS", ui="PASS", security="PASS")
WITH_CHANGES = reviews(code="CHANGES", ux="PASS", ui="PASS", security="PASS")


def pr_states(*states, draft=False):
    """Answers for successive `gh pr view 7` calls."""
    fs = {}
    for n, state in enumerate(states, 1):
        fs[f"doubles/gh/pr_view_7.{n}.stdout"] = json.dumps({"state": state, "isDraft": draft}) + "\n"
    return fs


MERGE_GIT = {
    **answer("git", "fetch_origin"),
    **answer("git", "merge_--no-edit"),
    **answer("git", "rev-parse_HEAD", SHA + "\n"),
    **answer("git", "diff_--name-status", ""),
    **answer("git", f"push_origin_factory_{ID}"),
    **answer("git", "push_origin_--delete", code=1, stderr="error: unable to delete: remote ref does not exist\n"),
    **answer("gh", "pr_merge"),
    "factory/checks.sh": "#!/usr/bin/env bash\necho checks ok\nexit 0\n",
}
MERGING = job("merging", pr=7, round_=2)


def adv(name, stage, job_json, transition, **files):
    return (name, ["advance", ID, stage], "", case(job_json, **files), transition)


def merge(name, job_json, transition, **files):
    return (name, ["merge", ID], "", case(job_json, **files), transition)


CASES = [
    # ---- intake → needs-info / rejected / spec --------------------------------
    adv("intake-01-needs-info", "needs-info", job("intake", 0), "intake→needs-info", **{"intake.md": INTAKE["NEEDS-INFO"]}),
    adv("intake-02-needs-info-wrong-decision", "needs-info", job("intake", 0), "intake→needs-info",
        **{"intake.md": INTAKE["ACCEPT"]}),
    adv("intake-03-needs-info-no-intake-md", "needs-info", job("intake", 0, intake=None), "intake→needs-info"),
    adv("intake-04-rejected", "rejected", job("intake", 0), "intake→rejected", **{"intake.md": INTAKE["REJECT"]}),
    adv("intake-05-rejected-wrong-decision", "rejected", job("intake", 0), "intake→rejected",
        **{"intake.md": INTAKE["NEEDS-INFO"]}),
    adv("intake-06-spec", "spec", job("intake", 0), "intake→spec",
        **{"intake.md": INTAKE["ACCEPT"], **answer("git", "fetch_origin"), **answer("git", "worktree_add")}),
    adv("intake-07-spec-not-routed", "spec", job("intake", 0, agents=None), "intake→spec", **{"intake.md": INTAKE["ACCEPT"]}),
    adv("intake-08-spec-fetch-fails", "spec", job("intake", 0), "intake→spec",
        **{"intake.md": INTAKE["ACCEPT"], **answer("git", "fetch_origin", code=128, stderr="fatal: no remote\n")}),
    adv("intake-09-spec-body-without-sections", "spec", job("intake", 0), "intake→spec",
        **{"intake.md": "DECISION: ACCEPT\nRISK: low\nSURFACES: none\n", **answer("git", "fetch_origin"),
           **answer("git", "worktree_add")}),
    adv("intake-10-spec-comment-fails", "spec", job("intake", 0), "intake→spec",
        **{"intake.md": INTAKE["ACCEPT"], **answer("gh", "issue_comment", code=1, stderr="HTTP 403\n")}),
    adv("intake-11-illegal", "build", job("intake", 0), "intake→(illegal)", **{"intake.md": INTAKE["ACCEPT"]}),
    # ---- needs-info → intake ------------------------------------------------
    adv("info-01-back-to-intake", "intake", job("needs-info", 0), "needs-info→intake",
        **{"intake.md": INTAKE["NEEDS-INFO"], **J.issue(1, comments=[J.comment_json("alice", "Settings.", "2026-11-18T03:00:00Z")])}),
    adv("info-02-second-time", "intake", job("needs-info", 0), "needs-info→intake",
        **{"intake.md": INTAKE["NEEDS-INFO"], "intake-1.md": "DECISION: NEEDS-INFO\n", **J.issue(1)}),
    adv("info-03-concurrency", "intake", job("needs-info", 0), "needs-info→intake",
        **{"intake.md": INTAKE["NEEDS-INFO"], **J.issue(1),
           **J.existing("002-a", 2, "build"), **J.existing("003-b", 3, "gate"), **J.existing("004-c", 4, "review")}),
    adv("info-04-no-intake-md", "intake", job("needs-info", 0, intake=None), "needs-info→intake", **J.issue(1)),
    # ---- spec → build -------------------------------------------------------
    adv("spec-01-build", "build", job("spec", 0), "spec→build", **{"spec.md": "# Spec\n"}),
    adv("spec-02-no-spec-md", "build", job("spec", 0), "spec→build"),
    # ---- build → gate -------------------------------------------------------
    adv("build-01-gate", "gate", job("build"), "build→gate", **{"build.md": BUILD_READY, **GIT_READY}),
    adv("build-02-not-ready", "gate", job("build"), "build→gate", **{"build.md": "STATUS: WORKING\n", **GIT_READY}),
    adv("build-03-no-build-md", "gate", job("build"), "build→gate", **GIT_READY),
    adv("build-04-dirty", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("git", "status_--porcelain", " M src/app.py\n")}),
    adv("build-05-no-commits", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("git", "rev-list_--count", "0\n")}),
    adv("build-06-not-pushed", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("git", f"rev-parse_origin_factory_{ID}", OTHER + "\n")}),
    adv("build-07-no-remote-branch", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY,
           **answer("git", f"rev-parse_origin_factory_{ID}", code=128, stderr="fatal: bad revision\n")}),
    adv("build-08-no-open-pr", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("gh", "pr_list", "[]\n")}),
    adv("build-09-pr-already-known", "gate", job("build", pr=7), "build→gate", **{"build.md": BUILD_READY, **GIT_READY}),
    adv("build-10-status-fails", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("git", "status_--porcelain", code=128, stderr="fatal: not a git repository\n")}),
    adv("build-11-rev-list-not-a-number", "gate", job("build"), "build→gate",
        **{"build.md": BUILD_READY, **GIT_READY, **answer("git", "rev-list_--count", "many\n")}),
    # ---- build → needs-human (section, escalate, needs-human comment) ---------
    adv("blocked-01-needs-human", "needs-human", job("build"), "build→needs-human", **{"build.md": BUILD_BLOCKED}),
    adv("blocked-02-no-blocked-section", "needs-human", job("build"), "build→needs-human",
        **{"build.md": "STATUS: BLOCKED\n\n## Summary\n- x\n"}),
    adv("blocked-03-draft-pr", "needs-human", job("build", pr=7), "build→needs-human",
        **{"build.md": BUILD_BLOCKED, **pr_states("OPEN", draft=True)}),
    adv("blocked-04-ready-pr", "needs-human", job("build", pr=7), "build→needs-human",
        **{"build.md": BUILD_BLOCKED, **pr_states("OPEN")}),
    adv("blocked-05-not-blocked", "needs-human", job("build"), "build→needs-human", **{"build.md": BUILD_READY}),
    # ---- gate → review / build / needs-human ----------------------------------
    adv("gate-01-review", "review", job("gate"), "gate→review",
        **{"round-1/checks.md": checks_md("PASS"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-02-not-run", "review", job("gate"), "gate→review", **answer("git", "rev-parse_HEAD", SHA + "\n")),
    adv("gate-03-other-commit", "review", job("gate"), "gate→review",
        **{"round-1/checks.md": checks_md("PASS", OTHER), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-04-review-but-failed", "review", job("gate"), "gate→review",
        **{"round-1/checks.md": checks_md("FAIL"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-05-build-next-round", "build", job("gate"), "gate→build",
        **{"round-1/checks.md": checks_md("FAIL"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-06-build-but-last-round", "build", job("gate", round_=3), "gate→build",
        **{"round-3/checks.md": checks_md("FAIL"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-07-needs-human-last-round", "needs-human", job("gate", round_=3), "gate→needs-human",
        **{"round-3/checks.md": checks_md("FAIL"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-08-needs-human-but-passed", "needs-human", job("gate", round_=3), "gate→needs-human",
        **{"round-3/checks.md": checks_md("PASS"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-09-needs-human-too-early", "needs-human", job("gate"), "gate→needs-human",
        **{"round-1/checks.md": checks_md("FAIL"), **answer("git", "rev-parse_HEAD", SHA + "\n")}),
    adv("gate-10-rev-parse-fails", "review", job("gate"), "gate→review",
        **{"round-1/checks.md": checks_md("PASS"), **answer("git", "rev-parse_HEAD", code=128, stderr="fatal: bad\n")}),
    # ---- review → approve / build / needs-human (check_reviews_done) ----------
    adv("review-01-approve", "approve", job("review"), "review→approve", **ALL_PASS),
    adv("review-02-missing-review", "approve", job("review"), "review→approve",
        **reviews(code="PASS", ui="PASS", security="PASS")),
    adv("review-03-invalid-verdict", "approve", job("review"), "review→approve",
        **{**ALL_PASS, "round-1/review-ux.md": "Looks good\n"}),
    adv("review-04-approve-but-changes", "approve", job("review"), "review→approve", **WITH_CHANGES),
    adv("review-05-build-next-round", "build", job("review"), "review→build", **WITH_CHANGES),
    adv("review-06-build-but-all-pass", "build", job("review"), "review→build", **ALL_PASS),
    adv("review-07-needs-human-last-round", "needs-human", job("review", round_=3, pr=7), "review→needs-human",
        **{k.replace("round-1", "round-3"): v for k, v in WITH_CHANGES.items()}, **pr_states("OPEN")),
    adv("review-08-build-but-last-round", "build", job("review", round_=3), "review→build",
        **{k.replace("round-1", "round-3"): v for k, v in WITH_CHANGES.items()}),
    # ---- approve → merging / needs-human ----------------------------------------
    adv("approve-01-merging", "merging", job("approve", pr=7), "approve→merging", **{"decision.md": "APPROVE\n\n## Why\nok\n"}),
    adv("approve-02-merging-but-escalated", "merging", job("approve", pr=7), "approve→merging",
        **{"decision.md": "ESCALATE\n\n## Why\nrisky\n"}),
    adv("approve-03-no-decision", "merging", job("approve", pr=7), "approve→merging"),
    adv("approve-04-needs-human", "needs-human", job("approve", pr=7), "approve→needs-human",
        **{"decision.md": "ESCALATE\n\n## Why\nThe change touches billing.\n", **pr_states("OPEN")}),
    adv("approve-05-needs-human-one-line-decision", "needs-human", job("approve"), "approve→needs-human",
        **{"decision.md": "ESCALATE"}),
    # ---- other transitions through `job advance` --------------------------------
    adv("other-01-merged-without-merged-pr", "merged", job("review", pr=7), "*→merged (advance)", **pr_states("OPEN")),
    adv("other-02-needs-human-to-build-without-feedback", "build", job("needs-human"), "needs-human→build (advance)",
        **{"decision.md": "ESCALATE\n\n## Why\nx\n"}),
    adv("other-03-rejected-without-rejection-md", "rejected", job("needs-human"), "needs-human→rejected (advance)"),
    adv("other-04-terminal-stage", "build", job("merged"), "merged→(illegal)"),
    # ---- merge ------------------------------------------------------------------
    merge("merge-01-already-merged", job("approve", pr=7, round_=2), "merge",
          **{"decision.md": "APPROVE\n\n## Why\nok\n", **pr_states("MERGED", "MERGED")}),
    merge("merge-02-happy-path", MERGING, "merge", **{**MERGE_GIT, **pr_states("OPEN", "OPEN", "MERGED"), "round-2/build.md": "x\n"}),
    merge("merge-03-draft-pr", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN", "MERGED", draft=True), "round-2/build.md": "x\n"}),
    merge("merge-04-conflict", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN"), "round-2/build.md": "x\n",
             **answer("git", "merge_--no-edit", "CONFLICT (content): Merge conflict in app.py\n", code=1),
             "decision.md": "APPROVE\n\n## Why\nok\n"}),
    merge("merge-05-gate-fails", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN"), "round-2/build.md": "x\n",
             "factory/checks.sh": "#!/usr/bin/env bash\necho tests failed\nexit 1\n", "decision.md": "APPROVE\n"}),
    merge("merge-06-push-fails", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN", "OPEN"), "round-2/build.md": "x\n", "decision.md": "APPROVE\n",
             **answer("git", f"push_origin_factory_{ID}", code=1, stderr="rejected: non-fast-forward\n")}),
    merge("merge-07-wrong-stage", job("build"), "merge"),
    merge("merge-08-fetch-fails", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN"), **answer("git", "fetch_origin", code=128, stderr="fatal: offline\n")}),
    merge("merge-09-gh-merge-fails", MERGING, "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN", "OPEN"), "round-2/build.md": "x\n", "decision.md": "APPROVE\n",
             **answer("gh", "pr_merge", code=1, stderr="Pull request is not mergeable\n")}),
    merge("merge-10-from-approve-full", job("approve", pr=7, round_=2), "merge",
          **{**MERGE_GIT, **pr_states("OPEN", "OPEN", "MERGED"), "round-2/build.md": "x\n",
             "decision.md": "APPROVE\n\n## Why\nok\n"}),
]


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    names = [c[0] for c in CASES]
    assert len(names) == len(set(n[:24] for n in names)), "case names must differ in their first 24 characters"
    with OUT.open("w") as f:
        for name, args, stdin, fs, _ in CASES:
            f.write(json.dumps({"name": name, "args": args, "stdin": stdin, "env": {}, "files": fs}) + "\n")
    print(f"wrote {len(CASES)} cases to {OUT.relative_to(ROOT)}")
    for transition, n in sorted(Counter(c[4] for c in CASES).items()):
        print(f"  {n:3d}  {transition}")


if __name__ == "__main__":
    main()
