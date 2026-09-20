# ADR-248: product analytics of screens stays off

## Статус

Принято, 2026-09-20. Docs-only. No chrome. Not Visual v1 sign-off.
PIN stays null.

## Нумерация

После ADR-247 следующий свободный номер — **248**. Не S33.

## Контекст

Wave C closed local Observation (sampler, cache, Fresh status,
Система readout). Screen product-analytics is a different product:
funnels, dwell, which tab was open. Goal forbids Intent text in that
pipeline and requires default off. Export from the device stays later
and only with explicit consent.

## Decision

1. **Default off.** No screen-analytics sampler, no new EventBus
   kind, no cloud. `TelemetrySampler` stays `system.metrics` →
   Observation.
2. **No Intent text.** Analytics, if ever enabled, may not store or
   export Intent body, compose field, or transcription.
3. **Not Observation.** Screen events are not World Model facts and
   do not enter `ObservationCache` or entity-store.
4. **Consent later.** Device export of any analytics remains a later
   slice with an explicit grant. This ADR does not add UI for that.

## Consequences

Wave C is closed without a monitoring product. Enabling analytics
needs a new ADR, a default-off flag, and a test that Intent text is
absent. Rollback: this document only.

## Verification

Host: no new crate; `rg` does not find a screen-analytics sampler.
Panther: unchanged this slice. Marker on. Leave Сейчас.
