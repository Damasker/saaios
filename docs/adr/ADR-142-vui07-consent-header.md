# ADR-142: VUI-07 consent — ContextHeader + requested DataRow

## Статус

Принято, 2026-09-19. App-consent paints through `ContextHeader` and
Static `DataRow`s instead of a free-floating title at `header.y+220`
and accent-square bullets. Host-green; panther `cff22339…`.
Do not tap **Разрешить**. Screenshot via live launch, then **Отклонить**.
Remote pairing, PIN keypad chrome, and Space detail are not this slice.

## Нумерация

После ADR-141 следующий свободный номер — **142**. Не S33.

## Контекст

VUI-07 remaining modal after all four root tabs use `ContextHeader`.
Consent already owns a real `consent_view` (header pane + accept/decline
hit rects, ADR-020) but still paints `{app} запрашивает доступ` at a
fixed offset and each capability as a 16px accent square. That sits
under the status clock the same way `draw_root` did. `DecisionOverlay`
names a workflow/OAM choice, not requested capabilities — do not reuse
it here. Do not invent scopes. Do not restyle remote pairing or the
PIN keypad in the same slice.

## Decision

1. **Header** is `ContextHeader` with section `Разрешение`. Space name
   is the live selected space. No invented lifecycle.
2. **Rows** are Static `DataRow` flattened to `ActionCardView`: first
   the live app name with status `запрашивает доступ`; then each
   requested capability label (same `capability_label` vocabulary).
   Empty requested set is one row `Без дополнительных разрешений`.
3. **Buttons stay** `consent_view` accept/decline rects. Labels stay
   `Разрешить` / `Отклонить`. Hit-test is unchanged.

## Consequences

- Consent is a real section, not a diagnostic overlay.
- Decline still records the decision and retries launch (existing
  `ConsentDecided` path). Screenshot happens before that tap.
- Rollback: restore `draw_consent`'s title + bullet loop.

## Verification

Host: heading is `{space} · Разрешение`; empty requested is named;
button hit-test unchanged. Panther: live launch shows the list below
the clock. Only **Отклонить** is tapped.
