# ADR-321: Falkon loads hello.html over loopback HTTP, not file://

## Статус

Принято, 2026-09-21. APP-06 panther document via HTTP. Not NetInternet.
Not public browse. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd.

## Нумерация

После ADR-320 следующий свободный номер — **321**. Не S33.

## Контекст

ADR-316 painted `file://…/hello.html` once fontconfig existed.
QtWebEngine's network service talks to 127.0.0.1 even for documents.
Empty NEWNET now has loopback (ADR-313). Granting `NetInternet` would
expose USB/wlan and needs a user grant (do not tap Разрешить). APP-06
still owed HTTP navigation, not only a file URL.

## Decision

1. **`launch.c` listens on `127.0.0.1:8765` only.** It forks a
   one-connection-at-a-time server of `share/hello.html`, then execs
   Falkon with `http://127.0.0.1:8765/hello.html` and
   `--private-browsing`. `PR_SET_PDEATHSIG` kills the server when
   Falkon dies. Bind failure is `_exit(125)`, not a silent file://
   fallback.
2. **The page says `loopback http`.** Screencap of that string is
   HTTP, not the previous file:// paint.
3. **Capabilities stay empty.** Loopback is not `NetInternet`. No
   grant JSON, no consent chrome, no public URL.

## Consequences

- WebEngine must fetch over HTTP inside the sandbox netns. Failure
  looks like ADR-313's "Failed loading page", not a dark hello.
- Rollback: restore `file://` in `launch.c`.
- Public browse and URL chrome still wait NetInternet + AUTH-10.
- Do not flash displayd/shell. Replace `bin/launch` and
  `share/hello.html` in the installed package only.

## Verification

Zig `aarch64-linux-musl` build of `launch.c`. Panther: launch
`org.saaios.demo.falkon`; `webengine.log` contains
`http://127.0.0.1:8765/hello.html`; screencap shows `loopback http`;
stop Дом · Сейчас. dest-no-lock kept. Leave Сейчас.
