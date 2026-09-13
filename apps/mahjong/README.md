# org.saaios.mahjong

A simple tile-matching game (6x4 board, 12 pairs), independent of
`saai-shell`/`saai-displayd` -- built as a real test of installing a
genuinely new third-party-style app through `saai-appd`'s `Install`
RPC (see ADR-072). `capabilities = []`: touches no capability-gated
resource, so launching it never shows the consent screen.

Bundled into the `init_boot` image under `saaios/packages/
org.saaios.mahjong/` (no lighter on-device delivery channel exists
yet -- see ADR-072). Installing it for real still requires running,
once, on the device:

```
/saaios/packages/org.saaios.mahjong/bin/saai-mahjong --install
```

This copies the package into `saai-appd`'s own `apps/` directory via
the real `Install` RPC; after that it shows up on "Сейчас" like any
other installed app.
