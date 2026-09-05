#!/saaios/busybox sh
set -eu

exec > /run/audio-test.log 2>&1
[ -e /run/audio-ready ]
: > /run/audio-playing
trap '/saaios/busybox rm -f /run/audio-playing' EXIT
/saaios/tinyplay /saaios/test-tone.wav -D 0 -d 1
