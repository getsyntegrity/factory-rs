# App replay

Build: `sh launch.sh rust`

Passed: 22/29

Fallback rate: not reported by this build

## Failing cases

### example-001

Arguments: `'$@'`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 24, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
+ gate: factory/config.json is missing
```

### gate-11-job-missing

Arguments: `999-nope`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 121, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/gate", line 113, in main
-     gate(argv[0], merge=False)
-   File "[CWD]/factory/bin/gate", line 62, in gate
-     job = json.loads((FACTORY / "jobs" / job_id / "job.json").read_text())
-                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/jobs/999-nope/job.json'
+ gate: job 999-nope has no factory/jobs/999-nope/job.json
```

### gate-12-round-dir-missin

Arguments: `001-add-a-thing`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 121, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/gate", line 113, in main
-     gate(argv[0], merge=False)
-   File "[CWD]/factory/bin/gate", line 72, in gate
-     (round_dir / f"{name}.log").write_text(log)
-   File "/usr/lib/python3.12/pathlib.py", line 1049, in write_text
-     with self.open(mode='w', encoding=encoding, errors=errors, newline=newline) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/jobs/001-add-a-thing/round-1/checks.log'
+ gate: cannot write factory/jobs/001-add-a-thing/round-1/checks.log
```

### gate-13-config-missing

Arguments: `001-add-a-thing`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 24, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
+ gate: factory/config.json is missing
```

### hook-04-invalid-stdin

Arguments: `--hook`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 121, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/gate", line 111, in main
-     hook()
-   File "[CWD]/factory/bin/gate", line 92, in hook
-     event = json.load(sys.stdin)
-             ^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/__init__.py", line 293, in load
-     return loads(fp.read(),
-            ^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/__init__.py", line 346, in loads
-     return _default_decoder.decode(s)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 337, in decode
-     obj, end = self.raw_decode(s, idx=_w(s, 0).end())
-                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 355, in raw_decode
-     raise JSONDecodeError("Expecting value", s, err.value) from None
- json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)
+ gate: the hook event on stdin is not valid JSON (line 1, column 2)
```

### hook-05-no-job-in-prompt

Arguments: `--hook`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 121, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/gate", line 111, in main
-     hook()
-   File "[CWD]/factory/bin/gate", line 98, in hook
-     job_id = prompt.split("Job: ", 1)[1].split(".", 1)[0]
-              ~~~~~~~~~~~~~~~~~~~~~~~~^^^
- IndexError: list index out of range
+ gate: the subagent prompt names no job ("Job: <id>.")
```

### merge-03-dash-job

Arguments: `-weird --merge`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/gate", line 121, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/gate", line 115, in main
-     gate(argv[0], merge=True)
-   File "[CWD]/factory/bin/gate", line 62, in gate
-     job = json.loads((FACTORY / "jobs" / job_id / "job.json").read_text())
-                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/jobs/-weird/job.json'
+ gate: job -weird has no factory/jobs/-weird/job.json
```
