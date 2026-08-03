# Platform 0.4 — observability & proactive telemetry

## Runtime status

UDS op `Status` returns health snapshot:

- provider name / kind
- tools mode + tool list
- memory / automation / telemetry flags
- budgets, uptime, session grants

TUI: press `h`.

## Telemetry sampler

`telemetry` crate periodically runs `system.metrics` and publishes a `ToolResult`
envelope on the event bus (also audited). Existing automation rules can fire
without a user diagnose request.

Enabled when:

- `--real-linux` (appliance default path), or
- `--telemetry` / `SAAIOS_TELEMETRY=1`

Disabled with `--no-telemetry`. Interval: `--telemetry-interval-secs` (default 30).

Mock mode leaves telemetry **off** by default so the 97% CPU fixture does not
spam automation during demos.

## system.disk

New low-risk tool. Mock fixture reports `root_used_pct=70`. Linux path parses
`df -Bk` for `/`, `/home`, `/var`, `/tmp`.

## Automation

Added `high-mem-notify` when `system.metrics.mem_used_pct >= 90`.
`mem_used_pct` is now part of `SystemMetrics`.
