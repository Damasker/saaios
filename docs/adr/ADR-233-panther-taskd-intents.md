# ADR-233: panther — `saai-taskd` supervises live intents

## Статус

Принято, 2026-09-20. Free-form `saaios.intent` on panther was persisted
by `saai-shell` and then stalled at Object View «Нет задачи» /
«Нет исполнения». `saai-entityd` and `saaios-runtime` were already
live. `saai-taskd` was not installed, not in `native-init.c`, and
WORK-02 stayed host-only. This slice installs the binary on `/data`
and starts it against the selected space plus the on-device runtime.
PIN stays null. Not Visual v1 sign-off.

## Нумерация

После ADR-232 следующий свободный номер — **233**. Не S33.

## Контекст

ADR-030/033 already define the workflow: `saai-taskd` watches one
space, turns each `saaios.intent` into Task/Action via IRAB or the
`saaios-runtime` diagnose bridge, and never auto-executes
`WaitingConfirmation`. ADR-032 proved that path with a manual
process. PIXEL-PATH left boot wiring until the daemon existed on
`/data`. Live Work space intent `install firexox` (`ff76bfec…`) is
the physical trigger.

## Decision

1. **Binary.** `/data/saaios/system/saai-taskd`, same persistent
   delivery as `saai-entityd`. Outside init_boot.
2. **Live start.** `--entityd-socket /run/saaios/entityd.sock`
   `--space` from `selection.json` (fallback `home`) `--runtime-addr
   172.31.7.1:38127` — the address `saaios-runtime` already binds.
3. **Boot.** `native-init.c` starts and restarts it after entityd and
   runtime. Missing binary logs and skips; it is not a boot-critical
   failure.
4. **One space.** Multi-space watch stays later. Reconcile picks up
   intents created while the daemon was down.

## Consequences

- Object View can show a Task/Action for a typed intent. Runtime
  unreachable fails the Task instead of crashing the daemon
  (ADR-033). Dangerous delete still waits for live confirm
  (ADR-032). Rollback: stop the process and `rm` the binary; restore
  the previous `native-init.c`. Still not Visual v1 sign-off.

## Verification

Host: `cargo test -p saai-taskd` unchanged. Panther: binary
`1530507a…`; pid 4742; `--space work`; existing intent `ff76bfec…`
became Task `81154eac…`. Runtime accepted `diagnose` then hit the
60s request budget (`correlation_id=67d7c8ea…`). Сейчас shows
«Задача: install firexox» Failed, not «Нет задачи». Do not tap
Разрешить/Сопряжь. Native-init wiring is in tree; this boot started
the process from `/data`. Boot ownership is ADR-234.
