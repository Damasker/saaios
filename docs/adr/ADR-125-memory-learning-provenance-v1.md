# ADR-125: Memory, Learning & Provenance Model v1 — evolve memory-store, do not migrate it

## Статус

Принято, 2026-09-18. MEM-00 (этот ADR + roadmap) — docs.
MEM-01 — scope-aware `(space_id, key)` compaction + labelled context
projection. Остальные фазы — [MEM-ROADMAP.md](../os/sprints/MEM-ROADMAP.md).
Pixel 7 slices — MEM-10, после Visual queue.

## Нумерация

После ADR-124 следующий свободный номер — **125**. Не S33.

## Контекст

`crates/memory-store` уже существует: append-only JSONL, `MemoryFact`,
tools `memory.remember` / `memory.recall` / `memory.forget`, reboot
persistence. ADR-038 правильно оставил Memory на Platform Track и
протянул `space_id` / `origin_correlation_id`. ADR-039 подтвердил
изоляцию *разных* ключей между Home и Work.

Два дефекта остаются внутри этого store, не снаружи:

1. `latest_by_key()` компактизировал только по `key`. Одинаковый
   semantic key в Work и Home перезаписывал друг друга до space filter.
2. `format_context(12)` вставлял записи в system prompt как
   `Known facts (memory)`, а модель могла сама вызвать
   `memory.remember` и затем прочитать свой вывод как факт.

SOM (ADR-118), World Model (ADR-122), workflow history и audit отвечают
на другие вопросы. Memory — не shadow database ни одного из них.

## Решение

**MLP v1** эволюционирует существующий JSONL store. Не `saai-memoryd`.
Не `saaios.memory` entity. ADR-038 **не отменяется**: Platform/native
граница, `space_id` от `saai-taskd`, correlation provenance остаются.

```text
Remembered ≠ true now
Inferred ≠ explicitly known
Nothing durable without explainable origin
Scope learned in one context cannot silently escape
Memory never creates permission
The user can always overrule inference
Forget must have honest storage semantics
The model may propose memory; it cannot declare facts
```

Target record (MEM-03, не этот срез): `MemoryRecord` с `kind`
(`ExplicitFact` / `ExplicitPreference` / `LearnedHypothesis`),
`scope`, structured `provenance`, `state`, `sensitivity`, `validity`.
Legacy `MemoryFact` читается как view. Нет `LearnedFact`. Нет vector DB.
Нет personality engine.

Logical identity v1: `(scope, key)`. Default write — current Space, не
Global. `None` больше не должен означать All для ordinary callers
(MEM-02). All-scopes — explicit admin/debug.

Context: `MemoryContextProjection` с kind/scope labels. Values are
data, not instructions. Restricted records не уходят на remote model
без policy. Secrets — не Memory.

Learning — после typed memory, correction, erase и UAM. Hypothesis
требует independent evidence; self-reference и model repetition не
считаются. Promotion только через явное подтверждение пользователя.
Learning не пишет grants, SOM relations, ContextFrame signals, Actions.

`saaios-runtime` не становится клиентом `saai-entityd` ради памяти.

## Что сделано в MEM-01

- Compact key = `(space_id, key)`.
- Space > Global для одинакового key при scoped recall.
- `forget` tombstone'ит только совпадающую identity.
- `format_context` больше не говорит `Known facts`; records wrapped as
  `<memory_records>` data.
- System prompt запрещает модели объявлять собственные выводы фактами.
  Tool `memory.remember` ещё зарегистрирован; authoritative write
  снимается в MEM-05.

## Roadmap

`docs/os/sprints/MEM-ROADMAP.md`

## Ссылки

- ADR-038, ADR-039 — space-scoped Platform memory (extended, not replaced)
- ADR-118 SOM, ADR-119 OAM, ADR-120 IRAB
- ADR-122 World Model, ADR-123 Attention, ADR-124 UAM
- `docs/platform-0.3-memory.md`
- `crates/memory-store`
