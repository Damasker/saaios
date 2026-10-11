# ADR-475: Locality weather is Open-Meteo forecast telemetry

## Статус

Принято, 2026-10-11. Pixel now has a bearer. Weather is still not a
NOW decoration. It is locality telemetry from a public forecast
document. Modem GNSS stays unpublished (MODEM-11). The widget shows
the configured query coordinates, not a camp GPS dump. Empty source
`weather.temp` stays rejected. Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-474 следующий свободный номер — **475**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Concept boards and ADR-241/308 forbade invented weather on Сейчас.
A live HTTP forecast is a different fact: source, time, and place
must be real. Open-Meteo Forecast `/v1/forecast` is public, no API
key, CC BY 4.0, and answers on the current bearer. Modem GPS lock
does not publish coordinates. The Местность widget still needs a
place to name, so it prints the lat/lon used for the request.

## Decision

1. **Source.** `open-meteo.forecast` is the only weather importer.
   Attribution is `Open-Meteo (CC BY 4.0)`. No key, no invented
   vendor. NOW shows that sourced line (ADR-476), not an invented
   card.
2. **Place.** Latitude and longitude are configuration
   (`SAAIOS_LOCALITY_*` or `--lat/--lon/--name`). Missing place is
   unavailable, not a guessed city. Query coordinates are
   `locality.latitude` / `locality.longitude` and appear on the
   widget. They are not modem GNSS.
3. **Facts.** `observations_from_open_meteo_current` emits
   `locality.name`, latitude, longitude, temperature, humidity,
   wind, WMO code, condition, and attribution. Quality is
   Approximate. Kind is Imported. TTL is 45 minutes. Missing
   `current` invents nothing.
4. **App.** `saai-weather` fetches that document (five attempts)
   and writes a cache. Система Местность is a tap: empty weather
   stays quiet until the fifth failure, then the widget shows the
   reason. Tap starts a new five-attempt cycle. The line shows
   temperature, condition, wind, humidity, and query coordinates.
   Сейчас shows the same line as `now.weather` (ADR-476).

## Consequences

- Weather exists as WORLD locality telemetry, not chrome fiction.
  Rollback: drop `locality.rs` and `saai-weather`. Still not Visual
  v1 sign-off.

## Verification

Host: fixture JSON becomes `locality.weather.*` plus query
coordinates with source `open-meteo.forecast`; empty/NaN place
emits nothing; widget line keeps main facts and lat/lon; reason
is held until five attempts. Live fetch is optional and not a
unit-test network. Panther: do not flash.
