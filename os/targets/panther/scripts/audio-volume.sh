#!/saaios/busybox sh
set -eu

M=/saaios/tinymix
VOLUME_FILE=/run/audio-volume
value=600
[ ! -r "$VOLUME_FILE" ] || value=$(/saaios/busybox cat "$VOLUME_FILE")

case "${1:-}" in
    up) value=$((value + 40)) ;;
    down) value=$((value - 40)) ;;
    *) exit 2 ;;
esac

[ "$value" -le 817 ] || value=817
[ "$value" -ge 400 ] || value=400
$M -D 0 'Digital PCM Volume' "$value"
$M -D 0 'R Digital PCM Volume' "$value"
printf '%s\n' "$value" > "$VOLUME_FILE"
