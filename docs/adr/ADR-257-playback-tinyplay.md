# ADR-257: playback row names tinyplay + test-tone, does not play

## Статус

Принято, 2026-09-20. Shell Система. Not Visual v1 sign-off.
PIN stays null. Voice stays off (ADR-092). Do not spawn tinyplay.

## Нумерация

После ADR-256 следующий свободный номер — **257**. Не S33.

## Контекст

Wave E asked for sound besides PCM gain. Volume already writes tinymix
(ADR-253). The proven speaker path is
`os/targets/panther/scripts/audio-test.sh`: `/saaios/tinyplay
/saaios/test-tone.wav -D 0 -d 1`. Both binaries exist on panther.
Auto-playing a tone from Система would be a tap we are not allowed to
exercise this week, and would not be voice.

## Decision

1. **Presence only.** Ready iff `/saaios/tinyplay` and
   `/saaios/test-tone.wav` exist.
2. **Readout.** «Воспроизведение» is `tinyplay · test-tone` or
   «Нет tinyplay». No dispatch.
3. **Not voice.** Capture PCM nodes stay unused.

## Consequences

Система names the speaker path without claiming a tone played.
BT adapter presence stays a following E slice. Rollback: revert the
playback row.

## Verification

Host: `cargo test -p saai-shell --offline -- playback_`.
Leave Сейчас. Do not tap Громкость. Do not play the tone from UI.
