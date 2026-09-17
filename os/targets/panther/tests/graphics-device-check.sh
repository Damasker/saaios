#!/bin/sh
set -eu

failures=0

check() {
    label=$1
    shift
    if "$@"; then
        printf 'PASS %s\n' "$label"
    else
        printf 'FAIL %s\n' "$label"
        failures=$((failures + 1))
    fi
}

check "DRM card0 exists" test -c /dev/dri/card0
check "touchscreen exists" test -c /dev/input/touchscreen
check "power button exists" test -c /dev/input/power-button
check "volume buttons exist" test -c /dev/input/volume-buttons
check "DRM renderer is running" \
    sh -c "busybox ps | busybox grep '[d]rm-splash' >/dev/null"
check "renderer log has selected mode" \
    sh -c "busybox grep 'mode=.*1080x2400@60' /run/drm-splash.log >/dev/null"
check "renderer log has no refresh failures" \
    sh -c "! busybox grep -E 'SETCRTC failed|display on failed|input poll failed' /run/drm-splash.log >/dev/null"

printf '\nRenderer metrics:\n'
busybox grep 'drm-splash: perf ' /run/drm-splash.log | busybox tail -n 3 || true

printf '\nManual acceptance still required:\n'
printf '%s\n' \
    '- BGRX calibration colors are correct' \
    '- lock screen blocks app/input dispatch' \
    '- every touch target matches its visible control' \
    '- power off/on and touch wake restore the same frame' \
    '- text is unclipped on all 12 preview-equivalent screens' \
    '- no tearing is visible during repeated navigation'

if [ "$failures" -ne 0 ]; then
    printf '\ngraphics-device-check: %d failure(s)\n' "$failures"
    exit 1
fi
printf '\ngraphics-device-check: automated checks passed\n'
