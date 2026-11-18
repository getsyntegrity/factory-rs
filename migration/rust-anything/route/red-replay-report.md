# App replay

Build: `sh launch.sh rust`

Passed: 0/22

Fallback rate: not reported by this build

## Failing cases

### 01-no-args

Arguments: ``

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: usage: route <job-id>
```

### 02-two-args

Arguments: `001-add-a-thing extra`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: usage: route <job-id>
```

### 03-job-missing

Arguments: `999-no-such-job`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: job: no job 999-no-such-job
```

### 04-no-intake

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: 001-add-a-thing has no intake.md
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

### 05-low-no-surfaces

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code haiku
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "low",
-     "surfaces": []
-   },
-   "agents": {
-     "reviewers": {
-       "code": "haiku"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 06-medium-no-surfaces

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code sonnet
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "medium",
-     "surfaces": []
-   },
-   "agents": {
-     "reviewers": {
-       "code": "sonnet"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 07-high-no-surfaces

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code sonnet
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "high",
-     "surfaces": []
-   },
-   "agents": {
-     "reviewers": {
-       "code": "sonnet"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 08-surface-ui

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code haiku, ux sonnet, ui haiku
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "low",
-     "surfaces": [
-       "ui"
-     ]
-   },
-   "agents": {
-     "reviewers": {
-       "code": "haiku",
-       "ux": "sonnet",
-       "ui": "haiku"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 09-surface-security

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code sonnet, security opus
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "medium",
-     "surfaces": [
-       "security"
-     ]
-   },
-   "agents": {
-     "reviewers": {
-       "code": "sonnet",
-       "security": "opus"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 10-surfaces-ui-security

Arguments: `001-add-a-thing`

stdout:

```diff
- 001-add-a-thing: reviewers code sonnet, ux sonnet, ui haiku, security opus
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "high",
-     "surfaces": [
-       "ui",
-       "security"
-     ]
-   },
-   "agents": {
-     "reviewers": {
-       "code": "sonnet",
-       "ux": "sonnet",
-       "ui": "haiku",
-       "security": "opus"
-     }
-   },
+   "intake": null,
+   "agents": null,
    "gate": null,
    "reviews": {},
    "cost_usd": 0.0,
  ⋮
```

### 11-unknown-risk

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: risk 'extreme' isn't in config.json routing (low, medium, high)
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "extreme",
-     "surfaces": []
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### 12-unknown-surface

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: surface 'api' isn't in config.json surface_reviewers (ui, security)
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "low",
-     "surfaces": [
-       "ui",
-       "api"
-     ]
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### 13-sync-fails

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: double: sync sync 001-add-a-thing
- sync exploded
```

### 14-agents-fails

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: agents refused: [001-add-a-thing] [{"reviewers": {"code": "sonnet", "ux": "sonnet", "ui": "haiku", "security": "opus"}}]
```

### 15a-invalid-chars-id

Arguments: `'bad id/../x'`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: job: no job bad id/../x
```

### 15b-issue-number-id

Arguments: `1`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/route", line 51, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/route", line 33, in main
-     job = json.loads((FACTORY / "jobs" / job_id / "job.json").read_text())
-                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/jobs/1/job.json'
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "low",
-     "surfaces": []
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### 16a-config-missing

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/route", line 17, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
```

### 16b-config-invalid

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/route", line 17, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/__init__.py", line 346, in loads
-     return _default_decoder.decode(s)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 337, in decode
-     obj, end = self.raw_decode(s, idx=_w(s, 0).end())
-                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 353, in raw_decode
-     obj, end = self.scan_once(s, idx)
-                ^^^^^^^^^^^^^^^^^^^^^^
- json.decoder.JSONDecodeError: Expecting property name enclosed in double quotes: line 1 column 3 (char 2)
```

### 17-intake-rejected

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: job: 001-add-a-thing: route only an intake job that intake ACCEPTed
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "REJECT",
-     "rescoped": false,
-     "risk": "low",
-     "surfaces": []
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### 18-intake-without-risk

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: risk None isn't in config.json routing (low, medium, high)
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": null,
-     "surfaces": []
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### 19-quote-in-risk

Arguments: `001-add-a-thing`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- route: risk "it's" isn't in config.json routing (low, medium, high)
```

file factory/jobs/001-add-a-thing/.lock:

```diff
not written
```

file factory/jobs/001-add-a-thing/job.json:

```diff
  ⋮
    "stage": "intake",
    "round": 0,
    "round_limit": 3,
-   "intake": {
-     "decision": "ACCEPT",
-     "rescoped": false,
-     "risk": "it's",
-     "surfaces": []
-   },
+   "intake": null,
    "agents": null,
    "gate": null,
    "reviews": {},
  ⋮
```

### example-001

Arguments: `'$@'`

status:

```diff
expected exit code 1, got exit code 0
```

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/route", line 17, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
```
