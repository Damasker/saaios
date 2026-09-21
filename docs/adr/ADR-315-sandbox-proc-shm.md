# ADR-315: sandbox procfs and 128 MiB tmp/shm; renderer still SIGTRAP

## Статус

Принято, 2026-09-21. APP-06 sandbox. Not a browse. Not Visual v1
sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-314 следующий свободный номер — **315**. Не S33.

## Контекст

ADR-314 packed swrast. `launch.c` now mirrors stdout/stderr to
`$SAAIOS_DATA_DIR/webengine.log` because appd nulls them. The log
showed Chromium `Less than 64MB of free space in temporary
directory for shared memory files: 8` (`/tmp` tmpfs was 16 MiB)
and `Failed to read /proc/sys/fs/inotify/max_user_watches`
(`/proc` was a 64 k tmpfs). Then
`ProcessGone: 3 (5)` in a tight loop — renderer crash, exit 5
(SIGTRAP).

## Decision

1. **Mount a real procfs** instead of masking `/proc`. Chromium
   needs maps/pid/inotify. Without `CLONE_NEWPID` this is still
   the host pid view; seccomp already covers ptrace/kill
   (ADR-020). `/sys` stays masked.
2. **`/tmp` and `/dev/shm` are 128 MiB tmpfs.** Chromium will not
   start discardable shared memory under 64 MiB free. Kirigami
   did not need this. Pixel 7 has 8 GiB; one browser may use
   that RAM.
3. **Keep the webengine.log.** It is how this slice is proven.
   Not product chrome.
4. **Not a painted page.** After the bump, the 64 MiB and inotify
   errors are gone. `/proc` lists pids, `size=131072k` on tmp
   and shm. Renderers still `ProcessGone: 3 (5)` immediately
   after "Falkon: 1 extensions loaded". Same SIGTRAP class as
   ADR-012 keymap. Displayd is not flashed this week.

## Consequences

- appd `2503f4f5…`, native-init restarted pid 9941.
- Rollback: remask `/proc`, restore 16 m/8 m.
- Visible Falkon document waits on a renderer that does not
  SIGTRAP, not on more Mesa.

## Verification

Host: `cargo test -p saai-appd`, clippy `-D warnings`.
Panther: log without the 64 MiB line; mountinfo `size=131072k`;
screencap still white. dest-no-lock kept. Stop Дом · Сейчас.
Leave Сейчас.
