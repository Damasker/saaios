#!/usr/bin/env python3
import subprocess
from pathlib import Path

stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")
lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(stream)], text=True, errors="replace")
keys = ("TerminalProfile", "TerminalCapability", "ProfileDownload", "StkProfile",
        "BuildSim", "BuildGetVoice", "BuildSetTerminal", "BuildEnvelope",
        "BuildRefresh", "BuildReportStk", "BuildGetImei", "BuildDeviceInfo",
        "BuildBaseband", "BuildGetSignal", "BuildOperator", "BuildSetupEvent")
for line in out.splitlines():
    if "Build" in line and any(k.replace("Build","") in line or k in line for k in keys):
        print(line)
    if any(k in line for k in ("TerminalProfile", "ProfileDownload", "SetupEventList", "StkSetProfile")):
        print(line)

data = lib.read_bytes()
for n in (b"Terminal Profile", b"TERMINAL PROFILE", b"SET_PROFILE", b"StkSetProfile",
          b"ProfileDownload", b"SETUP EVENT", b"CheckAndAutoVerifyPin",
          b"APPSTATE_READY", b"ignores pin", b"pin state disabled"):
    j = data.find(n)
    if j >= 0:
        end = data.find(b"\0", j)
        print(hex(j), data[j:end][:100])

# list ProtocolSimBuilder Build* we may not have sent
print("== all ProtocolSimBuilder ==")
for line in out.splitlines():
    if "ProtocolSimBuilder" in line and "Build" in line:
        print(line)
