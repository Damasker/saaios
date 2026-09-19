# ADR-155: VUI-07 SurfacePattern — empty, loading, offline

## Статус

Принято, 2026-09-19. Shared empty / loading / offline is one
`SurfacePattern` (caller-supplied message + `UniversalState`). First
consumers: NOW empty/offline, apps-grid empty/offline, Bluetooth scan
loading. Flashed panther `e18f3d9d…`: Bluetooth list with no scan log
paints `Сканирование…`. Blocked, failed, permission, confirmation, and
recovery are not this slice. Space detail still deferred. Do not tap
Сопрячь.

## Нумерация

После ADR-154 следующий свободный номер — **155**. Не S33.

## Контекст

VUI-07 still lists shared empty / loading / offline / blocked / failed
/ permission / confirmation / recovery as one remaining task. Empty
and offline already exist as local copy (NOW `Ничего срочного` vs
`Нет связи с пространствами`; apps grid `Нет приложений` vs `Нет
связи`; list composites as Static DataRows). Loading does not: a
Bluetooth scan in progress with no devices yet paints a blank under
the header. ADR-130 named that absence as "no placeholder." The blank
is the dishonest hole — scan is Waiting, not Idle.

There is no `SurfacePattern` type. Inventing a second loading painter
in the Bluetooth frame would freeze the same split the rest of VUI-07
has been closing.

## Decision

1. **`SurfacePattern { state, message }`**. Callers supply the
   message. `empty` is `Idle` and paints no mark. `loading` is
   `Waiting`. `offline` is `Offline`. Busy accessibility follows
   Waiting.
2. **`draw_surface_pattern`** centers Body text in the content rect.
   Non-Idle states add the compact `StatusMark` above the line. Idle
   stays text-only, matching HIA-13's calm empty.
3. **NOW and the apps grid** construct the pattern instead of a raw
   string. Copy does not change.
4. **Bluetooth** occupies slot 0 while `devices` is empty:
   `loading("Сканирование…")` until `DONE`, then `empty("Нет
   устройств")`. Controls stay trailing. Slot 0 is not Искать and is
   not Сопрячь.

## Consequences

- Loading is a named Waiting state, not a blank and not a fake device.
- Empty Idle still has no mark. Offline still has the Offline mark.
- Rollback: restore the Bluetooth blank and the raw NOW/apps strings.

## Verification

Host: empty has no mark; loading is Waiting and busy; NOW/apps copy
unchanged; Bluetooth pending is `Сканирование…` and not tappable as a
device. Panther `e18f3d9d…` pid 8329: Bluetooth list with empty scan
log paints `Сканирование…` above Искать / Обновить / Назад. Do not
tap Сопрячь. Leave with Назад.
