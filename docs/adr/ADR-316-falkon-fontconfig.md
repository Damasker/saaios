# ADR-316: Falkon paints hello.html once fontconfig has a config

## Статус

Принято, 2026-09-21. APP-06 panther document. Not a browse. Not
Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-315 следующий свободный номер — **316**. Не S33.

## Контекст

ADR-315 gave Chromium a real procfs and 128 MiB `/tmp`. The 64 MiB
and inotify errors went away. Renderers still `ProcessGone: 3 (5)`
immediately after "Falkon: 1 extensions loaded". That exit is
SIGTRAP. ADR-315 named the ADR-012 keymap class. The same log also
said `Fontconfig error: Cannot load default config file: No such
file: (null)`. Qt widgets tolerate a missing config. Blink does
not: it `__builtin_trap`s. Sandbox already reveals `/saaios/fonts`
(ADR-081). appd only sets `FONTCONFIG_PATH` when
`<code_dir>/etc/fonts` exists. The Falkon package had neither
directory nor `fonts.conf`.

## Decision

1. **Ship `etc/fonts/fonts.conf`.** It lists `/saaios/fonts` and
   caches under `/tmp/fontconfig-cache`. Supervisor then sets
   `FONTCONFIG_PATH`. `launch.c` sets `FONTCONFIG_PATH` and
   `FONTCONFIG_FILE` from cwd so a future package rebuild does not
   depend on that one env.
2. **Do not pack a TTF into Falkon.** Montserrat is already on the
   device at `/saaios/fonts`.
3. **ADR-315's SIGTRAP was this, not keymap.** Ozone already
   fell back to `StubKeyboardLayoutEngine` in the browser process.
   After the config exists: no Fontconfig error, `ProcessGone`
   count 0, a live `--type=renderer`, and `file://…/hello.html`
   paints (dark `#1a1208`, "SaaiOS", `local field`). Still no
   NetInternet. Still not a URL chrome/browse.

## Consequences

- Live tree: `etc/fonts/fonts.conf` under
  `/data/saaios/apps/org.saaios.demo.falkon`. Recipe copies it on
  the next package rebuild. appd binary unchanged (`2503f4f5…`).
- Host qemu hello-frame does not use `launch.c` or this config.
- Rollback: remove `etc/fonts`. Renderer dies SIGTRAP again.
- APP-04 still has not typed the Qt field on panther.

## Verification

Panther: `webengine.log` without the Fontconfig line; `ps` shows
`--type=renderer`; screencap of hello.html; dest-no-lock kept.
Stop Дом · Сейчас. Leave Сейчас.
