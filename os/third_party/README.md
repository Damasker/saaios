Debian busybox-static 1.37.0-6+b9 arm64 (GPL-2).
https://deb.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_arm64.deb
SHA256 (usr/bin/busybox): 7d93682be37cf6ed46699f6fff546b80bf133cf8ec342c2d04bc23387f78bc34

Dropbear + scp + sftp-server: static aarch64 musl. See dropbear-aarch64.url, dropbear-aarch64.sha256, dropbear-aarch64.copyright.
Binaries are gitignored; `make -f os/Makefile dropbear` re-fetches them.

TD4150: clean Samsung DXJ6 sources in `td4150_oss_dxj6/` (17 files; zip `/srv/media/SM-A127F_SWA_13_Opensource.zip`). The built maze is gitignored `kernel_samsung_a12/.../td4150/`. Canonical: `docs/os/targets/sm-a127f/kernel-touch.md`. Do not apply maze opcode patches to OSS.
