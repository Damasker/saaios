#!/bin/sh
# Host-only regression: no-argument invocation must refuse before device access.
set -eu
HERE=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
OUT=$(mktemp "${TMPDIR:-/tmp}/rfs-poll-default-test.XXXXXX")
trap 'rm -f "$OUT"' EXIT HUP INT TERM
${CC:-cc} -O2 -Wall -Wextra -Werror "$HERE/rfs-poll.c" -o "$OUT"
set +e
MESSAGE=$("$OUT" 2>&1)
STATUS=$?
set -e
test "$STATUS" -eq 64
case "$MESSAGE" in
    *"purges queued CP requests"*) ;;
    *) printf '%s\n' "default refusal did not explain queue purge" >&2; exit 1 ;;
esac
printf '%s\n' "PASS rfs-poll refuses default invocation"
