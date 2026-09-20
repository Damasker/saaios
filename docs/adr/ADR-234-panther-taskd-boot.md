# ADR-234: panther — `saai-taskd` starts from native-init after reboot

## Статус

Принято, 2026-09-20. ADR-233 installed `/data/saaios/system/saai-taskd`
and wired `native-init.c`, but this boot had started the process by
hand. This slice flashes `init_boot_a` so PID 1 starts and restarts
it. PIN stays null. Not Visual v1 sign-off. Multi-space watch and
WORK-02 live dispatch stay later (A2/A4).

## Нумерация

После ADR-233 следующий свободный номер — **234**. Не S33.

## Контекст

ADR-233's verification recorded `native-init` wiring in tree only.
`/run/boot.log` on that boot had `native userspace ready` and no
`system service started: saai-taskd`. A reboot would drop the manual
pid. PIXEL-PATH P1 called the daemon live; boot ownership was still
open.

Live `init_boot_a` (`sda11`) matched R620
`saaios-panther-gpu2-init_boot.img` SHA-256 `85ce9d54…`. Only `/init`
changed: old `40a5221e…` (85856 bytes) → new `dcc1fcc1…` (86832
bytes, `start_saai_taskd` after `start_runtime`).

## Decision

1. **Flash.** `fastboot flash init_boot_a` of the gpu2 image with
   `/init` replaced. Slot A, unlocked. Other ramdisk payloads stay.
2. **Boot proof.** Cold reboot. `saai-taskd` must appear in
   `/run/boot.log` with `--space` from `selection.json` (here `work`)
   without a manual exec.
3. **Marker.** Restore `/run/saaios/dev-no-lock` after the reboot
   drops `/run`. Do not invent a PIN. Do not kill the locked shell.

## Consequences

- Reboot no longer drops workflow supervision. Rollback: reflash the
  previous `init_boot_a` (`85ce9d54…`) and stop `/data`'s taskd.
  Runtime space switches still do not retarget `--space` until A2.
  WORK-02 stays host-only until A4. Still not Visual v1 sign-off.

## Verification

Host: `native-init.c` already in `f27fb55`; zig `aarch64-linux-musl`
`/init` `dcc1fcc1…`. Panther: image `d52a48be…`; `/proc/uptime`
34.59s; `/init` `dcc1fcc1…`; boot.log `system service started:
saai-taskd space=work` then `native userspace ready`; pid 422
`--space work --runtime-addr 172.31.7.1:38127`. entityd 156, appd
158, file-recv 159, displayd 385, shell 402, runtime 421. Marker
restored. Do not tap Разрешить/Сопряжь. Do not 7-tap.
