# Platform 0.6 — Stage GUI (AI-native)

## Idea

Not a classic admin dashboard. **SaaiOS Stage** is a single conversation surface:

- The brand and prompt dominate the first viewport
- Tools appear as progressive *steps*, not menu trees
- Dangerous actions open a **Decision** overlay (once / session / cancel)
- Ambient presence orb shows runtime health — not a stats strip

TUI remains for headless / SSH. Stage is for local browser / kiosk on the appliance.

## Run

```bash
just run-mock          # terminal 1 — runtime
just run-stage         # terminal 2 — http://127.0.0.1:7420
just run-stage-kiosk   # kiosk layout
just run-stage-lan     # 0.0.0.0 + token (set SAAIOS_STAGE_TOKEN)
```

Env / flags:

| Var / flag | Default | Notes |
|---|---|---|
| `SAAIOS_SOCK` | `/tmp/saaios.sock` | Runtime UDS |
| `SAAIOS_STAGE_BIND` | `127.0.0.1:7420` | Localhost bind |
| `--lan` / `SAAIOS_STAGE_LAN` | off | Bind `0.0.0.0:$PORT` |
| `--port` / `SAAIOS_STAGE_PORT` | `7420` | Used with `--lan` |
| `--token` / `SAAIOS_STAGE_TOKEN` | unset | Required with `--lan` unless `--allow-open` |
| `--tls-cert` / `--tls-key` | unset | Enable HTTPS |
| `--kiosk` | off | Default kiosk layout (`/kiosk` always available) |

Binary: `saaios-stage` (`crates/console-web`).

### LAN + token

```bash
SAAIOS_STAGE_TOKEN=labsecret just run-stage-lan
# open http://<host>:7420/?token=labsecret  (sets HttpOnly cookie)
# or: Authorization: Bearer labsecret
```

### TLS

```bash
saaios-stage --lan --token secret \
  --tls-cert /etc/saaios/stage.crt \
  --tls-key /etc/saaios/stage.key
```

## Bridge

Stage talks to the same UDS JSON API as the TUI:

- `POST /api/diagnose` → UDS `diagnose` with `stream: true` (NDJSON)
- `POST /api/confirm` → confirm scopes
- `GET /api/status|events|grants`
- `POST /api/chat/reset`

Chat `session_id` is held in the Stage process (one browser ↔ one conversation).

## systemd

Optional unit: `deploy/systemd/saaios-stage.service` (localhost by default).
Enable LAN via drop-in env (`SAAIOS_STAGE_LAN=1`, `SAAIOS_STAGE_TOKEN=…`).

## Non-goals

- Full account auth / OAuth
- Native toolkit (egui/iced)
- Voice
