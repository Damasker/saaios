# ADR-565: displayd sets connector DPMS On

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven from ADR-564. Experimental. PIN stays null. Not Visual
v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-564 следующий свободный номер — **565**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

After dest-no-lock lift the panel went black. `saai-displayd` and
`saai-shell` stayed up and kept committing (`surface=lock`, then
after reboot `hardware output initialized`). Sysfs showed
`card0-DSI-1` `enabled` and `dpms=Off`. Backlight `bl_power=0`
`brightness=1023`. Writing `dpms` from ash is Permission denied.

drm-splash `SETCRTC` lights the connector (`dpms=On`). displayd
atomic modeset does not. Cold displayd restart after a blanked
CRTC also fails `Atomic Test` on CRTC 175; init then stays on
splash fallback. A `reboot -f` brought displayd back as primary
(pid 410, shell 418) with the same Off connector — black again.

Shell does not blank the CRTC (S04 known limit). dest-no-lock
stays off.

## Решение

1. On every `present()`, displayd writes connector property
   `DPMS` = `DRM_MODE_DPMS_ON` (0) as DRM master.
2. First modeset logs `connector DPMS On` when the write lands.
3. Do not restore dest-no-lock. Do not invent a wake protocol.

Host proof: `rustc --test services/saai-displayd/src/dpms.rs`
(2 passed). `cargo test -p saai-displayd` needs libxkbcommon on this
WSL and is not this slice.

## Откат

Stop writing connector DPMS from `present()`.

## Ссылки

- ADR-005, ADR-010, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
