# Raspberry Pi / Linux appliance

SaaiOS Platform runs as a userspace service on Linux. Own kernel is out of scope here.

## Build on the Pi (or cross-compile)

```bash
cargo build -p saaios-runtime --release
cargo build -p console-tui --release
```

## Install systemd unit

```bash
sudo ./deploy/install.sh
```

This installs:

- `/usr/local/bin/saaios-runtime`
- `/etc/saaios/saaios.toml` (from `deploy/saaios.toml`, if missing)
- `/etc/systemd/system/saaios-runtime.service`
- state dir `/var/lib/saaios`
- runtime socket `/run/saaios/saaios.sock`

Default unit: `--config /etc/saaios/saaios.toml` plus `SAAIOS_MOCK=1`.

Edit `/etc/saaios/saaios.toml` for provider/telemetry/memory; see [Platform 0.5 config](platform-0.5-config.md).

Providers (`SAAIOS_PROVIDER` / `--provider` / `[provider].kind`): `mock` | `remote` | `local` | `auto`.

```toml
[provider]
kind = "auto"
local_base = "http://127.0.0.1:11434/v1"
local_model = "llama3.2"
# optional remote fallback via env:
# SAAIOS_API_BASE / SAAIOS_API_KEY / SAAIOS_MODEL
```

Then:

```bash
sudo systemctl restart saaios-runtime
```

## A/B updates + BOOT_OK

See [Platform 0.3](platform-0.3.md). Quick path:

```bash
sudo SAAIOS_AB=1 ./deploy/install.sh
sudo /usr/local/lib/saaios/install-slot.sh B ./target/release/saaios-runtime
sudo /usr/local/lib/saaios/switch.sh B
sudo /usr/local/lib/saaios/status.sh
```

## Connect TUI

```bash
SAAIOS_SOCK=/run/saaios/saaios.sock ./target/release/saaios-console
```

## Replay an audit chain (dry-run)

```bash
just audit-replay <correlation-id> /var/lib/saaios/audit.jsonl
```

Replay never executes tools; dangerous calls are listed under `side_effects_avoided`.
