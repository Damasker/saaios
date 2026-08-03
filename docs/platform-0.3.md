# Platform 0.3 — local models and A/B BOOT_OK

## Model providers

`model-provider` supports:

| Kind | Flag / env | Behavior |
|---|---|---|
| `mock` | `--provider mock` / `SAAIOS_MOCK=1` | Deterministic diagnose path |
| `remote` | `--provider remote` | OpenAI-compatible HTTP (`SAAIOS_API_*`) |
| `local` | `--provider local` | Ollama OpenAI-compat at `SAAIOS_LOCAL_BASE` |
| `auto` | `--provider auto` (default) | Local if healthy → remote if configured → mock |

Env:

```bash
# Remote
export SAAIOS_API_BASE=https://api.openai.com/v1
export SAAIOS_API_KEY=...
export SAAIOS_MODEL=gpt-4o-mini

# Local (Ollama)
export SAAIOS_LOCAL_BASE=http://127.0.0.1:11434/v1
export SAAIOS_LOCAL_MODEL=llama3.2
```

Examples:

```bash
just run-mock
just run-local          # --provider local --real-linux
just run-auto           # prefer Ollama, else remote/mock
```

`auto` builds a `FallbackProvider` chain so a downed Ollama does not brick the appliance.

## A/B slots + BOOT_OK

Appliance updates use two slots under `/var/lib/saaios/ab`:

```
ab/
  A/bin/saaios-runtime
  B/bin/saaios-runtime
  current -> A|B
  shared/
```

Workflow:

```bash
sudo ./deploy/ab/layout.sh
sudo ./deploy/ab/install-slot.sh B ./target/release/saaios-runtime
sudo ./deploy/ab/switch.sh B
# after healthy boot:
sudo ./deploy/ab/boot-ok.sh
sudo ./deploy/ab/status.sh
```

`boot-ok.sh` waits for the UDS socket and writes `BOOT_OK` on the active slot.
`switch.sh` refuses to leave a slot that never marked `BOOT_OK` unless `--force`.

systemd oneshot: `deploy/systemd/saaios-boot-ok.service` (enabled by `deploy/install.sh`).

Seed layout during install:

```bash
sudo SAAIOS_AB=1 ./deploy/install.sh
```
