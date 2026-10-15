# ADR-692: defer local Sistema probes onto the Wi-Fi turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `8f60e55c…` (ADR-691). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-691 следующий свободный номер — **692**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-691 settled `S` after `surface=me`. The camp turn still
walked `/sys/class/net` with per-iface IPv4/counters, listed
`video4linux`, parsed the Bluetooth SAVED log, and read
authorized_keys. dest-no-lock stays off. Do not start the
shell from COM13. Do not type PSK. Camera names stay
`video*`.

## Решение

1. `me_facts()` keeps Bluetooth count 0, empty camera and
   bearer lists, and zero trusted clients. Camp log and
   `/proc` still run on the full-facts turn.
2. `apply_me_net_probes` also runs `apply_me_local_probes` on
   the existing Wi-Fi/`df` turn. No extra event-loop flag.
3. Host test: stripping local probes keeps `cellular_cp`;
   applying values restores `video0` / `rmnet0`. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Revert `apply_me_local_probes`. Live `8f60e55c…` still walks
net/v4l/Bluetooth/keys during the camp turn.

## Ссылки

- ADR-686, ADR-691, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
