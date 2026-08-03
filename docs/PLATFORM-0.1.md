# Platform 0.1 status

## Done in this branch

- Cargo workspace + just + CI
- ADR-001/002/003
- CBOR protocol v1
- Audit log
- Tool registry + mock/real system tools
- Policy engine (allow / ask / deny + injection guard)
- Event bus (in-process broadcast)
- Mock + OpenAI-compatible model providers
- AI runtime tool loop
- UDS runtime server
- TUI console
- e2e: diagnose-slow-system

## How to demo

```bash
just e2e
just run-mock   # terminal 1
just run-tui    # terminal 2
```

Ask: `Почему система тормозит?`
Confirm/cancel proposed kill with `y` / `n`.

## Added after 0.1 bootstrap

- Real Linux `/proc` adapter with sampled CPU% + VmRSS
- Richer `network.status` (interface operstate)
- Audit dry-run replay (`--replay` / `just audit-replay`)
- systemd unit + `deploy/install.sh` for Pi/Linux appliance
- `just run-linux` / `just e2e-linux`
