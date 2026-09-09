#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
temporary=$(mktemp -d)
displayd_pid=
base_pid=
appd_pid=

cleanup() {
    for pid in "$appd_pid" "$base_pid" "$displayd_pid"; do
        if [ -n "$pid" ]; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    case "$temporary" in
        /tmp/*) rm -rf "$temporary" ;;
    esac
}
trap cleanup EXIT INT TERM

cd "$repo_root"
cargo build --locked -p saai-displayd -p saai-demo-surface -p saai-appd

runtime="$temporary/runtime"
data_root="$temporary/data"
package="$temporary/package"
appd_socket="$temporary/appd.sock"
displayd_log="$temporary/displayd.log"
mkdir -m 0700 "$runtime"
mkdir -p "$package/bin"
cp apps/demo-surface/manifest.toml "$package/manifest.toml"
cp target/debug/saai-demo-surface "$package/bin/saai-demo-surface"
chmod 0755 "$package/bin/saai-demo-surface"

XDG_RUNTIME_DIR="$runtime" target/debug/saai-displayd >"$displayd_log" 2>&1 &
displayd_pid=$!

attempt=0
socket=
while [ "$attempt" -lt 200 ]; do
    socket=$(sed -n 's/.*WAYLAND_DISPLAY=//p' "$displayd_log" | tail -n 1)
    [ -n "$socket" ] && break
    attempt=$((attempt + 1))
    sleep 0.05
done
[ -n "$socket" ] || {
    printf '%s\n' "displayd did not publish a socket" >&2
    cat "$displayd_log" >&2
    exit 1
}

# A persistent ordinary toplevel stands in for the already running shell.
# The app launched by appd must become the second focus, and stopping it must
# restore this first surface as the third focus transition.
SAAIOS_APP_ID=org.saaios.host-base \
    XDG_RUNTIME_DIR="$runtime" WAYLAND_DISPLAY="$socket" \
    target/debug/saai-demo-surface base >"$temporary/base.log" 2>&1 &
base_pid=$!

attempt=0
while [ "$attempt" -lt 200 ]; do
    focus_count=$(grep -c 'keyboard focus set to' "$displayd_log" || true)
    if grep -q 'application frame' "$temporary/base.log" && [ "$focus_count" -ge 1 ]; then
        break
    fi
    attempt=$((attempt + 1))
    sleep 0.05
done
if ! grep -q 'application frame' "$temporary/base.log" || [ "${focus_count:-0}" -lt 1 ]; then
    printf '%s\n' "base toplevel did not become ready" >&2
    cat "$displayd_log" "$temporary/base.log" >&2
    exit 1
fi

target/debug/saai-appd \
    --data-root "$data_root" \
    --socket "$appd_socket" \
    --runtime-dir "$runtime" \
    --wayland-display "$socket" \
    >"$temporary/appd.log" 2>&1 &
appd_pid=$!

attempt=0
while [ ! -S "$appd_socket" ] && [ "$attempt" -lt 200 ]; do
    attempt=$((attempt + 1))
    sleep 0.05
done
[ -S "$appd_socket" ] || {
    printf '%s\n' "appd did not publish its socket" >&2
    cat "$temporary/appd.log" >&2
    exit 1
}

request() {
    printf '%s\n' "$1" | nc -N -U "$appd_socket"
}

install=$(request "{\"schema\":1,\"request_id\":\"host:install\",\"command\":\"install\",\"package_path\":\"$package\"}")
printf '%s' "$install" | grep -q '"result":"installed"'

launch=$(request '{"schema":1,"request_id":"host:launch","command":"launch","app_id":"org.saaios.demo-surface"}')
printf '%s' "$launch" | grep -q '"result":"launched"'

attempt=0
while [ "$attempt" -lt 200 ]; do
    focus_count=$(grep -c 'keyboard focus set to' "$displayd_log" || true)
    [ "$focus_count" -ge 2 ] && break
    attempt=$((attempt + 1))
    sleep 0.05
done
[ "${focus_count:-0}" -ge 2 ] || {
    printf '%s\n' "launched application did not receive focus" >&2
    cat "$displayd_log" >&2
    exit 1
}

stop=$(request '{"schema":1,"request_id":"host:stop","command":"stop","app_id":"org.saaios.demo-surface"}')
printf '%s' "$stop" | grep -q '"result":"stopped"'

attempt=0
while [ "$attempt" -lt 200 ]; do
    focus_count=$(grep -c 'keyboard focus set to' "$displayd_log" || true)
    [ "$focus_count" -ge 3 ] && break
    attempt=$((attempt + 1))
    sleep 0.05
done
[ "${focus_count:-0}" -ge 3 ] || {
    printf '%s\n' "stopping application did not restore previous focus" >&2
    cat "$displayd_log" >&2
    exit 1
}

kill -0 "$displayd_pid"
kill -0 "$appd_pid"
printf '%s\n' "application Wayland host vertical: PASS"
