# ADR-487: VUI-09 — Object related offline is not empty

## Статус

Принято, 2026-10-11. Handoff Object offline says an unknown
related source is not zero peers. Missing store and no cached
SOM peers is Status «Связи недоступны», not an omitted section
and not «Нет данных». Cached peers stay visible. Intent store-down
stays ADR-483. Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-486 следующий свободный номер — **487**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-472 omits related Buttons when peer count is 0. That is
correct only when the store confirmed the empty set. A down
store with an empty cache is unknown.

## Decision

1. **Host-lock** the related absence. Connected and 0 peers:
   omit the section. Store down and 0 cached peers: Status
   «Связи недоступны» as `object.detail.0`, not
   `object.related.0`.
2. **Cached.** Store down with in-memory peers still paints
   those Buttons. Do not invent a peer.
3. **Intent.** «План недоступен · Нет связи» stays the Intent
   store-down caption. Do not stack a second related-offline
   line on that view.
4. **Not this slice.** Do not change `object-public.sui`. Do not
   scroll Space / NOW. Do not convert leftover keyboard /
   gallery / lock formulas.

## Consequences

- Unknown related is readable. Rollback: omit on 0. Still not
  Visual v1 sign-off.

## Verification

Host: connected empty stays no related hit; store-down empty
is «Связи недоступны» and not `Related(0)`; one live peer still
hits. Panther: do not flash; leave Сейчас.
