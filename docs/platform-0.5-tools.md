# Platform 0.5 — richer system tools

## New / expanded tools

| Tool | Risk | Notes |
|---|---|---|
| `system.temperature` | Low | Thermal zones / hwmon; mock = 62.5C |
| `system.journal` | Low | `journalctl` tail (`lines`, optional `unit`); dmesg fallback |
| `network.status` | Low | operstate + ipv4 + rx/tx bytes + default gateway |
| `process.kill_request` | High | **Real SIGTERM/KILL** on Linux after confirm; refuses pid≤1 and self |

## Kill safety

1. Policy denies `pid <= 1` before AskUser
2. Tool refuses pid ≤ 1 and the runtime's own pid
3. Allowed signals: `TERM` (default), `KILL`, `HUP`, `INT`, `QUIT`, `USR1`, `USR2`
4. Confirmation still required (`y` / `s`)

Mock mode still simulates kill without signaling.

## Automation

Rule `high-temp-notify` fires when `system.temperature.celsius >= 80`.
