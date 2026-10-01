#!/bin/sh
# Host-only fixture test. self-test exits before opening any modem device.
set -eu
HERE=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
OUT=$(mktemp "${TMPDIR:-/tmp}/ready-network-once-test.XXXXXX")
trap 'rm -f "$OUT"' EXIT HUP INT TERM
${CC:-cc} -O2 -Wall -Wextra -Werror "$HERE/ready-network-once.c" -o "$OUT"
"$OUT" self-test
${CC:-cc} -O2 -Wall -Wextra -Werror "$HERE/test-sit-network-layout.c" -o "$OUT"
"$OUT"
