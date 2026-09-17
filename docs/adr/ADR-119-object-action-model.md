# ADR-119: Object Action Model — semantic verbs over SOM objects

## Статус

Принято, 2026-09-17. Host-verified: `cargo test -p saai-object-actions`,
clippy `-D warnings` on that package. Pixel Object View integration and
taskd execution bridge are not claimed here.

## Нумерация

На `origin/feat/pixel7-native-saaios` @ `0797c5a` последний ADR — 116.
Незакоммиченный VUI-04 держит 117, SOM в этом worktree — 118. Этот
документ поэтому **119**.

## Контекст

SOM (ADR-118) отвечает, что существует и как связано. Уже существуют
отдельные слои, которые нельзя дублировать:

- ADR-020 app **capabilities** — permissions приложений, не действия
  над объектом;
- `ToolSpec` / `ToolRegistry` — executable backend;
- `PolicyEngine` — Allow / AskUser / Deny, session grants, hard deny;
- Intent / Task / Action / Result — persistent workflow (ADR-030).

Недостающий кусок: какие **semantic** действия применимы к конкретному
SOM-объекту и каким существующим tool они исполняются.

Слово `capability` занято. Этот слой называется Object Action Model.

## Решение

Новый crate `saai-object-actions`, не daemon и не второй registry tools:

```text
Object  →  ObjectActionResolver  →  ResolvedAction
                ↓
            ToolSpec  →  PolicyEngine  →  existing Action path
```

`ObjectActionSpec` описывает `action_id`, selector по `entity_type`,
`tool_name`, structured `ArgumentBinding`, `provider_id`. Risk,
`requires_confirmation`, schemas остаются на `ToolSpec`. `action_id` и
`provider_id` — dotted tokens без underscore; `tool_name` принимает
уже существующие имена вроде `process.kill_request`.

Resolver детерминирован: один available provider → `Resolved`; несколько
→ `Ambiguous`, не «первый зарегистрированный». Missing property / missing
tool → `Unavailable` с причиной, не panic.

`preflight` читает **живой** `PolicyEngine`. `execute_if_allowed`
перепроверяет policy и не вызывает executor на Deny / AskUser / stale
`Entity.revision`. Production execution по-прежнему должен идти через
taskd; этот gate — общий запрет обхода policy.

Первый builtin: `display.inspect` → `system.identity` для
`saaios.display`.

## Что не делается

Нет второго PolicyEngine, нет нового ToolRegistry, нет oam daemon, нет
eval/шаблонов, нет LLM в discovery, нет перевода всего Object View, нет
авто-выбора provider по порядку регистрации, нет обхода AskUser.

## Последствия

Planner/Orb/Object View получают ограниченный набор реальных глаголов
для объекта, а не сырой список всех tools. App capabilities и workflow
entities не переименовываются.

## Ссылки

- ADR-020, ADR-030, ADR-032, ADR-033, ADR-088, ADR-118
- `crates/saai-object-actions`
- `crates/tool-registry`, `crates/policy-engine`
