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

run:
    cargo run -p saaios-runtime

run-tui:
    cargo run -p console-tui

e2e:
    cargo test -p diagnose-slow-system -- --nocapture

audit-show path="saaios-audit.jsonl":
    @if [ -f "{{path}}" ]; then cat "{{path}}"; else echo "No audit file at {{path}}"; fi

ci: fmt lint test e2e
