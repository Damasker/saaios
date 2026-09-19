# ADR-175: VUI-08 — FramePace surface traces

## Статус

Принято, 2026-09-20. Each commit is tagged with `FrameSurface`. The
32-sample ring dumps to `/run/saaios/shell-frame.trace`. Overlay and
Orb tags are host-proven; panther covers me/scroll, tabs, apps list,
and intent keyboard. No new daemon. Do not tap Inbox/Spaces rows,
Saai Demo, PIN, or a network. Leave Отмена, leave Сейчас.

## Нумерация

После ADR-174 следующий свободный номер — **175**. Не S33.

## Контекст

ADR-172/173 record produce time and a scroll-only p95, but the last
line cannot show which chrome produced the sample. VUI-08 still needs
traces for Система drag, fast tabs, lists, keyboard, overlays, and
Orb. Presentation timestamps stay omitted.

## Decision

1. **`FrameSurface`** next to `FrameReason`: `now` / `inbox` / `spaces`
   / `me` / `list` / `keyboard` / `overlay` / `orb` / `lock`.
   `frame_surface` prefers lock, then overlay, keyboard, list, orb,
   then the current tab. Orb is only the activity-clock commit, not a
   scroll.
2. **Each `FrameSample` carries `surface`.** `.last` appends
   `surface=`. `/run/saaios/shell-frame.trace` is the ring oldest to
   newest. Missing `/run` is a silent no-op.
3. **Panther** swipes Система, switches tabs, opens Приложения then
   closes it, opens compose and Отмена, `cat`s the trace, leaves
   Сейчас. Overlay/Orb without a live Running task or consent stay
   host-only.

## Consequences

- A mixed session is readable without guessing from `reason` alone.
  Rollback: drop `surface` and the `.trace` write.

## Verification

Host: classifier priority; chronological dump names each surface.
Panther: `surface=me` / `inbox` / `spaces` / `list` / `keyboard` in
the trace, leave Сейчас.
