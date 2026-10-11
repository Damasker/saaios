# ADR-477: VUI-09 — Object OAM quick actions and composer mic disabled

## Статус

Принято, 2026-10-11. The board names 0/1/2 Object quick actions
from OAM and a disabled composer microphone. The only registered
spec is `display.inspect`. The shell has no `system.identity`
provider, so that row is Status with the live reason. Confirm /
Hide stay the trailing workflow Buttons. Intent compose Field
help is «Голос недоступен», not a listen toggle. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-476 следующий свободный номер — **477**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-471–473 locked related peers and Intent steps. Quick actions
were still leftover trailing labels. The board forbids invented
«Open Settings» / «Check Identity». Voice is hardware-blocked
(ADR-092). A clickable mic would lie.

## Decision

1. **Host-lock** `object.action.{i}` DataRows after plan steps,
   at most two, from `builtin_object_actions()` + policy preflight.
2. **Live.** Available OAM titles would be Buttons. Unavailable /
   Deny / Ambiguous stay Status with the reason. Zero matching
   specs omit the block. Do not invent labels.
3. **Composer.** Intent Field help always names that voice is
   unavailable. Offline keeps «Нет связи» and adds the same
   voice clause. The Field is not a listen control.
4. **Not this slice.** Do not change `intent-public.sui`. Do not
   enable listen. Do not convert leftover Confirm/Hide formulas
   into OAM. Do not add a provider tool.

## Consequences

- Object View shows the real OAM set, which today is one disabled
  display inspect. Rollback: omit `object.action.{i}` and restore
  empty Field help when connected. Still not Visual v1 sign-off.

## Verification

Host: display entity paints «Состояние экрана: нет инструмента»
with no hit; task entity emits no OAM rows; compose help names
voice. Panther: do not flash; leave Сейчас.
