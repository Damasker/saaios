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
- `/etc/systemd/system/saaios-runtime.service`
- state dir `/var/lib/saaios`
- runtime socket `/run/saaios/saaios.sock`

Default appliance mode uses `--real-linux` tools and `SAAIOS_MOCK=1` so the service starts without an API key. To use a remote model, edit the unit environment:

```ini
Environment=SAAIOS_API_BASE=https://api.openai.com/v1
Environment=SAAIOS_API_KEY=...
Environment=SAAIOS_MODEL=gpt-4o-mini
# remove SAAIOS_MOCK=1
```

Then:

```bash
sudo systemctl restart saaios-runtime
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
