# ADR-180: VUI-09 — `.sui` v2 vocabulary from proven components

## Статус

Принято, 2026-09-20. `.sui` v2 may name only the component and
surface vocabulary already proven in `saai-ui-core` and on panther.
This slice does not parse `sui 2`, does not change `root.sui`, and
does not move layout or hit testing. Space detail and MEM-08 stay
out of the vocabulary. No new daemon. Do not tap Inbox rows. Leave
Сейчас.

## Нумерация

После ADR-179 следующий свободный номер — **180**. Не S33.

## Контекст

ADR-017 described a declarative tree (`stack` / `row` / `list` /
style tokens) before the Visual Language existed. Live v1 is
narrower: `saai-ui-compiler` accepts only `sui 1` root chrome
(content actions + four tabs). Screens after VUI-03 are procedural
compositions of `saai-ui-core` contracts. VUI-09 must not invent a
second widget set or compile a format whose types are not already
on the device.

## Decision

1. **v1 stays the only compiled document.** `compile()` still
   rejects `sui 2`. `services/saai-shell/ui/root.sui` is unchanged.
2. **v2 names are the public `saai-ui-core` contracts** (primitives
   and composites) plus the live shell surfaces. Layout remains the
   existing `Node` / `Rect` / `SafeInsets` IR, not a new markup
   layout engine in this slice.
3. **Deferred and privileged are explicit.** Space detail, Memory
   review, chat, and widgets are not names. Orb, status layer, lock,
   diagnostic gallery, `DecisionOverlay`, `CapabilityRow`, and
   `TrustedClientRow` are named but not a third-party subset.
4. **No paint, protocol, or daemon change.** A later ADR adds the
   versioned grammar. Cleanup of leftover v1 NOW cards is not this
   slice.

## Consequences

- A future `sui 2` file that names an unknown component is a compile
  error, not a new Rust widget. Rollback: drop the vocabulary
  module; keep v1.

## Verification

Host: `sui 1` `root.sui` still compiles; `sui 2` is rejected;
vocabulary tests name every primitive/composite and omit deferred
names. Panther: Сейчас still shows the four v1 tabs, leave Сейчас.
No flash — the running binary is unchanged.
