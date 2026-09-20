# ADR-217: VUI-09 — NOW content hits come from `compile_v2()`

## Статус

Принято, 2026-09-20. Production tabs already use `layout_v2(root.sui)`
(ADR-216). Live Сейчас footer and ObjectSummary hits now come from
`layout_v2(compile_v2(ui/now.sui))`, the same public NOW grammar.
A missing live object invents no `open_object`. Paint still draws
`Frame::Now`; footer rects are the compiled nodes. No new daemon.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-216 следующий свободный номер — **217**. Не S33.

## Контекст

`root.sui` names only tabs. Приложения / Новое намерение and the
NOW object still used `now_footer_action_rect` /
`now_object_summary_rect`. `now-public.sui` already matched those
hits on host. Closing procedural NOW means the shell compiles that
document instead of keeping a second formula.

## Decision

1. **Document.** `services/saai-shell/ui/now.sui` is the production
   NOW chrome (same grammar as `now-public.sui`).
2. **Hits.** `now_footer_action_at` and `now_object_tapped` read
   `layout_v2`. Object tap still requires `now_object_summary()`.
3. **Footer paint.** `now_footer_action_rect` is the compiled `apps`
   / `intent` node rect so draw and hit-test cannot drift.
4. **Reflash.** Chrome hit-test path changed. PIN stays null.

## Consequences

- NOW content destinations are compiled. Inbox/Spaces/Me/lists stay
  procedural until their own documents. Rollback: restore the
  formula helpers. Still not Visual v1 sign-off.

## Verification

Host: footer 540,1860 / 540,2080; object 540,335 only with a live
summary. Panther: flash; Сейчас; do not tap Приложения or the
object; leave Сейчас.
