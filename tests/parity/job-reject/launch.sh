#!/bin/sh
# `job reject` cases live in their own harness because they are recorded with
# the `timestamps` normalizer (reject writes `ended = now()` into job.json);
# keeping it here leaves every other job case unnormalized.
exec sh "$(cd "$(dirname "$0")" && pwd)/../job/launch.sh" "$@"
