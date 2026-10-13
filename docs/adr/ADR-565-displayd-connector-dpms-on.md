# ADR-565: displayd sets connector DPMS On

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-displayd` is
`2462a9e9…` (pixel7 musl, panther-hardware). Previous kept as
`saai-displayd.prev-adr565` (`481f7ab3…`). dest-no-lock stays
off. Unlock stays proven from ADR-564. Experimental. PIN stays
null. Not Visual v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-564 следующий свободный номер — **565**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

After dest-no-lock lift the panel went black. `saai-displayd` and
`saai-shell` stayed up and kept committing. Sysfs showed
`card0-DSI-1` `enabled=disabled` and `dpms=Off`, backlight
`bl_power=4`. Writing `dpms` from ash is Permission denied.

drm-splash `SETCRTC` lights the connector (`dpms=On`). displayd
atomic modeset does not. Cold displayd restart after a blanked
CRTC also fails `Atomic Test` on CRTC 175; init then stays on
splash fallback. A `reboot -f` brought displayd back as primary
(pid 410, shell 418). User confirmed the panel was visible again.
Sysfs `dpms` can stay `Off` while `enabled` and `bl_power=0`.

Shell does not blank the CRTC (S04 known limit). dest-no-lock
stays off.

## Решение

1. After a successful `present()`, displayd writes connector
   property `DPMS` = `DRM_MODE_DPMS_ON` (0) as DRM master.
2. First modeset logs `connector DPMS On` when the write lands.
3. Do not restore dest-no-lock. Do not invent a wake protocol.

Host proof: `rustc --test services/saai-displayd/src/dpms.rs`
(2 passed). `cargo test -p saai-displayd` needs libxkbcommon on this
WSL and is not this slice.

Panther: PUT+rename; kill 410 then 2528; displayd 2694; shell
2696; `connector DPMS On`; `hardware output initialized`;
`surface=lock` `backend=shm`; dest-no-lock absent; `bl_power=0`
`brightness=1023`; `enabled`. Sysfs `dpms` still reads `Off`
after the property write.

## Откат

`mv /data/saaios/system/saai-displayd.prev-adr565 /data/saaios/system/saai-displayd`
then kill the current displayd. Init restarts the previous binary.

## Ссылки

- ADR-005, ADR-010, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
