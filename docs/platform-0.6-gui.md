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
```

Env:

| Var | Default |
|---|---|
| `SAAIOS_SOCK` | `/tmp/saaios.sock` |
| `SAAIOS_STAGE_BIND` | `127.0.0.1:7420` |

Binary: `saaios-stage` (`crates/console-web`).

## Bridge

Stage talks to the same UDS JSON API as the TUI:

- `POST /api/diagnose` → UDS `diagnose` with `stream: true` (NDJSON)
- `POST /api/confirm` → confirm scopes
- `GET /api/status|events|grants`
- `POST /api/chat/reset`

Chat `session_id` is held in the Stage process (one browser ↔ one conversation).

## Non-goals (this slice)

- Auth / TLS / bind-to-LAN by default (localhost only)
- Native toolkit (egui/iced) — web is enough for Pi kiosk + laptop
- Voice
- Token-level LLM SSE (tool/assistant progress already streams)
