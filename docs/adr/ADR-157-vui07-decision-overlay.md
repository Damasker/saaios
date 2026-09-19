# ADR-157: VUI-07 DecisionOverlay on Object View

## Статус

Принято, 2026-09-19. Confirmation is the already-named `DecisionOverlay`
composite, not a new SurfacePattern. Object View stops flattening it
into `details` Caption lines. Permission and recovery are not this
slice. Do not tap Подтвердить or Отклонить.

## Нумерация

После ADR-156 следующий свободный номер — **157**. Не S33.

## Контекст

ADR-120/137 already build `DecisionOverlay` while a Task/Action is
`waiting_confirmation`. `object_view_details` joins `fact_lines()`
into the same 28px secondary dump as activity and OAM permission.
The two buttons stay generic action labels. Component-library 7.8
says this is a Dialog with `Button`s, distinct from a list row.

App-consent stays ADR-142 (`ContextHeader` + Static `DataRow`).
That is permission, not confirmation.

## Decision

1. **`Frame::ObjectView` carries `Option<DecisionOverlay>`**. Identity
   stays `ObjectSummary`. Overlay facts are not copied into `details`.
2. **Paint facts as Body**, then the overlay's `accept` / `decline`
   `Button` labels on the existing two-button geometry. Do not paint
   `heading()` — that duplicates the summary title.
3. **Do not tap Подтвердить / Отклонить.** Panther proof opens Object
   View from NOW when a waiting task exists. Leave by an unlocked
   shell restart with `dev-no-lock`, not by deciding.

## Consequences

- Confirmation is a named Dialog, not a vanished subtitle or a
  Caption dump.
- Rollback: restore `fact_lines()` into `details`.

## Verification

Host: waiting confirmation keeps `DecisionOverlay` facts off
`object_view_details`; the two buttons still hit-test as before.
Panther: screenshot Object View with facts + Подтвердить / Отклонить.
Do not tap either.
