# App replay

Build: `sh launch.sh rust`

Passed: 67/70

Fallback rate: not reported by this build

## Failing cases

### approve-05-needs-human-o

Arguments: `advance 001-add-a-thing needs-human`

stderr:

```diff
  Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
+   File "[CWD]/factory/bin/job.py", line 718, in <module>
      main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 693, in main
+   File "[CWD]/factory/bin/job.py", line 693, in main
      job = advance(sync(resolve(args[0])), args[1])
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 491, in advance
+   File "[CWD]/factory/bin/job.py", line 491, in advance
      body = f"### Factory: needs a human\n\n{(d / 'decision.md').read_text().split(chr(10), 1)[1].strip()}"
                                              ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~^^^
  IndexError: list index out of range
```

### build-11-rev-list-not-a

Arguments: `advance 001-add-a-thing gate`

stderr:

```diff
  Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
+   File "[CWD]/factory/bin/job.py", line 718, in <module>
      main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 693, in main
+   File "[CWD]/factory/bin/job.py", line 693, in main
      job = advance(sync(resolve(args[0])), args[1])
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 424, in advance
+   File "[CWD]/factory/bin/job.py", line 424, in advance
      require(int(git(job, "rev-list", "--count", f"origin/{BASE}..HEAD").stdout) > 0,
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  ValueError: invalid literal for int() with base 10: 'many\n'
```

### info-04-no-intake-md

Arguments: `advance 001-add-a-thing intake`

stderr:

```diff
  Traceback (most recent call last):
-   File "[CWD]/factory/bin/job", line 718, in <module>
+   File "[CWD]/factory/bin/job.py", line 718, in <module>
      main(sys.argv[1:])
-   File "[CWD]/factory/bin/job", line 693, in main
+   File "[CWD]/factory/bin/job.py", line 693, in main
      job = advance(sync(resolve(args[0])), args[1])
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
-   File "[CWD]/factory/bin/job", line 411, in advance
+   File "[CWD]/factory/bin/job.py", line 411, in advance
      (d / "intake.md").rename(d / f"intake-{k}.md")
    File "/usr/lib/python3.12/pathlib.py", line 1365, in rename
      os.rename(self, target)
  ⋮
```
