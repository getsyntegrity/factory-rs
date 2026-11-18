#!/bin/sh
# Which language answers each step of the e2e scenarios in hybrid mode.
# Runs each scenario outside rust-anything with the hybrid install, but with a
# factory/bin/job.py that logs its argv before running v2's job, so every
# command the Rust job hands over is counted.
# Usage: sh tests/parity/e2e/trace.sh   (from the repository root)
set -u
root=$(pwd -P)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
trace="$work/trace"
: > "$trace"
for sc in full-cycle waiting-loop rework-rounds; do
  repo="$work/$sc"
  mkdir -p "$repo"
  cp -R "tests/parity/e2e/rust/app-cases/$sc.in/." "$repo/"
  mkdir -p "$repo/factory/bin"
  for b in route job gate; do cp "target/release/$b" "$repo/factory/bin/$b"; done
  cp .oracle/v2/factory/bin/job "$repo/factory/bin/job-v2.py"
  printf '#!/bin/sh\necho "%s $*" >> "%s"\nexec python3 "$(dirname "$0")/job-v2.py" "$@"\n' "$sc" "$trace" > "$repo/factory/bin/job.py"
  chmod +x "$repo/factory/bin/"*
  (
    cd "$repo"
    CLAUDE_CONFIG_DIR="$repo/.none"; export CLAUDE_CONFIG_DIR
    . "$root/tests/parity/doubles.sh"
    sh "$root/tests/parity/e2e/scenarios/$sc.sh" > steps.out 2>&1
    rm -rf "$doubles"
  )
  steps=$(grep -c '^exit=' "$repo/steps.out")
  python=$(grep -c "^$sc " "$trace")
  echo "$sc: $steps steps, $((steps - python)) answered by Rust, $python by Python (job.py)"
done
echo "by Python:"
sed 's/ 0[0-9][0-9]-[a-z0-9-]*/ <id>/' "$trace" | cut -d' ' -f2- | sort | uniq -c
