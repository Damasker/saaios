# ADR-294: native Wayland clipboard is deny-by-default

## Статус

Принято, 2026-09-21. APP-COMPAT / AUTH follow-up to ADR-023.
PIN stays null. Do not flash displayd.

## Нумерация

После ADR-293 следующий свободный номер — **294**. Не S33.

## Контекст

ADR-021 advertises `wl_data_device_manager` so GTK4 will open a
display. Smithay 0.7 then brokers real client-to-client copy/paste.
`SelectionHandler` cannot refuse SetSelection or Receive (ADR-023).
Portal clipboard is already gated (AUTH-08). Panther accidentally
blocks SetSelection because it has no `wl_keyboard`. x86 seats have
a keyboard (ADR-251), so the accidental block does not apply to the
laptop node.

## Decision

1. **Own the global.** `SaaiDataDeviceManager` advertises
   `wl_data_device_manager` v3. GTK still binds. Smithay's
   `DataDeviceState` is not used.
2. **Deny native selection.** `SetSelection` and `StartDrag` send
   `cancelled` on the source. No `wl_data_offer` is created, so
   Receive never runs.
3. **Portal stays the grant path.** `clipboard.read` /
   `clipboard.write` still go through `decide_capability`. This slice
   does not put GrantStore inside displayd.
4. **Host only.** Panther binary unchanged until the next allowed
   displayd experiment.

## Consequences

- Native GTK/Qt clipboard is empty by design, including on x86.
- Rollback: restore smithay `delegate_data_device!`.

## Verification

Host: `cargo test -p saai-displayd --offline data_device`.
No panther flash.
