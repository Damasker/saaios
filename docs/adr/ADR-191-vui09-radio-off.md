# ADR-191: VUI-09 — live radio-off on panther without a Me tap

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`. `wlan0` went
`up` → `down` → `up` through the existing sysfs/`ip` path
`wifi_is_up()` already reads. Status bar named `Нет сети` while the
radio was down; Сейчас still showed the live ObjectSummary, not
«Нет связи с пространствами», because entityd was still connected. No Me tap, so no 7-tap, no lock-timeout row, no
PIN, no Orb, no SSID. `compile()` stays `sui 1`. No new daemon.
Leave Сейчас.

## Нумерация

После ADR-190 следующий свободный номер — **191**. Не S33.

## Контекст

The ledger left «network / AI offline» as host-only named copy. Live
radio-off is `wlan0` operstate, not store-down. Opening Система to
reach Wi-Fi sits under the 7-tap build row. Bringing the interface
down from serial is the same `wifi_is_up()` path the status bar
polls every second.

## Decision

1. **Do not tap Система.** `ip link set wlan0 down`, wait for the
   status-bar poll, shot Сейчас, restore `up` + `wpa_cli reassociate`.
2. **Do not reflash.** Binary stays ADR-188.
3. **Do not lock, reboot, or kill displayd.** USB-NCM stays up.
   Radio-off is not entityd-offline.

## Consequences

- Live radio-off is proven on HEAD chrome. Rollback: leave `wlan0`
  up. Lock cycle, display restart, and cold boot stay open.

## Verification

Host: `wifi_is_up` still reads `/sys/class/net/wlan0/operstate`;
ledger cites ADR-191. Panther: Сейчас shows `Нет сети` then `Wi-Fi`
again; leave Сейчас.
