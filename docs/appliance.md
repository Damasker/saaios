# Raspberry Pi / Linux appliance

SaaiOS Platform runs as a userspace service on Linux. Own kernel is out of scope here.

## Build on the host (x86_64)

```bash
cargo build -p saaios-runtime -p console-tui --release
sudo ./deploy/install.sh
```

## Cross-compile for Raspberry Pi (aarch64)

```bash
# needs one of: gcc-aarch64-linux-gnu | cargo-zigbuild+zig | cross+Docker
./deploy/cross-pi.sh
./deploy/package.sh target/aarch64-unknown-linux-gnu/release
# copy dist/saaios-*.tar.gz to the Pi, extract, then:
sudo ./install.sh
```

Or with an explicit linker:

```bash
sudo apt-get install -y gcc-aarch64-linux-gnu
SAAIOS_CROSS=gnu ./deploy/cross-pi.sh
```

## Install systemd unit

```bash
sudo ./deploy/install.sh
# or: sudo ./deploy/install.sh /path/to/saaios-runtime
```

This installs:

- `/usr/local/bin/saaios-runtime`
- `/usr/local/bin/saaios-console` (when built next to the runtime)
- `/etc/saaios/saaios.toml` (from `deploy/saaios.toml`, if missing)
- `/etc/systemd/system/saaios-runtime.service`
- A/B helpers under `/usr/local/lib/saaios/`
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

## Package tarball

```bash
just package                 # host release
just package-pi              # after cross-pi
```

Produces `dist/saaios-<version>-<stamp>.tar.gz` with binaries + deploy scripts + top-level `install.sh`.

## A/B updates + BOOT_OK

See [Platform 0.3](platform-0.3.md). Quick path:

```bash
sudo SAAIOS_AB=1 ./deploy/install.sh
sudo /usr/local/lib/saaios/install-slot.sh B ./target/release/saaios-runtime
sudo /usr/local/lib/saaios/switch.sh B
sudo /usr/local/lib/saaios/status.sh
```

Runtime `status` (TUI `h`) also reports A/B layout when `SAAIOS_AB_ROOT` (default `/var/lib/saaios/ab`) exists:

```json
"ab": {
  "enabled": true,
  "current": "A",
  "slots": [
    {"name":"A","active":true,"binary_present":true,"boot_ok":true,"boot_ok_at":"..."},
    {"name":"B","active":false,"binary_present":false,"boot_ok":false,"boot_ok_at":null}
  ]
}
```

`boot-ok.sh` pings `{"op":"ping"}` on the UDS socket before writing `BOOT_OK`.

## Connect TUI

```bash
SAAIOS_SOCK=/run/saaios/saaios.sock saaios-console
# or
SAAIOS_SOCK=/run/saaios/saaios.sock ./target/release/saaios-console
```

## Replay an audit chain (dry-run)

```bash
just audit-replay <correlation-id> /var/lib/saaios/audit.jsonl
```

Replay never executes tools; dangerous calls are listed under `side_effects_avoided`.
