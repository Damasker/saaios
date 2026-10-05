#!/usr/bin/env python3
import subprocess
from pathlib import Path
lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
data = lib.read_bytes()
for n in (b"AutoVerifyPin", b"AutoPin", b"SetAutoPin", b"covertAutoPin",
          b"auto pin", b"AUTOPIN", b"VerifyPin after", b"pin disabled"):
    j = 0
    while True:
        j = data.find(n, j)
        if j < 0: break
        end = data.find(b"\0", j)
        s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:min(end if end>0 else j+80, j+100)])
        print(hex(j), s)
        j += 1

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(lib)], text=True, errors="replace")
for line in out.splitlines():
    if "AutoPin" in line or "AutoVerify" in line:
        print(line)
