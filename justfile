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

run:
    cargo run -p saaios-runtime -- --real-linux --provider remote

run-tui:
    cargo run -p console-tui

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
