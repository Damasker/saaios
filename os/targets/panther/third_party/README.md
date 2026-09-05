# Panther UI third-party components

## Inter 4.1

The native UI bundles `Inter-Regular.ttf` and `Inter-SemiBold.ttf` from the
official Inter 4.1 release. Inter is distributed under the SIL Open Font
License 1.1; the complete license is stored next to the font files in
`assets/fonts/OFL-1.1.txt`.

The checked-in files are build-time subsets containing Basic Latin, Latin-1,
Cyrillic, Cyrillic Extended-A/B, and the small set of UI symbols used by the
shell. This keeps both faces below 150 KiB combined while retaining Russian UI
and response text support.

Source: <https://github.com/rsms/inter/releases/tag/v4.1>

## stb_truetype

`stb/stb_truetype.h` is pinned to upstream commit
`2c980bb59875b0d32144a71867fbdebb2f77cd20`. It is used only with the trusted
font files shipped in the boot image; SaaiOS does not accept external fonts at
runtime.

Source: <https://github.com/nothings/stb>
