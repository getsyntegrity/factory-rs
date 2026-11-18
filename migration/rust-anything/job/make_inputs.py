#!/usr/bin/env python3
"""Generate tests/parity/job/inputs.jsonl, the recorded cases for `job sync`
and `job agents` (plus a few argv shapes that the Rust job hands to its v2
fallback).

Each line follows the `rust-anything app record --inputs` schema (v1.5.1):
{"name", "args": [...], "stdin", "env": {...}, "files": {path: text}}.
Transcript fixtures live under claude-config/projects/REPO/; launch.sh moves
them to the path job derives from the sandbox and replaces @REPO@ in them.

`reject` cases go to tests/parity/job-reject/inputs.jsonl: they are recorded
with the `timestamps` normalizer because reject writes `ended = now()`.

Usage: python3 migration/rust-anything/job/make_inputs.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONFIG = (ROOT / ".oracle/v2/factory/config.json").read_text()
OUT = ROOT / "tests/parity/job/inputs.jsonl"
REJECT_OUT = ROOT / "tests/parity/job-reject/inputs.jsonl"

ID = "001-add-a-thing"


def job(job_id=ID, issue=1, stage="intake", round_=0, intake=None, agents=None, **extra):
    data = {
        "id": job_id,
        "issue": issue,
        "title": "Add a thing",
        "issue_url": None,
        "pr": None,
        "pr_url": None,
        "branch": f"factory/{job_id}",
        "worktree": None,
        "stage": stage,
        "round": round_,
        "round_limit": 3,
        "intake": intake,
        "agents": agents,
        "gate": None,
        "reviews": {},
        "cost_usd": 0.0,
        "started": "2026-11-18T00:00:00Z",
        "ended": None,
    }
    data.update(extra)
    return json.dumps(data, indent=2) + "\n"


ACCEPTED = {"decision": "ACCEPT", "rescoped": False, "risk": "low", "surfaces": []}
REVIEWERS = {"builder": "sonnet", "reviewers": {"code": "sonnet", "ux": "sonnet", "ui": "haiku", "security": "opus"}}


def base(job_json=None, job_id=ID, config=CONFIG, **more):
    fs = {} if config is None else {"factory/config.json": config}
    fs[f"factory/jobs/{job_id}/job.json"] = job_json if job_json is not None else job(job_id)
    fs.update({f"factory/jobs/{job_id}/{k}": v for k, v in more.items()})
    return fs


def transcript(job_id, *entries, first=None):
    if first is None:
        first = {"type": "user", "message": {"role": "user",
                 "content": f"Job folder: @REPO@/factory/jobs/{job_id}/\nBuild it."}}
    return "".join(json.dumps(e) + "\n" for e in (first, *entries))


def usage(msg_id, model, **u):
    return {"type": "assistant", "message": {"id": msg_id, "model": model, "usage": u}}


def tpath(session, agent):
    return f"claude-config/projects/REPO/{session}/subagents/agent-{agent}.jsonl"


COST_FILES = {
    tpath("s1", "a1"): transcript(
        ID,
        usage("m1", "claude-sonnet-5-5", input_tokens=10000, output_tokens=5000,
              cache_creation_input_tokens=20000, cache_read_input_tokens=100000,
              cache_creation={"ephemeral_1h_input_tokens": 5000}),
        {"type": "user", "message": {"content": "tool result"}},
        usage("m1", "claude-sonnet-5-5", input_tokens=10000, output_tokens=6000,
              cache_creation_input_tokens=20000, cache_read_input_tokens=100000,
              cache_creation={"ephemeral_1h_input_tokens": 5000}),
        usage("m2", "claude-haiku-5-5", input_tokens=333, output_tokens=777),
        usage("m3", "claude-unknown-9", input_tokens=0),
        {"type": "assistant", "message": {"id": "m4", "model": "claude-opus-5-5", "usage": {}}},
    ),
    tpath("s2", "b2"): transcript(
        ID,
        usage("m9", "claude-opus-5-5", input_tokens=1234, output_tokens=4321, cache_read_input_tokens=7),
        first={"type": "user", "message": {"content": [
            {"type": "text", "text": "Context."}, "not a block",
            {"type": "text", "text": f"Job folder: @REPO@/factory/jobs/{ID}/ go"}]}},
    ),
    tpath("s3", "c3"): transcript("002-other", usage("m5", "claude-opus-5-5", input_tokens=10**9)),
    tpath("s4", "empty"): "",
    "claude-config/projects/REPO/s5/other/agent-x.jsonl": transcript(ID, usage("m6", "claude-opus-5-5", input_tokens=10**9)),
    "claude-config/projects/REPO/.hidden/subagents/agent-h.jsonl": transcript(ID, usage("m7", "claude-opus-5-5", input_tokens=10**9)),
}

INTAKE_FULL = "DECISION: ACCEPT\nRISK: high\nSURFACES: ui, security\nRESCOPED: yes\n\n## Summary\n- x\n"
INTAKE_ODD = "decision : REJECT \nsurfaces: none\nno colon here\nrescoped: YES\n   \nRISK: low\n"
INTAKE_CRLF = "DECISION: ACCEPT\r\nRISK: medium\r\nSURFACES: ui,  , none, security\r\n\r\nrest\r\n"

REVIEW_JOB = job(stage="review", round_=1, intake=ACCEPTED, agents=REVIEWERS)
ROUND_FILES = {
    "round-1/checks.md": "GATE: PASS\nCHECKS: 3\n",
    "round-1/review-code.md": "VERDICT: PASS\n\nlooks fine\n",
    "round-1/review-ux.md": "  VERDICT:CHANGES  \nfix it\n",
    "round-1/review-security.md": "VERDICT: pass\n",
}

CASES = [
    # ---- sync -----------------------------------------------------------
    ("sync-01-fresh-job", ["sync", ID], base()),
    ("sync-02-intake-full", ["sync", ID], base(**{"intake.md": INTAKE_FULL})),
    ("sync-03-intake-odd-header", ["sync", ID], base(**{"intake.md": INTAKE_ODD})),
    ("sync-04-intake-crlf", ["sync", ID], base(**{"intake.md": INTAKE_CRLF})),
    ("sync-05-review-round", ["sync", ID], base(REVIEW_JOB, **ROUND_FILES)),
    ("sync-06-build-stage-keeps-reviews", ["sync", ID],
     base(job(stage="build", round_=1, intake=ACCEPTED, agents=REVIEWERS, reviews={"code": "PASS"}), **ROUND_FILES)),
    ("sync-07-round-without-checks", ["sync", ID],
     base(job(stage="review", round_=2, intake=ACCEPTED, agents=REVIEWERS), **ROUND_FILES)),
    ("sync-08-review-cr-only", ["sync", ID],
     base(REVIEW_JOB, **{"round-1/review-code.md": "VERDICT: CHANGES\rmore\r", "round-1/checks.md": "GATE: FAIL\r\n"})),
    ("sync-09-by-issue-number", ["sync", "2"],
     {**base(), **base(job("002-second", issue=2), job_id="002-second"),
      **base(job("003-third", issue=2), job_id="003-third")}),
    ("sync-10-issue-not-found", ["sync", "99"], base()),
    ("sync-11-job-missing", ["sync", "999-nope"], base()),
    ("sync-12-cost-from-transcripts", ["sync", ID], {**base(), **COST_FILES}),
    ("sync-13-cost-unknown-model", ["sync", ID],
     {**base(), tpath("s1", "a1"): transcript(ID, usage("m1", "claude-mystery-1", output_tokens=5))}),
    ("sync-14-json-formatting", ["sync", ID],
     base(job(title="Añadir \"cosa\" ✓ 😀", extra=[1e16, 100.0, 1e-05, 0.5, -0.0, 123456789012, True, None, {}, []]))),
    ("sync-15-keys-appended-in-order", ["sync", ID],
     base(json.dumps({"id": ID, "issue": 1, "stage": "intake", "round": 0, "agents": None}) + "\n",
          **{"intake.md": INTAKE_FULL})),
    ("sync-16-job-json-invalid", ["sync", ID], base("{ broken\n")),
    ("sync-17-config-missing", ["sync", ID], base(config=None)),
    ("sync-18-invalid-chars-id", ["sync", "bad id/../x"], base()),
    # ---- agents ---------------------------------------------------------
    ("agents-01-accepted", ["agents", ID, '{"reviewers": {"code": "haiku"}}'],
     base(**{"intake.md": "DECISION: ACCEPT\nRISK: low\nSURFACES: none\n"})),
    ("agents-02-not-intake-stage", ["agents", ID, '{"reviewers": {}}'],
     base(job(stage="spec", intake=ACCEPTED))),
    ("agents-03-rejected", ["agents", ID, '{"reviewers": {}}'],
     base(**{"intake.md": "DECISION: REJECT\n"})),
    ("agents-04-no-intake", ["agents", ID, '{"reviewers": {}}'], base()),
    ("agents-05-invalid-json", ["agents", ID, "{nope"],
     base(**{"intake.md": "DECISION: ACCEPT\n"})),
    ("agents-06-spec-formatting", ["agents", ID,
     '{"reviewers":{"code":"sonnet","ux":"sonnet"},"note":"ñ","n":[1,2.50,1e3]}'],
     base(**{"intake.md": "DECISION: ACCEPT\n"})),
    ("agents-07-by-issue-number", ["agents", "1", '{"reviewers": {"code": "opus"}}'],
     base(**{"intake.md": "DECISION: ACCEPT\n"})),
    ("agents-08-job-missing", ["agents", "999-nope", "{}"], base()),
    # ---- handed to the v2 fallback by the Rust job ------------------------
    ("fallback-01-no-args", [], base()),
    ("fallback-02-help", ["--help"], base()),
    ("fallback-03-sync-two-args", ["sync", ID, "extra"], base()),
    ("fallback-04-agents-one-arg", ["agents", ID], base()),
    ("fallback-05-unknown-command", ["frobnicate"], base()),
    ("fallback-06-summary-ended-job", ["summary", ID],
     base(job(stage="merged", round_=1, ended="2026-11-18T01:05:00Z"))),
]


# ---- summary, feedback, reject --------------------------------------------

OPERATOR = {"doubles/gh/api_user.stdout": "factory-operator\n"}


def ended_job(job_id, issue, stage, round_, pr, started, ended, **extra):
    return job(job_id, issue=issue, stage=stage, round_=round_, pr=pr, started=started, ended=ended, **extra)


SUMMARY_JOBS = {
    **base(ended_job(ID, 1, "merged", 2, 7, "2026-11-18T00:00:00Z", "2026-11-18T01:05:00Z")),
    **base(ended_job("002-second", 12, "rejected", 0, None, "2026-11-18T00:00:00Z", "2026-11-18T00:59:59Z"),
           job_id="002-second"),
    **base(ended_job("003-a-much-longer-job-id", 3, "merged", 1, 1234, "2026-11-17T00:00:00Z",
                     "2026-11-18T01:00:00Z"), job_id="003-a-much-longer-job-id"),
    **base(ended_job("004-instant", 4, "rejected", 0, 0, "2026-11-18T00:00:00Z", "2026-11-18T00:00:30Z"),
           job_id="004-instant"),
}
SUMMARY_COST = {tpath("s1", "a1"): transcript(
    "002-second", usage("m1", "claude-sonnet-5-5", input_tokens=10000, output_tokens=6000,
                        cache_creation_input_tokens=20000, cache_read_input_tokens=100000,
                        cache_creation={"ephemeral_1h_input_tokens": 5000}))}

DECISION = "ESCALATE\n\n## Why\nThe builder is blocked.\n"


def waiting_job(stage="needs-human", round_=2, pr=None, worktree=".worktrees/001-add-a-thing", **extra):
    return job(stage=stage, round_=round_, pr=pr, worktree=worktree, intake=ACCEPTED, agents=REVIEWERS,
               ended="2026-11-18T02:00:00Z", **extra)


def feedback_case(job_json=None, decision=True, **more):
    fs = {**base(job_json or waiting_job()), **OPERATOR}
    if decision:
        fs[f"factory/jobs/{ID}/decision.md"] = DECISION
    fs.update(more)
    return fs


CASES += [
    ("summary-01-no-jobs", ["summary"], {"factory/config.json": CONFIG}),
    ("summary-02-all-jobs", ["summary"], {**SUMMARY_JOBS, **SUMMARY_COST}),
    ("summary-03-selected-jobs", ["summary", "12", ID], SUMMARY_JOBS),
    ("summary-04-unknown-job", ["summary", ID, "999-nope"], SUMMARY_JOBS),
]

FEEDBACK_CASES = [
    ("feedback-01-sends-back", ["feedback", ID], "  Please split the PR.\n\nThanks.  \n", feedback_case()),
    ("feedback-02-second-feedback", ["feedback", ID], "Again.\n",
     feedback_case(**{f"factory/jobs/{ID}/feedback-1.md": "# Feedback 1\n\nold\n"})),
    ("feedback-03-wrong-stage", ["feedback", ID], "x\n", feedback_case(waiting_job(stage="build"))),
    ("feedback-04-empty-stdin", ["feedback", ID], " \n\t\n", feedback_case()),
    ("feedback-05-operator-fails", ["feedback", ID], "x\n",
     {**feedback_case(), "doubles/gh/api_user.code": "1\n", "doubles/gh/api_user.stderr": "HTTP 401: Bad credentials\n"}),
    ("feedback-06-label-fails", ["feedback", ID], "x\n",
     {**feedback_case(), "doubles/gh/issue_edit.code": "1\n", "doubles/gh/issue_edit.stderr": "label not found\n"}),
    ("feedback-07-no-decision", ["feedback", ID], "x\n", feedback_case(decision=False)),
    ("feedback-08-by-issue", ["feedback", "1"], "By issue.\n", feedback_case()),
]

REJECT_CASES = [
    ("reject-01-needs-info", ["reject", ID], "Out of scope.\n",
     {**base(waiting_job(stage="needs-info", round_=0)), **OPERATOR}),
    ("reject-02-needs-human-with-pr", ["reject", ID], "  Too risky.\n\nSee thread. \n",
     {**base(waiting_job(pr=7, worktree="wt")), **OPERATOR, "wt/README.md": "worktree\n",
      f"factory/jobs/{ID}/decision.md": DECISION,
      "doubles/git/push_origin.code": "1\n", "doubles/git/push_origin.stderr": "error: unable to delete\n"}),
    ("reject-03-wrong-stage", ["reject", ID], "x\n", {**base(waiting_job(stage="build")), **OPERATOR}),
    ("reject-04-empty-stdin", ["reject", ID], "\n", {**base(waiting_job()), **OPERATOR}),
    ("reject-05-comment-fails", ["reject", ID], "x\n",
     {**base(waiting_job()), **OPERATOR, "doubles/gh/issue_comment.code": "1\n",
      "doubles/gh/issue_comment.stderr": "comment refused\n"}),
    ("reject-06-worktree-remove-fails", ["reject", ID], "x\n",
     {**base(waiting_job(worktree="wt")), **OPERATOR, "wt/README.md": "worktree\n",
      "doubles/git/worktree_remove.code": "1\n", "doubles/git/worktree_remove.stderr": "fatal: locked\n"}),
    ("reject-07-pr-close-fails", ["reject", ID], "x\n",
     {**base(waiting_job(pr=7)), **OPERATOR, "doubles/gh/pr_close.code": "1\n", "doubles/gh/pr_close.stderr": "no PR\n"}),
]


def write(out, cases):
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w") as f:
        for name, args, stdin, fs in cases:
            f.write(json.dumps({"name": name, "args": args, "stdin": stdin, "env": {}, "files": fs}) + "\n")
    print(f"wrote {len(cases)} cases to {out.relative_to(ROOT)}")


def main():
    write(OUT, [(n, a, "", fs) for n, a, fs in CASES] + FEEDBACK_CASES)
    write(REJECT_OUT, REJECT_CASES)


if __name__ == "__main__":
    main()
