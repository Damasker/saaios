# ADR-120: Intent Resolution & Action Bridge

## Статус

Принято, 2026-09-18. Host-verified: `cargo test -p intent-resolution`
`-p saai-taskd -p saai-object-actions`, clippy `-D warnings` on those
packages, plus `saai-shell` unit tests for context capture. Pixel
pronoun/clarification/degraded-AI acceptance is not claimed here.
Semantic Action execution still does not call ToolExecutor; that
bridge is the next slice.

## Нумерация

На `origin/feat/pixel7-native-saaios` @ `0797c5a` последний ADR — 116.
VUI-04 держит 117, SOM — 118, OAM — 119. Этот документ поэтому **120**.

## Контекст

ADR-033 доказал: free-form `saaios.intent` → `saaios-runtime.diagnose(text,
space_id)` → raw tool или `PendingConfirmation`. Модель при этом почти
не знает focused Object, semantic OAM actions и captured revision.

SOM даёт существительные, OAM — глаголы. Недостающий слой: что
пользователь имеет в виду **в текущем UI-контексте**.

Orb — UI-вход, не владелец интерпретации. Тот же механизм нужен Object
View, automation и workers.

## Решение

**Intent Resolution & Action Bridge (IRAB)** в `crates/intent-resolution`.
Не daemon, не второй PolicyEngine, не второй OAM registry.

Phase 1–2 этого ADR:

1. Shell в момент submit замораживает `ContextSnapshot` (focused
   `ObjectRef`, revision, effective space, source) в properties Intent.
   Живой UI focus после submit не перечитывается.
2. Детерминированный resolver: explicit `semantic_action_id` не вызывает
   модель; pronoun inspect использует captured Object; два кандидата →
   `Clarification`, не угадывание. Model-proposed Object/action
   проверяются против captured allowlist.
3. `WorkflowStatus::WaitingClarification` отдельно от
   `WaitingConfirmation`.
4. `saai-taskd` читает snapshot, логирует IRAB, не зовёт `diagnose`
   для resolved Action/Clarification/Unsupported. Legacy diagnose
   остаётся explicit fallback (`IRAB fallback: legacy_runtime_diagnose`).

`Diagnose` wire op не удаляется. `delete_entity` special-case не
удаляется.

## Что не делается

Нет `saai-irabd`, нет LLM на Object View explicit action, нет raw tool
выбора вместо semantic action_id, нет silent fallback после Policy Deny
(execution bridge ещё не вызывает Policy), нет chain-of-thought в store,
нет полного Orb semantic state machine.

## Последствия

Free-form без captured target по-прежнему идёт в ADR-033 diagnose.
IRAB добавляет контекст и детерминированную границу affordance.

## Ссылки

- ADR-030, ADR-032, ADR-033, ADR-087, ADR-088, ADR-118, ADR-119
- `crates/intent-resolution`
- `services/saai-shell/src/intent_context.rs`
