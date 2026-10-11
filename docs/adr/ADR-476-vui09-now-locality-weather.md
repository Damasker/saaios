# ADR-476: VUI-09 — NOW locality weather is a live Button

## Статус

Принято, 2026-10-11. Locality weather is real Open-Meteo telemetry
(ADR-475). The user asked that it appear on Сейчас. The row is
`now.weather`. Empty weather still waits five attempts, then shows
the reason. Tap starts the cycle again. Query coordinates stay on
the line. Modem GNSS stays unpublished. `now-public.sui` leftover
`now.empty` stays Fill. Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-475 следующий свободный номер — **476**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-241/308/475 forbade invented weather on NOW. A sourced forecast
is a different fact. The board wants compact device summary on
Сейчас. Система already shows Местность. The same line now docks
as the first stacked NOW DataRow so it is visible without opening
Система.

## Decision

1. **Host-lock** a NOW document with `now.weather` as a DataRow
   Button after `ObjectSummary` and before attention rows.
2. **Live.** The row label is «Местность». The secondary line is
   `saai_weather::widget_status()`: temperature, condition, wind,
   humidity, and query lat/lon, or the failure reason after five
   attempts. Tap calls `refresh_locality` and redraws. Do not fetch
   on every NOW paint.
3. **Empty.** Weather is compact chrome, not workflow content.
   Whole-screen «Ничего срочного» / offline still means no pending
   work (ADR-112/114). Do not invent a city.
4. **Not this slice.** Do not change `now-public.sui`. Do not enable
   the composer mic. Do not publish modem GNSS.

## Consequences

- Сейчас shows sourced weather without replacing attention or the
  calm empty pattern. Rollback: omit `now.weather`. Still not
  Visual v1 sign-off.

## Verification

Host: `now.weather` hits stacked slot 0; attention starts at slot
1; public `now.empty` stays Fill leftover. Panther: do not flash;
leave Сейчас.
