#!/bin/sh
# Re-verifies every claim of migration/AUDIT.md from scratch.
# Usage: sh migration/audit/run_audit.sh   (from the repo root; takes minutes)
set -u
section() { printf '\n== %s\n' "$1"; }

section "A1 oracle frozen and untouched"
git -C .oracle/src-repo rev-parse origin/main
tmp=$(mktemp -d)
git -C .oracle/src-repo archive c467603d6d62392cfaae0c07d2f701e99e111443 src/software-factory-claude-code-v2 | tar -x -C "$tmp"
diff -r -x .git "$tmp/src/software-factory-claude-code-v2" .oracle/v2 > /dev/null && echo "oracle == source at c467603d" || echo "ORACLE DIFFERS"
rm -rf "$tmp"
echo "uncommitted changes in .oracle/v2: $(git -C .oracle/v2 status --short | wc -l)"

section "A2 gates"
cargo fmt --check && echo "fmt OK"
cargo clippy --all-targets -- -D warnings 2>&1 | tail -1
cargo test 2>&1 | grep 'test result' | head -1
cargo build --release 2>&1 | tail -1
cargo llvm-cov --summary-only 2>&1 | grep '^TOTAL' | awk '{print "line coverage " $(NF-3)}'

section "A3 skill usage"
python3 -I migration/audit/measure_skill.py

section "A4 originals replay against their recordings (v2 vs v2)"
for d in route job job-reject job-queue gate job-advance e2e; do
  printf '%-11s ' "$d"; rust-anything app replay "tests/parity/$d" 2>&1 | head -1
done

section "A5 Rust (ported harnesses) and hybrid (still-Python harnesses)"
for d in route job job-reject job-queue gate job-advance; do
  out=$(rust-anything app replay "tests/parity/$d" --build "sh launch.sh rust" 2>&1)
  printf '%-11s %s | differing: %s\n' "$d" "$(echo "$out" | head -1)" "$(echo "$out" | grep '^- ' | sed 's/^- //' | tr '\n' ' ')"
done
printf '%-11s ' e2e; rust-anything app replay tests/parity/e2e --build "sh launch.sh hybrid" 2>&1 | head -1
printf '%-11s ' tripwire; rust-anything app replay tests/parity/route --build "sh launch.sh rust-tripwire" 2>&1 | head -1

section "A6 differing cases differ in stderr only, and only by the diagnostic line or a job.py traceback path"
for d in route job job-reject job-queue gate job-advance; do
  out=$(rust-anything app replay "tests/parity/$d" --build "sh launch.sh rust" 2>&1)
  r="tests/parity/$d/rust/app-cases/replay-report.md"
  fields=$(echo "$out" | grep '^- ' | sed 's/^[^:]*: //' | sort | uniq -c | tr -s ' ' | tr '\n' ';')
  added=$(grep -E '^\+ ' "$r" | grep -vc -E '^\+ (route|gate|job): |job\.py')
  printf '%-11s differing fields: %-14s added lines other than a diagnostic or job.py traceback: %s\n' "$d" "${fields:-none}" "$added"
done

section "A7 locks across languages"
sh tests/parity/e2e/concurrency.sh 100 | sed -n '/hold and wait/,$p'

section "A8 lines of v2 with a Rust counterpart"
python3 -I migration/audit/measure_lines.py

section "A9 git"
git branch -vv | cat
echo "uncommitted: $(git status --short | wc -l)"

section "A10 who answers each e2e step (hybrid)"
sh tests/parity/e2e/trace.sh

section "A11 gate --hook exit codes, v2 (recorded) vs Rust (run now)"
for c in hook-01-stop-hook-active hook-02-checks-pass hook-03-checks-fail hook-04-invalid-stdin hook-05-no-job-in-prompt; do
  v2=$(grep -o 'status.code = [0-9]*' "tests/parity/gate/rust/app-cases/$c.toml" | sed 's/status.code = //'); v2=${v2:-0}
  w=$(mktemp -d)
  cp -R "tests/parity/gate/rust/app-cases/$c.in/." "$w/" 2>/dev/null
  rust=$(cd "$w" && sh "$OLDPWD/tests/parity/gate/launch.sh" rust --hook < "$OLDPWD/tests/parity/gate/rust/app-cases/$c.stdin" > /dev/null 2>&1; echo $?)
  rm -rf "$w"
  case "$rust" in 0) effect="does not block (builder stops)";; 2) effect="BLOCKS (stderr to the builder)";; *) effect="non-blocking error: builder stops = fail-open";; esac
  printf '%-26s v2 %s  rust %s  %s\n' "$c" "$v2" "$rust" "$effect"
done
