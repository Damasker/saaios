#!/usr/bin/env python3
import subprocess
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"

# Find BuildSimGetStatus and InitRequestHeader
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",stream], text=True, errors="replace")
for line in out.splitlines():
    if "BuildSimGetStatus" in line or "InitRequestHeader" in line or "GetSimStatus" in line:
        print("stream", line)

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",lib], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("GetPhoneId", "GetSlotId", "m_nPhone", "DoGetSimStatus", "SimGetStatus", "OpenChannel", "MultiSim", "GetModemId", "SitIo", "IpcChannel")):
        print("lib", line)
