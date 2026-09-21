# ADR-338: Falkon URL second click re-enables v2; still no shm paint

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-337 следующий свободный номер — **338**. Не S33.

## Контекст

ADR-337: an 80 ms settle after the first URL `enable` hits `disable`.
A burst of two `inject-click 640 20` lines only logged one click.
Sequential: first click, wait `disable`, second click on the same
URL band (not a Y sweep):

1. Second `injected click 640 20` is compositor-true.
2. v2 **re-enables**. Immediate OSK still forwards `commit_string` ×4
   and `delete_surrounding`.
3. Main toplevel shm hash at that enable equals the hash 700 ms after
   OSK (same class as ADR-335).

A second focus on LocationBar is not `QLineEdit::setFocus` that
inserts (ADR-336).

## Decision

1. **Do not claim the LocationBar shows `hi!`.** Second enable is
   protocol-without-paint.
2. **Do not sweep Y.** URL band stays `640 20`.
3. **Do not flash panther.**

## Consequences

- APP-04 Falkon chrome is still enable + OSK forward, not a painted
  URL. Probe QLineEdit remains the only typed Qt 6 field.
- Rollback: restore single-click OSK (ADR-334).

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
