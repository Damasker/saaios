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

Default appliance mode uses `--real-linux` tools and `SAAIOS_MOCK=1` so the service starts without an API key.

Providers (`SAAIOS_PROVIDER` / `--provider`): `mock` | `remote` | `local` | `auto`.

```ini
# Prefer local Ollama, fall back to remote/mock
Environment=SAAIOS_PROVIDER=auto
Environment=SAAIOS_LOCAL_BASE=http://127.0.0.1:11434/v1
Environment=SAAIOS_LOCAL_MODEL=llama3.2
# Optional remote fallback:
# Environment=SAAIOS_API_BASE=https://api.openai.com/v1
# Environment=SAAIOS_API_KEY=...
# Environment=SAAIOS_MODEL=gpt-4o-mini
# remove SAAIOS_MOCK=1
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
