# Panther UI third-party components

## Montserrat

The native UI sans family is `Montserrat-Regular.ttf` and
`Montserrat-SemiBold.ttf`. They are static 400/600 instances retained from the
physically reviewed S13 font change; see ADR-057 and ADR-070. Montserrat is
distributed under the SIL Open Font License 1.1, stored as
`assets/fonts/Montserrat-OFL.txt`.

The files include the Cyrillic coverage required by the Russian and Ukrainian
SaaiOS interface. Their checked-in SHA-256 values are:

- Regular: `0a6e6afb7a98e0e7bfd6c78d12a899b96e00b3d52e9ea76f2b4896c09afd34fe`
- SemiBold: `91d71d817f649044c17089e4e4866dbeeb59bc6408619ca1a4506b14a1bab2da`

Source family: <https://github.com/google/fonts/tree/main/ofl/montserrat>

## IBM Plex Mono

Technical identifiers, commands, addresses, logs, and telemetry use the static
`IBMPlexMono-Regular.ttf`. It has Latin, Cyrillic, and Ukrainian coverage and
is distributed under the SIL Open Font License 1.1, stored as
`assets/fonts/IBMPlexMono-OFL.txt`.

- Upstream: <https://github.com/IBM/plex>
- Pinned commit: `78cd4223d8de9fcb78cba84eadecb269c56093c5`
- Font SHA-256: `7c6fbddca4b700be918f5f6183d9bd4464fa427fe435f0b480d77fe2bb8c5a43`
- License SHA-256: `7e6b2818edbd8f6a01ae80641cc8f16a51080d08fb4e532be3a0b6f74adb07da`

The shell falls back to the regular sans face if the optional mono asset is
missing or invalid, so a packaging error cannot remove all interface text.

## Feather Icons

The line-icon family (VUI-02, ADR-100) is the static `FeatherIcons.ttf`, a
webfont build of the Feather icon set (Cole Bemis and contributors), loaded
through the same `fontdue` path as the sans and mono text faces -- an icon is
just a glyph at a Private Use Area codepoint. Distributed under the MIT
License, stored as `assets/fonts/FeatherIcons-LICENSE.txt`.

- Icon source and license origin: <https://github.com/feathericons/feather>,
  commit `3dc050d97405062eba78aa57115c0a15c63abdaa`.
- Webfont build (the actual `.ttf` shipped here):
  <https://github.com/niklauslee/feather-webfont>, npm package version
  `4.22.3`, commit `9e852be2566542ddbfa2ddb9cc55c4f918b9bba5`.
- Font SHA-256: `f4aee0768b9c35d1269d1e4e79c89a9d03ababd4a88d8063bbad63a5b7d5b679`
- License SHA-256: `308028e93fcf84972523cdf6e616f73168546b4953895f516d01287f16fe7bee`

`saai-ui-core::IconGlyph` is the product API; callers never name codepoints or
the font file. There is no meaningful fallback for a missing icon glyph (a
letter cannot substitute for a wifi/lock/chevron mark) -- a missing or invalid
asset renders nothing for that one icon rather than failing the whole shell,
the same fail-soft posture the mono face already has for text.

## stb_truetype

`stb/stb_truetype.h` is pinned to upstream commit
`2c980bb59875b0d32144a71867fbdebb2f77cd20`. It is used only with the trusted
font files shipped in the boot image; SaaiOS does not accept external fonts at
runtime.

Source: <https://github.com/nothings/stb>
