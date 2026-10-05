#!/usr/bin/env python3
"""Disassemble DoGetSimStatus, BuildSimGetStatus, ipc open, RilContext phone id."""
import subprocess
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"

# Find BuildSimGetStatus in stream
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",stream], text=True, errors="replace")
for line in out.splitlines():
    if "BuildSimGetStatus" in line or "BuildGetSimStatus" in line:
        print(line)

# strings around umts_ipc
data = open(lib,"rb").read()
for n in (b"umts_ipc0", b"umts_ipc1", b"umts_rfs0", b"umts_rfs1"):
    j = 0
    while True:
        j = data.find(n, j)
        if j < 0: break
        print(hex(j), data[max(0,j-20):j+40])
        j += 1

# Find who references ipc1 - grep in readelf for IoChannel / ModemIo
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",lib], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("IoChannel", "ModemChannel", "SitClient", "OpenDevice", "GetPhoneId", "RIL_SOCKET", "SocketId", "mSocketId")):
        print("lib", line)
