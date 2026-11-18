# App replay

Build: `sh launch.sh rust`

Passed: 28/33

Fallback rate: not reported by this build

## Failing cases

### agents-05-invalid-json

Arguments: `agents 001-add-a-thing '{nope'`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 703, in main
-     cmd_agents(sync(resolve(args[0])), args[1])
-   File "[CWD]/factory/bin/job", line 658, in cmd_agents
-     job["agents"] = json.loads(spec)
-                     ^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/__init__.py", line 346, in loads
-     return _default_decoder.decode(s)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 337, in decode
-     obj, end = self.raw_decode(s, idx=_w(s, 0).end())
-                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 353, in raw_decode
-     obj, end = self.scan_once(s, idx)
-                ^^^^^^^^^^^^^^^^^^^^^^
- json.decoder.JSONDecodeError: Expecting property name enclosed in double quotes: line 1 column 2 (char 1)
+ job: the agent set is not valid JSON (line 1, column 2)
```

### example-001

Arguments: `'$@'`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 36, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
+ job: factory/config.json is missing
```

### sync-15-keys-appended-in

Arguments: `sync 001-add-a-thing`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 707, in main
-     reviews = " ".join(f"{k}:{v}" for k, v in job["reviews"].items())
-                                               ~~~^^^^^^^^^^^
- KeyError: 'reviews'
+ job: factory/jobs/001-add-a-thing/job.json has no reviews
```

### sync-16-job-json-invalid

Arguments: `sync 001-add-a-thing`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 705, in main
-     job = sync(resolve(args[0]))
-                ^^^^^^^^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 149, in resolve
-     return load(arg)
-            ^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 116, in load
-     return json.loads(path.read_text())
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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
+ job: factory/jobs/001-add-a-thing/job.json is not valid JSON (line 1, column 3)
```

### sync-17-config-missing

Arguments: `sync 001-add-a-thing`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 36, in <module>
-     CONFIG = json.loads((FACTORY / "config.json").read_text())
-                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1029, in read_text
-     with self.open(mode='r', encoding=encoding, errors=errors) as f:
-          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/pathlib.py", line 1015, in open
-     return io.open(self, mode, buffering, encoding, errors, newline)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
- FileNotFoundError: [Errno 2] No such file or directory: '[CWD]/factory/config.json'
+ job: factory/config.json is missing
```
