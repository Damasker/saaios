# ADR-256: camera row comes from a capture node, not v4l-touch

## Статус

Принято, 2026-09-20. Shell Система. Not Visual v1 sign-off.
PIN stays null. Voice stays off (ADR-092). Do not light a preview.

## Нумерация

После ADR-255 следующий свободный номер — **256**. Не S33.

## Контекст

Wave E asked for cameras. Panther's `video4linux` class on 2026-09-20
contains `v4l-touch0` (the SPI touch panel), not `/dev/video*`.
Painting a viewfinder or an Android camera app would invent a
capability that is not there.

## Decision

1. **Capture names only.** A `video4linux` entry whose name starts
   with `video` and does not contain `touch` is a camera node.
2. **Honest empty.** No such node → readout «Камера / Нет узла
   захвата», no dispatch, no preview, no Слушает.
3. **Live names when present.** Join the node names. Do not open the
   device in this slice.

## Consequences

Система does not pretend Pixel cameras work. Sound/BT stay later E.
Rollback: revert the camera row.

## Verification

Host: `cargo test -p saai-shell --offline -- capture_ camera_row`.
Panther `v4l-touch0` is the negative fixture. Leave Сейчас.
