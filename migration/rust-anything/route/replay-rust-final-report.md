# App replay

Build: `sh launch.sh rust`

Passed: 18/22

Fallback rate: not reported by this build

## Failing cases

### 15b-issue-number-id

Arguments: `1`

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
+ route: job 1 has no factory/jobs/1/job.json
```

### 16a-config-missing

Arguments: `001-add-a-thing`

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
+ route: factory/config.json is missing
```

### 16b-config-invalid

Arguments: `001-add-a-thing`

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
+ route: factory/config.json is not valid JSON (line 1, column 3)
```

### example-001

Arguments: `'$@'`

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
+ route: factory/config.json is missing
```
