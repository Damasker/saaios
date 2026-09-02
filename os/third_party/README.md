Debian busybox-static 1.37.0-6+b9 arm64 (GPL-2).
https://deb.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_arm64.deb
SHA256 (usr/bin/busybox): 7d93682be37cf6ed46699f6fff546b80bf133cf8ec342c2d04bc23387f78bc34

Dropbear + scp + sftp-server: static aarch64 musl. See dropbear-aarch64.url, dropbear-aarch64.sha256, dropbear-aarch64.copyright.
Binaries are gitignored; `make -f os/Makefile dropbear` re-fetches them.

iw: static aarch64 `iw` 6.9 + libnl-3.11 (nl80211). Gitignored `os/third_party/iw-aarch64`. Cross-built on R620 (`aarch64-linux-gnu-gcc -static`). Packed as `/sbin/iw` in v030. `make -f os/Makefile iw` checks the binary exists. ISC (`iw`) + LGPL-2.1 (`libnl`).

wpa_supplicant: static aarch64 `wpa_supplicant` 2.11 + `wpa_cli` (nl80211, internal TLS, libnl-3.11). Gitignored `os/third_party/wpa_supplicant-aarch64` and `wpa_cli-aarch64`. Built on R620 (`/tmp/e3-build-wpa.sh`) for the LIVE join; packed as `/sbin/wpa_supplicant` and `/sbin/wpa_cli` in v031. `make -f os/Makefile wpa` checks the binaries exist. **Do not pack `wpa_passphrase`.** BSD (`wpa_supplicant`). No PSK in this tree.

tinyalsa: clone `https://github.com/tinyalsa/tinyalsa` at `os/third_party/tinyalsa` (gitignored). Pin used on R620: `9fab97ca07184371ecad81154d1dadb09d0fa7cf`. BSD-3 / Android Open Source Project. `make -f os/Makefile beep` / `play` link `pcm.c`/`mixer.c` statically into `/sbin/beep` and `/sbin/play`. See [audio.md](../../docs/os/targets/sm-a127f/audio.md).

TD4150: clean Samsung DXJ6 sources in `td4150_oss_dxj6/` (17 files; zip `/srv/media/SM-A127F_SWA_13_Opensource.zip`). The built maze is gitignored `kernel_samsung_a12/.../td4150/`. Canonical: `docs/os/targets/sm-a127f/kernel-touch.md`. Do not apply maze opcode patches to OSS.
