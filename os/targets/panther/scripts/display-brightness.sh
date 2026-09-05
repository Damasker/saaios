#!/saaios/busybox sh
set -eu

BRIGHTNESS=/sys/devices/platform/1c2c0000.drmdsim/1c2c0000.drmdsim.0/backlight/panel0-backlight/brightness
SAVED=/metadata/saaios/brightness
value=1023
[ ! -r "$SAVED" ] || value=$(/saaios/busybox cat "$SAVED")

attempt=0
while [ ! -w "$BRIGHTNESS" ] && [ "$attempt" -lt 10 ]; do
    /saaios/busybox sleep 1
    attempt=$((attempt + 1))
done
[ -w "$BRIGHTNESS" ]

case "${1:-restore}" in
    up) value=$((value + 256)) ;;
    down) value=$((value - 256)) ;;
    restore) ;;
    *) exit 2 ;;
esac

[ "$value" -le 3071 ] || value=3071
[ "$value" -ge 255 ] || value=255
printf '%s\n' "$value" > "$BRIGHTNESS"
printf '%s\n' "$value" > "$SAVED"
/saaios/busybox chmod 600 "$SAVED"
