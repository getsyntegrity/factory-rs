#!/usr/bin/env python3
"""Generate tests/parity/gate/inputs.jsonl, the recorded cases for `gate`.

Each line follows the `rust-anything app record --inputs` schema (v1.5.1):
{"name", "args": [...], "stdin", "env": {...}, "files": {path: text}}.
Jobs use the relative worktree `wt` (gate runs git and checks.sh with it as
the working directory). `git` is the double from launch.sh, answering from
git-double/.

Usage: python3 migration/rust-anything/gate/make_inputs.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONFIG = (ROOT / ".oracle/v2/factory/config.json").read_text()
OUT = ROOT / "tests/parity/gate/inputs.jsonl"

ID = "001-add-a-thing"
SHA = "0123456789abcdef0123456789abcdef01234567"


def job(round_=1, worktree="wt"):
    return json.dumps({
        "id": ID, "issue": 1, "title": "Add a thing", "branch": f"factory/{ID}",
        "worktree": worktree, "stage": "gate", "round": round_, "round_limit": 3,
    }, indent=2) + "\n"


def git_answers(rev_code=0, rev_stderr="", diff="", diff_code=0, diff_stderr="", base="main"):
    fs = {
        "git-double/cwd": "wt",
        "git-double/rev-parse.args": "rev-parse\nHEAD\n",
        "git-double/rev-parse.code": f"{rev_code}\n",
        "git-double/diff.args": f"diff\n--name-status\norigin/{base}...HEAD\n",
        "git-double/diff.stdout": diff,
        "git-double/diff.code": f"{diff_code}\n",
    }
    if rev_code == 0:
        fs["git-double/rev-parse.stdout"] = SHA + "\n"
    if rev_stderr:
        fs["git-double/rev-parse.stderr"] = rev_stderr
    if diff_stderr:
        fs["git-double/diff.stderr"] = diff_stderr
    return fs


def bash_text(s):
    """A bash ANSI-C quoted string ($'...') that prints exactly `s`."""
    escaped = s.replace("\\", "\\\\").replace("'", "\\'").replace("\n", "\\n").replace("\r", "\\r")
    return f"$'{escaped}'"


def checks(stdout="all good\n", stderr="", code=0):
    return (
        "#!/usr/bin/env bash\n"
        f"printf '%s' {bash_text(stdout)}\n"
        f"printf '%s' {bash_text(stderr)} >&2\n"
        f"exit {code}\n"
    )


def case(round_=1, config=CONFIG, checks_sh=None, round_dir=True, **git):
    fs = {"wt/README.md": "worktree\n", f"factory/jobs/{ID}/job.json": job(round_)}
    if config is not None:
        fs["factory/config.json"] = config
    fs["factory/checks.sh"] = checks_sh if checks_sh is not None else checks()
    if round_dir:
        fs[f"factory/jobs/{ID}/round-{round_}/build.md"] = "STATUS: DONE\n"
    fs.update(git_answers(**git))
    return fs


PROTECTED_CONFIG = json.loads(CONFIG)
PROTECTED_CONFIG["protected_paths"] += ["docs/*.md", "src/?.rs", "lib/[ab]*", "data/[!x]*"]
PROTECTED_CONFIG = json.dumps(PROTECTED_CONFIG, indent=2) + "\n"

PROTECTED_DIFF = (
    "M\tfactory/checks.sh\n"
    "D\tfactory/config.json\n"
    "A\tfactory/new-protected.json\n"
    "A\tdocs/new.md\n"
    "M\tdocs/deep/nested/guide.md\n"
    "R100\tsrc/a.rs\tsrc/b.rs\n"
    "C075\tlib/alpha.py\tlib/copy.py\n"
    "M\tlib/cat.py\n"
    "M\tdata/x1\n"
    "T\tdata/y1\n"
    "M\tsrc/ab.rs\n"
    "M\tREADME.md\n"
)

LONG_OUT = "".join(f"line {i}\n" for i in range(1, 121))
HOOK_JOB_FILES = {"wt/README.md": "worktree\n", f"factory/jobs/{ID}/job.json": job(), "factory/config.json": CONFIG}


def transcript(content):
    return json.dumps({"type": "user", "message": {"content": content}}) + "\n" + json.dumps({"type": "assistant"}) + "\n"


def hook_case(event, checks_sh, transcript_text=None):
    fs = {**HOOK_JOB_FILES, "factory/checks.sh": checks_sh}
    if transcript_text is not None:
        fs["transcripts/agent-1.jsonl"] = transcript_text
    return json.dumps(event), fs


HOOK_EVENT = {"session_id": "s", "agent_transcript_path": "transcripts/agent-1.jsonl", "stop_hook_active": False}

CASES = [
    # ---- argv -------------------------------------------------------------
    ("argv-01-no-args", [], "", case()),
    ("argv-02-dash-arg", ["-x"], "", case()),
    ("argv-03-three-args", [ID, "--merge", "x"], "", case()),
    ("argv-04-second-not-merge", [ID, "--fast"], "", case()),
    ("argv-05-hook-with-extra", ["--hook", "x"], "", case()),
    # ---- gate <job> ------------------------------------------------------
    ("gate-01-pass", [ID], "", case()),
    ("gate-02-checks-fail", [ID], "",
     case(checks_sh=checks("out 1\nout 2\n", "err 1\nerr 2\n", 3))),
    ("gate-03-protected-paths", [ID], "", case(config=PROTECTED_CONFIG, diff=PROTECTED_DIFF)),
    ("gate-04-protected-and-fail", [ID], "", case(checks_sh=checks("", "boom\n", 1), diff="M\tfactory/config.json\n")),
    ("gate-05-long-crlf-output", [ID], "",
     case(checks_sh=checks(LONG_OUT.replace("\n", "\r\n") + "\n\n  \n", "tail err\r\n"))),
    ("gate-06-no-output", [ID], "", case(checks_sh=checks("", "", 0))),
    ("gate-07-round-zero", [ID], "", case(round_=0, round_dir=False)),
    ("gate-08-rev-parse-fails", [ID], "", case(rev_code=128, rev_stderr="fatal: not a git repository\n")),
    ("gate-09-diff-fails", [ID], "",
     case(diff_code=128, diff_stderr="fatal: ambiguous argument 'origin/main...HEAD'\n")),
    ("gate-10-other-base-branch", [ID], "",
     case(config=CONFIG.replace('"base_branch": "main"', '"base_branch": "trunk"'), base="trunk")),
    ("gate-11-job-missing", ["999-nope"], "", case()),
    ("gate-12-round-dir-missing", [ID], "", case(round_dir=False)),
    ("gate-13-config-missing", [ID], "", case(config=None)),
    ("gate-14-checks-sh-missing", [ID], "", {k: v for k, v in case().items() if k != "factory/checks.sh"}),
    ("gate-15-unicode-output", [ID], "", case(checks_sh=checks("ñandú ✓\n", "", 0))),
    # ---- gate <job> --merge ----------------------------------------------
    ("merge-01-pass", [ID, "--merge"], "", case()),
    ("merge-02-fail", [ID, "--merge"], "", case(checks_sh=checks("x\n", "", 2), diff="M\tfactory/checks.sh\n")),
    ("merge-03-dash-job", ["-weird", "--merge"], "", case()),
    # ---- gate --hook -----------------------------------------------------
    ("hook-01-stop-hook-active", ["--hook"], json.dumps({"stop_hook_active": True}), case()),
    ("hook-02-checks-pass", ["--hook"], *hook_case(HOOK_EVENT, checks(), transcript(f"Job folder: x\nJob: {ID}. Build it."))),
    ("hook-03-checks-fail", ["--hook"], *hook_case(
        HOOK_EVENT, checks(LONG_OUT, "fail!\n", 5),
        transcript([{"type": "text", "text": "Intro"}, {"type": "text", "text": f"Job: {ID}. Go"}]))),
    ("hook-04-invalid-stdin", ["--hook"], "not json", case()),
    ("hook-05-no-job-in-prompt", ["--hook"], *hook_case(HOOK_EVENT, checks(), transcript("no marker here"))),
]


def main():
    with OUT.open("w") as f:
        for name, args, stdin, fs in CASES:
            f.write(json.dumps({"name": name, "args": args, "stdin": stdin, "env": {}, "files": fs}) + "\n")
    print(f"wrote {len(CASES)} cases to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
