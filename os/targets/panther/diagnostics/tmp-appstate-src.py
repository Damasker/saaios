#!/usr/bin/env python3
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(stream)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("GetAppState", "GetPinState", "SimStatus", "AppStatus",
                               "FillRilCard", "BuildRilCard", "covertPin", "PinState",
                               "ApplicationState", "GetApplication")):
        print("stream", line)

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(lib)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("GetAppState", "BuildRilCard", "FillRilCard", "covertPin",
                               "PinState", "AppState", "PIN_REQUIRED", "DISABLED",
                               "TerminalProfile", "SimRefresh", "ProfileDownload")):
        if "UND" in line and "GetApp" not in line and "GetPin" not in line and "Build" not in line and "Fill" not in line:
            continue
        if any(k in line for k in ("AppState", "PinState", "BuildRil", "FillRil", "Terminal", "Refresh", "Profile", "DISABLED", "PIN_")):
            print("lib", line)

# strings
for name, p in (("lib", lib), ("stream", stream)):
    data = p.read_bytes()
    print("==", name, "strings")
    for n in (b"PIN_REQUIRED", b"PINSTATE_DISABLED", b"APPSTATE_PIN", b"app_state",
              b"pin1", b"GetAppState", b"covertPinState", b"PinStateToString",
              b"Terminal Profile", b"TERMINAL PROFILE", b"SimRefresh",
              b"PROFILE DOWNLOAD", b"ignore pin", b"pin disabled"):
        j = data.find(n)
        if j >= 0:
            end = data.find(b"\0", j)
            s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:min(end if end > 0 else j+80, j+100)])
            print(f"  {j:#x} {s}")
