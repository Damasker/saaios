# ADR-131: VUI-07 trusted-client list — TrustedClientRow

## Статус

Принято, 2026-09-18. «Доверенные клиенты» lists live `authorized_keys`
rows through `TrustedClientRow`. Flashed panther `cd207b18…`: title
below the status layer, `3 доверенных ключей`, `test-client` /
`home-server-test` / `fast-attempt-5` with SHA256 prefixes and
`Отозвать`, trailing «Назад». Wi-Fi password and lock restyle are
not this slice.

## Нумерация

После ADR-130 следующий свободный номер — **131**. Не S33.

## Контекст

VUI-07 next remaining surface after the Bluetooth list. Система
already opens this screen (`SettingRow::open` →
`open_trusted_clients`). The destination is still ADR-084
`draw_row_list`: one concatenated label (`name · fingerprint… ·
Отозвать`) plus trailing «Назад». The file is the only source of
truth (`docs/os/ideas.md`); the shell re-reads it every draw because
a revoke mutates that same file. There is no `TrustedClientRow`.
Do not invent a live-session bit, a full public key, a second
pairing frame, or a Space binding.

Keep revoke behavior: tap still runs `revoke_trusted_client(index)`
with no extra confirm (ADR-084). Fingerprint stays ADR-083 SHA256
prefix, not the raw key. Dev surface and the password keyboard stay
on `draw_row_list`.

## Decision

1. **`TrustedClientRow` wraps `DataRow`**. Live row: Navigation,
   primary = comment-field name (`(без имени)` when that field is
   missing), value = 24-char fingerprint prefix plus ellipsis.
   `revoke_trusted_client` is the action. Empty file (or missing
   file): Static «Нет клиентов». Missing file is an honest empty
   list, not a second error string — the screen already opened from
   Система, which named the SSH capability.
2. **Flatten to `ActionCardView`** (`Отозвать`; empty has no button).
   Hit-test still uses `stacked_row_rect`. Empty is not tappable.
   Back stays a trailing control card after the `TrustedClientRow`
   list (index shifts by one only when empty).
3. **Paint through `draw_action_row_list`**, same header inset as
   Wi-Fi / Bluetooth. Dev surface stays on `draw_row_list`.

## Consequences

- Name vs fingerprint stay separate fields, so two same-named
  clients remain distinguishable without stuffing «Отозвать» into
  the label.
- Rollback: restore concatenated labels in `Frame::TrustedClients`.

## Verification

Host: TrustedClientRow constructors; live list order and revoke
button; empty names the absence and is not tappable; Back after the
row list; no invented session/key material. Panther `cd207b18…`:
three live keys with SHA256 prefixes, no raw key material.
