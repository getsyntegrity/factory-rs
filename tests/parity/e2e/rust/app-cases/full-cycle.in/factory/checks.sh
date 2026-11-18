#!/usr/bin/env bash
# Runs in the job's worktree (.worktrees/<id>); fails while the scenario
# leaves .fail-checks at the repository root.
if [ -e ../../.fail-checks ]; then
  echo "tests failed"
  exit 1
fi
echo "all checks passed"
