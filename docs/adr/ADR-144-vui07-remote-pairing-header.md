# ADR-144: VUI-07 remote pairing — ContextHeader, not a free-floating title

## Статус

Принято, 2026-09-19. The SSH pairing prompt sits under a real
`ContextHeader` instead of `header.y+200` title text. Host-green;
panther `4a1f7553…`. Do not tap **Разрешить**. Leave with
**Отклонить**. Lock unlock stays `draw_lock_pin_entry` dots. Space
detail is not this slice.

## Нумерация

После ADR-143 следующий свободный номер — **144**. Не S33.

## Контекст

VUI-07 remaining modal chrome after PIN setup. `draw_remote_pair`
still paints «Разрешить SSH-доступ?» as raw text on a Surface-less
canvas using `task_confirm_view`'s header rect. Consent already uses
`ContextHeader` plus requested `DataRow`. Pairing is the same
header-plus-two-buttons shape; DecisionOverlay is not this screen
(workflow/OAM vs a live `pair-recv` key). Hit-test stays
`task_confirm_action_at`. Do not grant SSH. Do not invent a
fingerprint.

## Decision

1. **`Frame::RemotePairing` carries `ContextHeader`**. Section title
   is `SSH`. Space name is the live selected space. No invented
   lifecycle.
2. **The live client name is a Static `DataRow`** in the first stacked
   row. The fingerprint stays wrapped mono text below it — the full
   `SHA256:` string must remain readable, not clipped into a one-line
   status.
3. **Buttons stay** Разрешить (Accent) / Отклонить (Surface).

## Consequences

- Pairing is a named section, not a floating question.
- Rollback: restore the `header.y+200` title and client/fingerprint
  raw text.

## Verification

Host: heading is `{space} · SSH`; no Surface bar at y=210. Panther:
a live socket request shows the client and fingerprint under the
clock. Only **Отклонить** is tapped.
