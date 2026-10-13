# ADR-563: lock PIN field is not a compose header

## Статус

Принято, 2026-10-13. Host. dest-no-lock restored after the lift.
Unlock is not proven. Experimental. PIN stays null. Not Visual
v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-562 следующий свободный номер — **563**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

User asked to lift `/run/saaios/dev-no-lock`. Live shell `035db4fa…`
requested `ext-session-lock-v1`. Lock surface configured 1080×2400.
No one-slot SIGTRAP (ADR-532 holds). `present_lock_pin_entry` then
panicked: `ADR-434 compose overlay: missing ContextHeader`.

`lock_pin_field_rect` always called `overlay_field_paint("lock", …)`.
`layout_v2` treats `screen.id == "lock"` as `v2_lock_field_node`
(ADR-232 field slot, no ContextHeader). No-PIN lock still computed
that rect. Shell SIGABRT 3/3; displayd exited; init fell back to
`drm-splash`. dest-no-lock was written back. displayd 24060 and
shell 24062 came back under the marker; `surface=now`.

## Решение

1. `overlay_field_paint` does not require `ContextHeader` when the
   screen is `lock`. The field slot stays `v2_lock_field_node`.
2. No-PIN lock does not compute `lock_pin_field_rect`.
3. dest-no-lock stays until this binary is on `/data` and lock
   paints without abort. Do not convert leftover lock formulas.

## Откат

Require ContextHeader for every overlay field, including `lock`.

## Ссылки

- ADR-232, ADR-434, ADR-512, ADR-532, ADR-562
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
