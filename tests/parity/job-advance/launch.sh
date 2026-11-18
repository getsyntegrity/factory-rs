#!/bin/sh
# Oracle cases for the parts of `job` still in Python (most of `advance`,
# `merge`). Recorded with the `timestamps` normalizer: advance writes
# `ended = now()` when a job enters a waiting or terminal stage.
exec sh "$(cd "$(dirname "$0")" && pwd)/../job/launch.sh" "$@"
