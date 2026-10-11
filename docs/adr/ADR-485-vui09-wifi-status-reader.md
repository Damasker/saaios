# ADR-485: VUI-09 — Wi-Fi adapter, offline, and unread status

## Статус

Принято, 2026-10-11. Handoff Система asks to distinguish offline
Wi-Fi from a missing adapter and from an unavailable status
reader. Missing `wlan0` stays «Нет адаптера». Unreadable
`/sys/class/net` or silent `wpa_cli status` is «Нет статуса»,
not «Не подключено». Associated / not-associated stay live
when `wpa_state` is present. Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-484 следующий свободный номер — **485**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-129 opened the list when `wlan0` exists. Empty `wpa_cli`
output was treated as «Не подключено», which claims a known
idle radio when the status reader is down.

## Decision

1. **Host-lock** the Wi-Fi SettingRow on three sourced states.
2. **Live.** `/sys/class/net` readable and no `wlan0`: Status
   «Нет адаптера». `wlan0` present and `wpa_state=` present:
   open list with «Подключено: …» or «Не подключено».
3. **Unread.** No net class, or `wlan0` without `wpa_state=`:
   Status «Нет статуса». Do not open the list. Do not say
   «Не подключено».
4. **Not this slice.** Do not change `wifi-public.sui`. Do not
   invent a second probe. Do not convert leftover keyboard /
   gallery / lock formulas.

## Consequences

- Система names the three Wi-Fi absences honestly. Rollback:
  adapter-only gate. Still not Visual v1 sign-off.

## Verification

Host: missing adapter, unread status, and live «Не подключено»
stay distinct; timezone intern unchanged. Panther: do not
flash; leave Сейчас.
