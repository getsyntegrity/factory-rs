#!/bin/sh
# `job create` and `job queue` cases live in their own harness because they
# are recorded with the `timestamps` normalizer (create writes
# `started = now()`, queue's auto-merge writes `ended = now()`).
exec sh "$(cd "$(dirname "$0")" && pwd)/../job/launch.sh" "$@"
