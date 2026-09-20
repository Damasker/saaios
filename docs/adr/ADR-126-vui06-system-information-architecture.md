# ADR-126: VUI-06 `Система` — inventory, domain grouping, SettingRow

## Статус

Принято, 2026-09-18. Destination grouping, scroll-cache, and the
`Я`→`Система` label are on panther (`1d191d7a…`). Internal id stays
`me` / `select_root:me`. MEM-08 omitted.

## Нумерация

После ADR-125 следующий свободный номер — **126**. Не S33.

## Контекст

VUI-06 is the engineering view of the physical device. PIXEL-PATH puts
manual Memory review (MEM-08) on this sprint, not a separate phone
track. `Я` today is a flat 19-row settings dump plus installed-app
grant lines. Concept boards and Visual Language 9.5 group by device
domain, not by the sprint that added the card.

`Я`→`Система` rename waited until this destination was truthful.
MEM-08 review chrome is host (ADR-268); panther paint waits.

## Inventory (current `Я` and adjacent frames)

Real sources already on the shell. No new IPC.

| Domain | Rows / frames | Source | Action |
|---|---|---|---|
| Device | «Это устройство» spaces/objects | `spaces`, `entity_counts` | readout |
| Device | build / model / kernel / uptime | `SAAIOS_BUILD_ID`, `/proc` | silent 7-tap → `DevSurface` (HIA-20) |
| Device | «Хранилище» | `/data` usage | readout |
| Device | «Обновления» | boot slot + attempts | readout; no check-for-updates |
| Device | «Часовой пояс» | `ShellSettings` UTC offset cycle | cycle |
| Display | brightness, lock idle, deep idle, text scale, contrast | settings + sysfs/post-process | cycle |
| Sound | volume | settings; hardware effect unverified (S18) | cycle |
| Network | Wi-Fi | `wpa_supplicant` | open `WifiList` / password |
| Network | Bluetooth | `bt-scan` paired count, not live link | open `BluetoothList` |
| Network | SSH remote access | settings + dropbear | toggle |
| Network | trusted clients | `authorized_keys` | open `TrustedClients` |
| Privacy | PIN | settings; default any-tap unlock | open `PinSetup` |
| Space | space color | selected space entity | cycle (HIA-03) |
| Interface | Orb, reduced motion | settings | toggle |
| Apps | installed apps + grants | `appd` | readout; no revoke protocol |
| Diagnostics | `DevSurface` | live shell state | hidden behind build taps |

Battery already lives on the status layer (`SystemStatus`). No second
gauge on this page.

### Honest absences (omit, do not paint)

- **Memory list/correct/erase (MEM-08).** `memory-store` is Platform
  Track (ADR-038/125). Shell must not probe `saaios-runtime` (ADR-030)
  and must not grow a second JSONL client. No row, not «0 записей» and
  not «Недоступно» from a fake probe. Review UI waits on a proven
  shell-legal read (not this slice).
- **Android VM / notebook / car.** APP-COMPAT / later surfaces.
- **Power battery card.** Duplicate of status; Visual Language forbids
  gauges that do not inform a decision here.
- **Service health / recovery / evidence / dangerous-action
  composites.** No first consumer on this page yet. Recovery stays
  existing Wi-Fi/Bluetooth/SSH/PIN frames. Dangerous confirmation
  already exists as `DecisionOverlay` (VUI-05). Do not stub empty
  health clusters.

## Decision

1. **`SettingRow` / `CapabilityRow`** in `saai-ui-core` wrap `DataRow`.
   Setting: readout / cycle / open / silent (HIA-20). Capability: app
   name, state, grants; no revoke button. No new draw path.
2. **`me_system_sections`** groups the same 19 controls into device
   domains: Устройство, Экран, Звук, Связь, Приватность, Пространство,
   Интерфейс, and Приложения when apps exist or `appd` is down
   («Нет связи»). Empty-and-connected omits the section. Missing
   Wi-Fi/Bluetooth hardware is «Нет адаптера»; store down makes space
   color a readout. Empty sections omitted otherwise. Existing
   `SystemSection` is the grouping composite.
3. **Hit-test by dispatch key** carried on the row, not
   `me_fixed_card_action(index)`. Section headers are inert.
4. **Flatten to `ActionCardView`** for the current `draw_root` list so
   coalesced drag, `scrolled_row_rect`, and status/nav layering stay
   the accepted `Я` path. Migrating paint to `draw_data_row` is VUI-07.
5. **Rename the tab to `Система`.** Visible label only. `ROOT_TABS`
   id/`select_root:me`/`RootPage::Me` stay. «Пространства» is already
   the longest label; `Система` fits the same strip.

## Consequences

- User sees the same controls, grouped. No invented telemetry.
- Index-stable `ME_FIXED_CARD_COUNT` goes away; scroll length is the
  flattened row list (headers + settings + apps).
- MEM-08 remains backlog until a shell-legal memory read exists.
- Rollback: revert this commit; previous flat `Я` list returns.

## Verification

Host: `saai-ui-core` SettingRow/CapabilityRow tests; `saai-shell`
domain-order, omitted Memory/Android rows, build-info silent tap,
scroll still misses the nav strip. Physical Pixel review later with
the rest of VUI-06.
