# ADR-265: Visual v1 stays unsigned; public `.sui` stays Experimental

## Статус

Принято, 2026-09-20. Docs + existing compiler tests. Not Visual v1
sign-off. PIN stays null. `compile_v2_public()` is Experimental.
Thin visual tuning (ADR-213) is not next. APP-02 (GTK4) waits for a
displayd experiment.

## Нумерация

После ADR-264 следующий свободный номер — **265**. Не S33.

## Контекст

Goal wave F asked to keep Visual v1 unsigned until B is truthful, keep
`compile_v2_public` Experimental, and not queue ADR-213. B's five
surfaces are on panther. VUI-09 known limitations (ADR-193) still
list unsigned Visual v1. APP-COMPAT's next vertical is APP-02 on
`saai-displayd` (ADR-025). This week's hardware-changing shell
experiment is already spent.

## Decision

1. **Unsigned.** Visual Language v1 is not accepted. No Stable public
   `.sui` names. `sui_v2_stability` stays Experimental for public
   names; privileged stays `compile_v2` only.
2. **APP-02 later.** GTK4 shm crash remains ADR-025 until a panther
   GTK4 frame. ADR-266 is the host compositor protocol only. Do not
   paint Android VM.
3. **ADR-213 last.** Not scheduled as next.

## Consequences

- Product boards are an operating environment on panther, not a signed
  Visual v1.
- Rollback: none — this records a gate, it does not add chrome.

## Verification

Host: existing `compile_v2_public` tests still require Experimental
and reject privileged names. No panther flash. Leave Сейчас.
