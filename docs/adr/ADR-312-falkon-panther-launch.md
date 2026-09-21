# ADR-312: Falkon launches on panther through saai-appd

## Статус

Принято, 2026-09-21. APP-06 panther hello-frame. Not a browse.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-311 следующий свободный номер — **312**. Не S33.

## Контекст

ADR-306 hashed a Falkon shm frame on host qemu. file-recv refuses
PUT over 128 MiB, so the 790 MiB package could not land as one
file. First appd launch mapped `xdg_toplevel` and committed
`(no buffer)` — QtWebEngine waited on GPU. Host qemu already
used `--disable-gpu` / `QT_QUICK_BACKEND=software`.

## Decision

1. **Chunked PUT.** `tar cz` the package (317 MiB), `split -b 100m`
   into four parts, PUT each, `cat` + `tar xz` under incoming.
   Do not raise file-recv's 128 MiB cap this week.
2. **Real `Install` / `Launch` / `Stop`.**
   `org.saaios.demo.falkon` via `/run/saaios/appd.sock`. Empty
   capabilities. Private dbus-daemon as ADR-098.
3. **Software scanout in `launch.c`.** Same Chromium/Quick flags
   as ADR-306 host test. First launch without them never attached
   a buffer. Second launch pid 22349 commits hashed `wl_shm`
   (`frame sha256=af2123d1…` / `78d4a010…`).
4. **Hello-frame is the surface, not a URL.** No `NetInternet`.
   Screencap is a white client under the shell status strip.
   Falkon chrome (tabs/URL) is not this slice.

## Consequences

- The app stays installed under `/data/saaios/apps/`. Remove is
  a later `appd` call, not this commit.
- Rollback: `Remove` the app id. Revert `launch.c` env.

## Verification

appd `installed` then `launched` pid 22349. displayd
`client connected pid=22349`, `new xdg_toplevel 1080x2400`,
`frame sha256=`. dest-no-lock kept. Stop returns Дом · Сейчас.
Leave Сейчас.
