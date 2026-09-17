# ADR-124: Unified Authority Model — one language, existing boundaries

## Статус

Принято, 2026-09-18. AUTH-00 (этот ADR + roadmap) — docs.
AUTH-01 — host types only. PolicyEngine, portal, SSH, sandbox не меняются.
Pixel confirmation/session slices — AUTH-10, после Visual queue.

## Нумерация

После ADR-123 следующий свободный номер — **124**. Не S33.

## Контекст

Enforcement уже есть: ADR-020 grants/sandbox/portal, PolicyEngine
Allow/AskUser/Deny, SO_PEERCRED, SSH fingerprint pairing. Они говорят на
разных языках. Session grant сегодня — `HashSet<tool_name>` без Principal,
target, TTL. `decide_named()` создаёт `PolicyEngine::new()` и не видит live
grants. Это gap, чинится AUTH-04, не этим ADR.

## Решение

**UAM** — единая семантика authority, не новый daemon и не `authority.db`.

```text
Identity ≠ Trust ≠ Capability ≠ Grant ≠ Authority ≠ Confirmation
```

Principal задаёт trusted boundary, не JSON `"caller":"user"`.
Semantic action (OAM) — primary operation; ToolSpec risk остаётся trusted
metadata. Confirmation по умолчанию OneShot, bound to request/target/args.
Session grants не переживают reboot. Hard deny побеждает любой grant.
Planner/Attention/World Model/Learning не выдают authority.
SSH остаётся admin channel; UAM не притворяется, что policy-mediate каждую
root shell command. `authorized_keys` остаётся source of truth.

Существующие stores остаются: appd GrantStore, PolicyEngine session state,
workflow confirmation, SSH keys.

## Roadmap

`docs/os/sprints/AUTH-ROADMAP.md`

## Ссылки

- ADR-020, ADR-074, ADR-079, ADR-084, ADR-119, ADR-121
- `crates/policy-engine`
