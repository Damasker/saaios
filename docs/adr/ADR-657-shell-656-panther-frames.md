# ADR-657: panther frames after host 615–655 flash

## Статус

Принято, 2026-10-14. Live `/data/saaios/system/saai-shell` stays
`eb3197c9…` (ADR-656). dest-no-lock stays absent. displayd pid
2694 stayed. shell pid 2396 stayed. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked. VUI-04/05/06 Acceptance stay unchecked.

## Нумерация

После ADR-656 следующий свободный номер — **657**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user asked to verify the ADR-656 flash on panther by frames,
not hunt leftovers. Device clock is 2026-10-08. Deep idle had
blanked the panel (`dpms` Off, `deep idle timeout, screen off`).
Do not start the shell from COM13. Do not tap Orb. Do not restore
dest-no-lock. Do not invent weather.

## Решение

1. Center tap `(540, 1200)` on the lock surface: displayd
   `touch down … locked=true`, shell `unlocked by touch`,
   displayd `session unlocked`. DSI `dpms` On. Status layer
   `1080x120` and main `1080x2400` both blit after unlock.
2. Tab hits on the main surface (`locked=false`):
   `(135, 2250)` Сейчас, `(405, 2250)` Пространства,
   `(675, 2250)` Поиск. Log shows `switched to Now|Spaces|Search`
   and matching commits. Status `1080x120` stays present.
3. Injected `(945, 2250)` Sistema / `Me` reached the main
   surface. `switched to Me` was not in the flushed log. Do not
   treat that as a leftover regression: shell stdout to
   `/run/saai-displayd.log` may be block-buffered. Finger
   evidence on the glass is still required.
4. `/run/saaios/shell-frame.last` did not refresh during this
   walk (device mtime stayed `04:44` while `date` reached
   `05:03`). dest-no-lock stayed absent. No `/dev/fb0` pixel
   dump. Font compare stays a device task.

## Откат

No binary change. Previous binary remains
`/data/saaios/system/saai-shell.prev-adr614`.

## Ссылки

- ADR-656, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
