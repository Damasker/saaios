# ADR-308: panther shell carries host F chrome

## Статус

Принято, 2026-09-21. Host F chrome on the phone-gate shell.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-307 следующий свободный номер — **308**. Не S33.

## Контекст

MEM-08, ATTN-06, WORK-03 Result/Orb, Sistema «Узел», Restricted
omit, and the foreign OSK layer were host-green and unflashed
because the previous shell experiment was spent. Displayd still
owns DRM. Shell respawn does not need reboot; dest-no-lock lives
in `/run`.

## Decision

1. **One binary.** Replace `/data/saaios/system/saai-shell` with
   `4dc19018…`. Kill the shell only. displayd pid 3240 stays.
2. **IME stays optional.** panther displayd has no
   `zwp_input_method_manager_v2`. The new shell logs that and
   keeps the foreign OSK unmapped (ADR-273). Do not flash displayd.
3. **Leave Сейчас.** Nav is Сейчас · Пространства · Поиск · Система.
   Empty NOW is «Ничего срочного». No Inbox tab. No weather.

## Consequences

- Sistema Записи / Узел / Intent Result ride this binary; they
  were not a second screenshot this slice.
- Foreign OSK on Qt/GTK fields still waits a displayd IME
  experiment.
- Rollback: `saai-shell.pre-fchrome`.

## Verification

PUT `saai-shell`
`4dc19018ee5cbc089d316cecc555fc7686d4eefbdbec16f6d5dab986e2797ea8`.
Kill pid 6066; displayd respawn pid 6710. Log: `dev_no_lock
enabled`, `connected to saai-entityd`, `zwp_input_method_manager_v2
unavailable`. Screencap `n.bmp` 1080×2400: Дом · Сейчас, «Ничего
срочного», footer Сейчас · Пространства · Поиск · Система.
displayd pid 3240 unchanged. dest-no-lock kept. No reboot.
