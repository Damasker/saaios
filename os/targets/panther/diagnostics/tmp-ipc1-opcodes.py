#!/usr/bin/env python3
import subprocess
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",stream], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("Subscription", "DataAllowed", "AllowData", "SetUicc", "CardPower", "SimCardPower")):
        print("stream", line)

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",lib], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("UiccSubscription", "DoSetUiccSubscription", "SetDataAllowed", "AllowData", "DoRadioPower", "DoSetSimCardPower", "CardPowerHandler")):
        print("lib", line)

# disassemble BuildSetSimCardPower and BuildSetUicc and BuildRadioPower briefly for ids
for name, start, stop in (
    ("BuildSetSimCardPower", 0x80640, 0x806c4),
    ("BuildSetUicc", 0x805b0, 0x80640),
    ("BuildRadioPower", 0x76770, 0x76820),
):
    print("===", name)
    print(subprocess.check_output([
        "aarch64-linux-gnu-objdump","-d",
        f"--start-address={start:#x}", f"--stop-address={stop:#x}", stream
    ], text=True, errors="replace")[:1500])
