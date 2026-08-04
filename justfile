set shell := ["bash", "-cu"]

default: test

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

build:
    cargo build --workspace

run-mock:
    SAAIOS_MODE=mock cargo run -p saaios-runtime -- --mock

run-linux:
    cargo run -p saaios-runtime -- --real-linux --mock

run-local:
    cargo run -p saaios-runtime -- --real-linux --provider local

run-auto:
    cargo run -p saaios-runtime -- --real-linux --provider auto

run-telemetry:
    cargo run -p saaios-runtime -- --real-linux --mock --telemetry --telemetry-interval-secs 5

run-config:
    cargo run -p saaios-runtime -- --config saaios.example.toml --mock

run-auto-diagnose:
    cargo run -p saaios-runtime -- --real-linux --mock --telemetry --telemetry-interval-secs 5 --auto-diagnose

run:
    cargo run -p saaios-runtime -- --real-linux --provider remote

run-tui:
    cargo run -p console-tui

run-stage:
    cargo run -p console-web

run-stage-kiosk:
    cargo run -p console-web -- --kiosk

run-stage-lan:
    # Requires SAAIOS_STAGE_TOKEN (or pass --allow-open for lab only).
    cargo run -p console-web -- --lan --kiosk --token "${SAAIOS_STAGE_TOKEN:?set SAAIOS_STAGE_TOKEN}"

cross-pi:
    bash deploy/cross-pi.sh

package bin_dir="target/release":
    bash deploy/package.sh "{{bin_dir}}"

package-pi:
    bash deploy/package.sh target/aarch64-unknown-linux-gnu/release

e2e:
    cargo test -p diagnose-slow-system -- --nocapture

e2e-linux:
    cargo test -p system-tools real_linux_metrics_smoke -- --nocapture

ab-layout root="/tmp/saaios-ab":
    SAAIOS_AB_ROOT="{{root}}" bash deploy/ab/layout.sh

ab-status root="/tmp/saaios-ab":
    SAAIOS_AB_ROOT="{{root}}" bash deploy/ab/status.sh

audit-show path="saaios-audit.jsonl":
    @if [ -f "{{path}}" ]; then cat "{{path}}"; else echo "No audit file at {{path}}"; fi

audit-replay id path="saaios-audit.jsonl":
    cargo run -q -p saaios-runtime -- --audit "{{path}}" --replay "{{id}}"

ci: fmt lint test e2e
