#!/usr/bin/env python3
import subprocess

lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"

def symbols(path, substrs):
    out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", path], text=True, errors="replace")
    for line in out.splitlines():
        if all(s.lower() in line.lower() for s in substrs) or (
            any(s.lower() in line.lower() for s in substrs) and ("Build" in line or "DoSet" in line or "DoGet" in line or "Handler" in line or "Mapping" in line)
        ):
            # looser filter below
            pass
    for line in out.splitlines():
        low = line.lower()
        if any(s.lower() in low for s in substrs):
            print(line)

print("=== libsitril slot/mapping ===")
symbols(lib, ["slot", "mapping", "modems", "port", "dsds", "logical", "physical"])

print("=== sit-stream builders ===")
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", stream], text=True, errors="replace")
for line in out.splitlines():
    low = line.lower()
    if "build" in low and any(k in low for k in ("slot", "modem", "map", "port", "config", "logical", "physical", "dsds", "uicc")):
        print(line)
    if "setlogical" in low or "slotmap" in low or "modemsconfig" in low:
        print(line)
