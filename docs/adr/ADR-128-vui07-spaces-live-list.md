# ADR-128: VUI-07 `Пространства` — live list, SpaceRow

## Статус

Принято, 2026-09-18. Host slice: Spaces tab lists `entityd` spaces.
Space detail (people / members / focus work) is not this slice.
Pixel flash later.

## Нумерация

После ADR-127 следующий свободный номер — **128**. Не S33.

## Контекст

VUI-07 next surface after Inbox. Visual Language 9.3 is the Space
*page* (objects, people, related work). PIXEL-PATH still has a
**Пространства** tab of contexts. Today that tab is four hardcoded
`root.sui` cards (`home` / `work` / `personal` / `saaios`). Live
`Space` records, lifecycle, and one relation-or-lifecycle status
already exist (ADR-086) and paint through `content_card`. Disconnect
clears `self.spaces`, but the four markup cards remain, so the page
never looks empty and never looks like Inbox/Система offline.

HIA graph semantics to keep: tap selects; retap on the current space
cycles lifecycle; status shows object count plus at most one of
lifecycle label or first relation target; archived still only suffixes
the context header. Do not invent people, an app drawer, or a second
Space-detail frame.

## Decision

1. **`SpaceRow` wraps `DataRow`**. Live row: Navigation,
   `select_space:{id}`, selected flag for the current context. Empty
   (store up, no spaces): Static «Нет пространств». Offline (entityd
   down): Static «Нет связи». No people line. No fabricated count.
2. **List `self.spaces`**, not the four markup ids. Flatten to
   `ActionCardView` (`Выбрано` / `Открыть`, same status copy as
   today). Hit-test `stacked_row_rect`. Offline and empty are not
   tappable. Remove the four `page=spaces` actions from `root.sui`.
3. **Space detail waits.** Members, people, and focus work stay off
   this tab until a first real consumer besides the existing Object
   View / NOW object card.

## Consequences

- A fifth real space from the store appears without a `.sui` edit.
- Store-down matches Inbox: one «Нет связи» row, not four dead cards.
- Rollback: restore the four `root.sui` actions and `content_card`
  dispatch.

## Verification

Host: SpaceRow constructors; live list order and selected button;
lifecycle/relation still at most one extra; offline vs empty; markup
no longer hit-tests Spaces. Physical Pixel with the rest of VUI-07.
