# Platform 0.5 — TOML configuration

## Purpose

One file for runtime settings instead of only env/CLI scatter.

## Discovery

1. `--config path` / `SAAIOS_CONFIG`
2. `./saaios.toml`
3. `/etc/saaios/saaios.toml`
4. built-in defaults

## Precedence

`defaults → TOML file → env (provider secrets/endpoints) → CLI flags`

CLI still wins for explicit flags (`--mock`, `--real-linux`, `--no-memory`, …).

## Schema

See `saaios.example.toml` and `deploy/saaios.toml`.

| Section | Keys |
|---|---|
| `runtime` | `sock`, `audit`, `real_linux`, `mock_planner` |
| `provider` | `kind`, `api_base`, `api_key`, `model`, `local_base`, `local_model` |
| `budgets` | `max_concurrent`, `request_timeout_secs`, `max_tool_iters` |
| `memory` | `enabled`, `path` |
| `automation` | `enabled` |
| `telemetry` | `mode` (`auto`/`on`/`off`), `interval_secs` |

## Examples

```bash
# Use example config
cargo run -p saaios-runtime -- --config saaios.example.toml --mock

# Appliance file
sudo cp deploy/saaios.toml /etc/saaios/saaios.toml
just run-linux
```

`Status` (TUI `h`) includes `config_path` when a file was loaded.
