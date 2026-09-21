# ADR-313: empty NEWNET brings up loopback, not the public net

## Статус

Принято, 2026-09-21. APP-06 sandbox. Not a browse. Not Visual v1
sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-312 следующий свободный номер — **313**. Не S33.

## Контекст

ADR-312 launched Falkon through appd with empty capabilities, so
the sandbox takes `CLONE_NEWNET`. A fresh netns has `lo` down and
unaddressed. Falkon's session then restored `https://www.falkon.org`
and painted Chromium's "Failed loading page" (`#dddddd`). The same
error sat in `session.dat` as a `data:` document. `file://` of a
local `share/hello.html` did not replace it: QtWebEngine's network
service still talks to 127.0.0.1.

Loopback is not `NetInternet`. Granting that capability would
expose the USB/wlan routes. Bringing `lo` up inside the empty
netns does not.

## Decision

1. **`bring_up_loopback` after `unshare(NEWNET)`.** `SIOCSIFADDR`
   `127.0.0.1/8`, then `IFF_UP|IFF_RUNNING`, before capability
   drop. Skip when `NetInternet` is already granted (host netns).
2. **Local hello document.** `apps/falkon-demo/hello.html` ships
   in the package. `launch.c` execs
   `falkon --private-browsing file://$cwd/share/hello.html`.
   Private browsing skips the HTTPS session restore.
3. **Not a painted page.** After loopback, `/proc/<falkon>/net`
   shows `127.0.0.1` LOCAL and packets on `lo`. Renderer
   processes run. Screencap of `file://` and of a `data:`
   document with a dark body stay the white widgets surface
   under the shell status strip. QtWebEngine contents do not
   blit onto the ADR-312 `wl_shm` hello-frame.
4. **appd on `/data`, not a displayd flash.** Binary
   `e9b3c57c…`. native-init restarted pid 11700. dest-no-lock
   kept.

## Consequences

- Apps without `NetInternet` can use 127.0.0.1. They still
  cannot reach 172.31.7.0/24 or wlan.
- Rollback: revert `sandbox.rs` and replace `/data/saaios/system/saai-appd`.
- Visible Falkon chrome / APP-04 field is a later WebEngine
  software-blit slice, not this one.

## Verification

Host: `cargo test -p saai-appd` and clippy `-D warnings`.
Panther: `fib_trie` `127.0.0.1`, `lo` packets, Falkon pid with
`--private-browsing file://…/hello.html`. Screencap remains
white. Stop returns Дом · Сейчас. Leave Сейчас.
