#!/usr/bin/env bash
printf '%s' $'out 1\nout 2\n'
printf '%s' $'err 1\nerr 2\n' >&2
exit 3
