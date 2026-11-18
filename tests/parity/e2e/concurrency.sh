#!/bin/sh
# Lock compatibility between the Rust job and v2's Python job on one job.
# Usage: concurrency.sh <iterations>   (from the repository root)
#
# Both take an exclusive flock(2) on factory/jobs/<id>/.lock until they exit.
#
# 1. Hold-and-wait, both directions: `job feedback` loads (and locks) the job
#    before it reads stdin, in v2 and in Rust. Feeding it from a FIFO keeps
#    the lock held for HOLD seconds; the other language's `job sync` must
#    wait until then.
# 2. Race: Rust `job sync` and Python `job sync` start together on the same
#    job, <iterations> times; each must exit 0 and job.json must stay valid.
set -u
root=$(pwd -P)
iterations=${1:-100}
HOLD=2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/factory/bin" "$work/factory/jobs/001-a"
cp "$root/target/release/job" "$work/factory/bin/job"
cp "$root/.oracle/v2/factory/bin/job" "$work/factory/bin/job.py"
cp "$root/.oracle/v2/factory/config.json" "$work/factory/config.json"
cat > "$work/factory/jobs/001-a/job.json" <<'JSON'
{"id": "001-a", "issue": 1, "title": "A", "issue_url": null, "pr": null, "pr_url": null,
 "branch": "factory/001-a", "worktree": "/nowhere", "stage": "needs-human", "round": 1,
 "round_limit": 3, "intake": null, "agents": null, "gate": null, "reviews": {},
 "cost_usd": 0.0, "started": "2026-11-18T00:00:00Z", "ended": null}
JSON
export CLAUDE_CONFIG_DIR="$work/.none"
cd "$work"

now() { date +%s.%N; }
elapsed() { echo "$2 - $1" | bc; }

hold_and_wait() { # <holder: rust|python> <waiter: rust|python|control>
  holder=$1; waiter=$2
  [ "$holder" = rust ] && h=factory/bin/job || h=factory/bin/job.py
  case "$waiter" in
    rust) w=factory/bin/job ;;
    python) w=factory/bin/job.py ;;
    # Control: reads the job without locking; it must not wait, or the
    # measurement would not be measuring the lock.
    control) w=lockless ;;
  esac
  fifo="$work/fifo"; rm -f "$fifo"; mkfifo "$fifo"
  "$h" feedback 001-a < "$fifo" > /dev/null 2>&1 &
  holder_pid=$!
  exec 3> "$fifo"          # opens the FIFO: the holder now blocks reading stdin
  sleep 0.5                # the holder has loaded and locked the job by now
  start=$(now)
  ( sleep "$HOLD"; exec 3>&- ) &
  exec 3>&-
  if [ "$w" = lockless ]; then cat factory/jobs/001-a/job.json > /dev/null; rc=$?; else "$w" sync 001-a > /dev/null 2>&1; rc=$?; fi
  end=$(now)
  wait "$holder_pid" 2>/dev/null
  waited=$(elapsed "$start" "$end")
  ok=$(echo "$waited >= $HOLD - 0.6" | bc)
  [ "$waiter" = control ] && ok=$(echo "$waited < 0.5" | bc)
  printf '%s holds, %s waits: waited %.2fs (hold %ss), exit %s -> %s\n' \
    "$holder" "$waiter" "$waited" "$HOLD" "$rc" "$([ "$ok" = 1 ] && [ "$rc" = 0 ] && echo PASS || echo FAIL)"
}

echo "== hold and wait"
hold_and_wait python rust
hold_and_wait rust python
hold_and_wait python control
hold_and_wait rust control

echo "== race: Rust sync and Python sync together, $iterations runs"
fails=0
i=0
while [ "$i" -lt "$iterations" ]; do
  factory/bin/job sync 001-a > r.out 2> r.err & a=$!
  factory/bin/job.py sync 001-a > p.out 2> p.err & b=$!
  wait "$a"; ra=$?
  wait "$b"; rb=$?
  if [ "$ra" != 0 ] || [ "$rb" != 0 ] || ! python3 -c 'import json,sys; json.load(open(sys.argv[1]))' factory/jobs/001-a/job.json \
     || ! cmp -s r.out p.out; then
    fails=$((fails + 1))
    echo "run $i: rust exit $ra, python exit $rb"; cat r.err p.err
  fi
  i=$((i + 1))
done
echo "race runs: $iterations, failures: $fails"
