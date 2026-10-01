# ADR-427: ATTN-05 — deterministic context relevance and attention order

## Статус

Принято, 2026-10-01. Host crate only (`saai-attention`); no shell wiring,
no flash. Не S33. Номер 427: 424–425 заняты веткой
`feat/lockscreen-policy-widgets`, 426 — ADR Система capacity.

## Контекст

ADR-123 reserved «ContextFrame relevance» as a later slice and promised the
order `priority → actionability → relevance → time → key`. The projection
filled `relevance = Global` for everything and kept entry order, so the type
`AttentionRelevance` existed but nothing derived it. The accepted boards
show attention on `Сейчас` for the current context (Work), not a flat list.

## Decision

1. `AttentionContext { space_id, object }` is the only input: where the
   user is. Absent fields mean unknown and keep everything `Global`. No AI,
   no history, no profile, no scoring.
2. `project_in_context(entities, health, &ctx)` derives relevance per
   entity-backed item: `CurrentObject` if the item *is* the open object,
   `CurrentContext` if it lives in the selected space, else `Global`.
   `RelatedContext` needs relationships and is deliberately not derived.
   Health stays `Global`.
3. Order is a stable sort by priority, actionability, relevance (all
   descending). Ties keep input order. Time and key are not used yet: input
   order is already deterministic and ADR-123 inbox parity relies on it.
4. Relevance never outranks a pending decision: a `RequiresDecision` task in
   another space stays above a current-space notice.
5. Membership is unchanged by context. Context only reorders and labels.
6. `project_with_health` is `project_in_context` with the default context, so
   existing callers compile unchanged.

## Consequences

Visible order change for existing callers: an Unhealthy Health item now sorts
between pending decisions and notifications (High priority beats Normal),
where it used to come last. Tasks still precede notifications.

The shell keeps calling `project_with_health`. It only loads the selected
space's entities, so every item would be `CurrentContext` and the order would
not change; wiring starts to matter when a cross-space attention feed exists
(ATTN-07 / WORLD-08). Rollback: remove `project_in_context`'s sort and
`relevance_of`; `AttentionRelevance` stays `Global`.

## Verification

Host: `cargo test -p saai-attention` (22 passed) and
`cargo test -p saai-shell -- --test-threads=1` (309 passed), including
`current_space_outranks_other_spaces_at_equal_priority`,
`open_object_outranks_its_space`,
`relevance_never_outranks_a_pending_decision`,
`unhealthy_sits_between_decisions_and_notices`,
`context_does_not_change_membership_or_inbox_parity`.
