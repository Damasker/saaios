# ADR-253: panther volume writes tinymix Digital PCM Volume

## Статус

Принято, 2026-09-20. Shell volume. Not Visual v1 sign-off.
PIN stays null. Voice stays off (ADR-092).

## Нумерация

После ADR-252 следующий свободный номер — **253**. Не S33.

## Контекст

Wave E starts phone hardware. Система already had a volume cycle, but
`apply_volume` called `amixer sset Master` — a control that was never
shown to exist on panther. The proven path is
`os/targets/panther/scripts/audio-volume.sh`: `/saaios/tinymix -D 0
'Digital PCM Volume'` and `'R Digital PCM Volume'`, range 400–817,
echoed to `/run/audio-volume`.

## Decision

1. **Same controls as the script.** Percent 0–100 maps linearly onto
   400–817. Both L/R PCM controls are set. `/run/audio-volume` is
   written only when tinymix exists.
2. **Host no-op.** Without `/saaios/tinymix`, `apply_volume` returns.
   Tests cover the mapping and that missing tinymix does not panic.
3. **Not voice.** This is speaker PCM gain, not capture, not Слушает.

## Consequences

Boot and `cycle_volume` can actually change output on panther.
Power/modem/cameras stay later E slices. Rollback: revert `apply_volume`.

## Verification

Host: `cargo test -p saai-shell --offline --
pcm_volume_maps_percent_onto_tinymix_digital_pcm_range`.
Panther: flash shell; after restart `/run/audio-volume` is in 400–817
if tinymix is present. Leave Сейчас. Do not tap Громкость unless
needed to confirm a write.
