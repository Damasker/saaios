#!/usr/bin/env python3
import os
import subprocess
from pathlib import Path

sysroot = Path("/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot")
src = Path("/tmp/saaios-b2/services/saai-displayd/tests/gtk414_popover.c")
out = Path("/tmp/gtk4-probe/bin/gtk414-popover")
zig = Path("/home/mike/.local/zig-linux-x86_64-0.13.0/zig")
env = os.environ.copy()
env["PKG_CONFIG_SYSROOT_DIR"] = str(sysroot)
env["PKG_CONFIG_LIBDIR"] = f"{sysroot}/usr/lib/pkgconfig:{sysroot}/usr/share/pkgconfig"
cflags = subprocess.check_output(["pkg-config", "--cflags", "gtk4"], env=env).decode().split()
out.parent.mkdir(parents=True, exist_ok=True)
cmd = [
    str(zig),
    "cc",
    "-target",
    "aarch64-linux-musl",
    "--sysroot",
    str(sysroot),
    *cflags,
    "-o",
    str(out),
    str(src),
    f"-L{sysroot}/usr/lib",
    "-lgtk-4",
    "-lpangocairo-1.0",
    "-lpango-1.0",
    "-lharfbuzz",
    "-lgdk_pixbuf-2.0",
    "-lcairo-gobject",
    "-lcairo",
    "-lgraphene-1.0",
    "-lgio-2.0",
    "-lgobject-2.0",
    "-lglib-2.0",
    "-Wl,-rpath,/lib",
]
subprocess.check_call(cmd)
print("wrote", out, out.stat().st_size)
