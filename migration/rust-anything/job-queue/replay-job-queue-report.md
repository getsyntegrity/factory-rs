# App replay

Build: `sh launch.sh rust`

Passed: 15/16

Fallback rate: not reported by this build

## Failing cases

### queue-06-invalid-json

Arguments: `queue`

stderr:

```diff
- Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
-     main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 689, in main
-     cmd_queue()
-   File "[CWD]/factory/bin/job", line 556, in cmd_queue
-     issues = gh_json("issue", "list", "--label", LABELS["queue"], "--state", "open",
-              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 84, in gh_json
-     return json.loads(gh(*args))
-            ^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/__init__.py", line 346, in loads
-     return _default_decoder.decode(s)
-            ^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 337, in decode
-     obj, end = self.raw_decode(s, idx=_w(s, 0).end())
-                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "/usr/lib/python3.12/json/decoder.py", line 355, in raw_decode
-     raise JSONDecodeError("Expecting value", s, err.value) from None
- json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)
+ job: `gh issue list --label factory --state open --limit 200 --json number,title` did not print JSON (line 1, column 2)
```
