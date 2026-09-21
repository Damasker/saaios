# ADR-307: Laptop is a second surface of the panther SOM store

## Статус

Принято, 2026-09-21. PCE-25 D remainder. Not a replica. Not PCE-01..24.
Not Visual v1 sign-off. PIN stays null. Do not flash shell or displayd.

## Нумерация

После ADR-306 следующий свободный номер — **307**. Не S33.

## Контекст

ADR-249 forbids Resource Scheduler, replica, and placement.
ADR-252 left "shared Space/Entity/Intent/Observation across nodes"
as the D remainder. entityd spoke only Unix. The laptop window
therefore opened an empty local store, not the phone-gate objects.

USB NCM `172.31.7.1` already carries runtime TCP `38127` (ADR-073).
Observation can ride that. Space/Entity/Intent need the same store,
second transport, not a second world.

## Decision

1. **One store.** panther `saai-entityd` remains the writer. The
   laptop is a client. No replica log, no new space UUID, no PSK.
2. **Same wire.** Newline JSON, schema 1, over TCP
   `172.31.7.1:38128`. Default bind is fail-soft: if the gadget
   address is absent, UDS-only. `--tcp-bind none` disables.
   `--tcp-bind ADDR` is required.
3. **Laptop fallback.** x86 shell tries UDS first. If that is
   missing and USB NCM `172.31.7.2` exists, it connects TCP.
   panther (`phone_gate_surface`) never uses TCP. Unit tests do not.
4. **Observation.** The same USB check reads runtime `status` at
   `172.31.7.1:38127` when no local sock exists. Timeout 250 ms.
5. **Not a flash of shell/displayd.** entityd TCP is a service
   binary. Panther listener waits the entityd binary on `/data`.

## Consequences

- Two clients (UDS + TCP) see the same Intent in host tests.
- Until panther entityd is replaced, laptop Entity/Intent TCP
  gets connection refused; Observation can already work because
  runtime already binds `38127`.
- Rollback: drop `--tcp-bind` and the shell TCP fallback.

## Verification

Host: `cargo test -p saai-entityd --offline tcp_client_lists_the_same`
and `cargo test -p saai-shell --offline entityd_tcp_is_off_in_unit_tests`.
No panther shell/displayd flash. Leave Сейчас.
