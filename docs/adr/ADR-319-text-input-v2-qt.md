# ADR-319: displayd advertises `zwp_text_input_manager_v2` for Qt

## Статус

Принято, 2026-09-21. APP-04 compositor half on host. Not flashed.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-318 следующий свободный номер — **319**. Не S33.

## Контекст

ADR-317 / ADR-318: panther IME global is bound; `saai-shell-osk` never
maps. Falkon `<input>` and PCManFM-Qt Filter never
`zwp_text_input_v3::enable`. Host OSK tests used a dedicated v3 client,
not Qt.

`strings` on the packed toolkits:

- Qt 5.15 `libQt5WaylandClient` — only `zwp_text_input_manager_v2`.
- Qt 6.6.3 `libQt6WaylandClient` — v1, v2, v4 managers;
  `grep -c text_input_v3` = 0.
- displayd advertised only v3 (ADR-267). Intersection is **v2**.

v2 XML is KDE `wayland-protocols-plasma` (`text_input::v2`), not
upstream wayland-protocols 0.32 (that crate has v1+v3 only).

## Decision

1. **Advertise `zwp_text_input_manager_v2` next to v3.** Keep v3 for
   GTK. Same seat user-data and the same IME Activate path.
2. **v2 `enable(surface)` and `show_input_panel` Activate IME.**
   `commit_string` from the OSK is forwarded to the active v2 object
   as a plain string (not Option; no v3 `done`).
3. **Host-test only this week.** Next displayd flash (not this week)
   must carry ADR-311 `configure_bounds` **and** this v2 global.

## Consequences

- Qt on panther will be able to bind text-input after the next
  displayd flash. APP-04 is not closed until that flash and a typed
  field exist.
- Rollback: revert this commit; v3 path is unchanged.

## Verification

Host `cargo test -p saai-displayd --test text_input_v2`. dest-no-lock
kept. Do not flash. Leave Сейчас.
