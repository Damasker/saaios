#!/usr/bin/env bash
# Mark the active A/B slot as BOOT_OK after a healthy start.
# Called manually or by saaios-boot-ok.service oneshot.
# Prefers a JSON ping on the UDS socket; falls back to socket presence.
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"
SOCK="${SAAIOS_SOCK:-/run/saaios/saaios.sock}"
TIMEOUT_SECS="${SAAIOS_BOOT_OK_TIMEOUT:-30}"

SLOT_RAW="$(readlink "$AB_ROOT/current" 2>/dev/null || cat "$AB_ROOT/current_slot" 2>/dev/null || true)"
SLOT=""
case "$SLOT_RAW" in
  A|*/A) SLOT=A ;;
  B|*/B) SLOT=B ;;
  *) SLOT="$SLOT_RAW" ;;
esac
if [[ "$SLOT" != A && "$SLOT" != B ]]; then
  echo "no active slot under $AB_ROOT (got: ${SLOT_RAW:-empty})" >&2
  exit 1
fi

runtime_healthy() {
  if [[ ! -S "$SOCK" ]]; then
    return 1
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    # Socket presence is enough when python is unavailable.
    return 0
  fi
  python3 - "$SOCK" <<'PY'
import json, socket, sys
sock_path = sys.argv[1]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(2)
try:
    s.connect(sock_path)
    s.sendall(b'{"op":"ping"}')
    s.shutdown(socket.SHUT_WR)
    data = b""
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
    resp = json.loads(data.decode())
    sys.exit(0 if resp.get("ok") else 1)
except Exception:
    sys.exit(1)
finally:
    s.close()
PY
}

deadline=$((SECONDS + TIMEOUT_SECS))
while (( SECONDS < deadline )); do
  if runtime_healthy; then
    date -u +"%Y-%m-%dT%H:%M:%SZ" >"$AB_ROOT/$SLOT/BOOT_OK"
    echo "BOOT_OK written for slot $SLOT ($AB_ROOT/$SLOT/BOOT_OK)"
    exit 0
  fi
  sleep 1
done

echo "BOOT_OK failed: socket $SOCK not healthy within ${TIMEOUT_SECS}s" >&2
exit 1
