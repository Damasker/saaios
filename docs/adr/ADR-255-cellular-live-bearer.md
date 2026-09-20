# ADR-255: cellular row comes from a live net bearer, not google_modemctl

## Статус

Принято, 2026-09-20. Shell Система. Not Visual v1 sign-off.
PIN stays null. Voice stays off (ADR-092).

## Нумерация

После ADR-254 следующий свободный номер — **255**. Не S33.

## Контекст

Wave E asked for a modem. Panther's live net class on 2026-09-20 is
`usb0` (USB NCM to the laptop), `wlan0`/`wlan1` (Wi-Fi), and tunnel
nics. There is no `rmnet*` / `wwan*` / `qmimux*` / `ccmni*`.
`google_modemctl.ko` is loaded for BCL / `modem_force_crash_exit_ext`
(ADR-024), not a cellular bearer. Inventing LTE bars would violate
the no-fact-source stop.

## Decision

1. **Bearer names only.** A net iface whose stem is `rmnet`, `wwan`,
   `qmimux`, or `ccmni` is a modem. `usb0` and `wlan*` are not.
2. **Honest empty.** No such iface → readout «Сотовая сеть / Нет
   модема», no dispatch, no dBm, no operator, no IMEI.
3. **Live names when present.** Join the iface names. Do not guess
   radio technology.
4. **Host no-op.** Missing `/sys/class/net` yields an empty list.

## Consequences

Система stops implying a phone radio that is not there. Cameras stay
a later E slice (`video4linux` on panther is `v4l-touch0`, not a
capture node). Rollback: revert the cellular row.

## Verification

Host: `cargo test -p saai-shell --offline -- cellular_`.
Panther listing used as the negative fixture. Leave Сейчас. Do not
tap Сопряжь.
