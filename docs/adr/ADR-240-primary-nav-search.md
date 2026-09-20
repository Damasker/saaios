# ADR-240: Primary nav is Сейчас · Пространства · Поиск · Система

## Статус

Принято, 2026-09-20. Primary chrome still had Входящие as tab 2.
The boards' footer is Сейчас · Пространства · Поиск · Система.
This slice demotes Inbox to an Orb destination and adds Поиск over
live objects in the selected space. PIN stays null. Not Visual v1
sign-off.

## Нумерация

После ADR-239 следующий свободный номер — **240**. Не S33.

## Контекст

Attention already lives on Сейчас and Orb. Inbox as a primary tab
is a fifth OS. Поиск is a domain: find objects and intents that
already exist in the selected space. System records
(space-color/lifecycle/relation/signal), notifications, and
schedules are not search hits. No people row without a person
entity. Goal wave B, one screen. Do not invent voice, weather, or
an app store.

## Decision

1. **Tabs.** `root.sui` / `now.sui` / live `V2_ROOT_TABS` are
   `now`, `spaces`, `search`, `me`. Labels
   Сейчас · Пространства · Поиск · Система.
2. **Inbox.** `RootPage::Inbox` stays for Orb `OpenInbox`. It is
   not a tab. v1 rollback keeps inbox.
3. **Search.** DataRows from `selected_entities` that are not
   system/notification/schedule records. Kind caption is
   Намерение / Задача / Действие / Результат / Объект. Empty is
   «Нет объектов». Store-down is «Нет связи». Tap opens Object
   View. No query Field in this slice — listing the space is the
   domain.
4. **Public.** `search` is a v2 surface. `compile_v2_public` stays
   Experimental. Examples follow production tabs.

## Consequences

NOW honesty, Space detail, and Orb workflow states stay later B
slices. Rollback: restore inbox as tab 2. Visual v1 still unsigned.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+265 passed, including
`root_tabs_come_from_sui_markup`,
`search_rows_list_space_objects_not_inbox_or_system_records`,
`production_root_v2_tab_hits_are_now_spaces_search_me`.
Panther: shell `787f5982…` pid 1311. Leave Сейчас; do not tap Поиск
rows, Inbox, Spaces, Разрешить. Marker on.
