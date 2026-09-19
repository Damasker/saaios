# ADR-178: VUI-08 — keep dma-buf and wl_shm on the main surface

## Статус

Принято, 2026-09-20. Each main-surface commit records `FrameBackend`
(`dmabuf` or `shm`). VUI-08 does not change paint, import, or
displayd composition. Lock and status keep their existing fallbacks.
No new daemon. Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-177 следующий свободный номер — **178**. Не S33.

## Контекст

ADR-024 already paints the main surface into a dumb-buffer dma-buf
when `zwp_linux_dmabuf_v1` is present, and falls back to `wl_shm`
when a slot is busy or the path fails. VUI-08 motion/pacing must not
replace that split. Acceptance wants the two staging paths preserved
unless a separately measured change is accepted. This slice only
names which path produced the commit.

## Decision

1. **`FrameBackend`** is `Dmabuf` or `Shm`. `finish_frame` takes it
   from the branch that actually attached a buffer. A dropped SHM
   canvas (`pool.canvas` `None`) still only increments `dropped`.
2. **Last line and trace** append `backend=`. Host tests inject both
   values. The live shell does not abort or switch paths.
3. **Panther** reads `backend=dmabuf` on Сейчас (GPU import still
   live). `backend=shm` would still be a valid fallback, not a
   VUI-08 rewrite.

## Consequences

- A silent dma-buf disable shows up as `backend=shm` on the next
  commit. Rollback: drop the field.

## Verification

Host: both backends appear in `trace` / `line`. Panther: tap Сейчас,
`cat /run/saaios/shell-frame.last` has `backend=dmabuf`, leave Сейчас.
