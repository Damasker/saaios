# ADR-295: shell shows Verifying, not worker Result as Done

## Статус

Принято, 2026-09-21. WORK-03 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell.

## Нумерация

После ADR-294 следующий свободный номер — **295**. Не S33.

## Контекст

ADR-259 persists `verifying` after a worker Result. Done needs a
Fresh Observation. The shell only treated `running` as in-progress
and lit Orb Complete from any successful `saaios.result`. A
Verifying Task plus a Result looked finished. The boards' Результат
is verified work, not worker ok.

## Decision

1. **In progress.** `running` and `verifying` are «Продолжается».
   Confirmation stays attention. Done stays complete.
2. **Copy.** Status text is «Проверяется», not the raw `verifying`
   token and not «Готово».
3. **Orb.** Verifying is `UniversalState::Running`. Complete still
   requires a Result with no open verifying/running Task.
4. **Host only.** Panther shell is unchanged until the next allowed
   shell experiment. Do not flash taskd.

## Consequences

- Intent plan progress can show «Проверяется» before «Готово».
- Rollback: drop `TASK_STATUS_VERIFYING`.

## Verification

Host: `cargo test -p saai-shell --offline verifying`. No panther flash.
Leave Сейчас.
