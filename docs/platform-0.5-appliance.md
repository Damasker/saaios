# Platform 0.5 — appliance packaging polish

## Cross-compile (Pi aarch64)

```bash
./deploy/cross-pi.sh          # auto-detect linker (gnu|zig|cross)
just package-pi
```

## Package + install

```bash
cargo build -p saaios-runtime -p console-tui --release
just package
# on target host:
sudo ./install.sh             # from extracted tarball
```

`install.sh` now also installs `saaios-console` when present next to the runtime binary.

## A/B in runtime status

`ClientRequest::Status` includes `ab` from `SAAIOS_AB_ROOT` (default `/var/lib/saaios/ab`):

- `enabled` — layout directory exists
- `current` — active slot
- per-slot `binary_present` / `boot_ok` / `boot_ok_at`

TUI `h` prints a one-line A/B summary.

## BOOT_OK health check

`deploy/ab/boot-ok.sh` sends `{"op":"ping"}` over the UDS socket (Python3) before writing `BOOT_OK`. Falls back to socket presence if Python is missing.
