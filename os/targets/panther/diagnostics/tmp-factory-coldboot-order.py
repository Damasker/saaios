#!/usr/bin/env python3
"""Extract factory cold-boot BuildSim*/NET order from libsitril + sit-stream."""
import struct
from pathlib import Path

def strings(data, min_len=6):
    out = []
    cur = bytearray()
    start = 0
    for i, b in enumerate(data):
        if 32 <= b < 127:
            if not cur:
                start = i
            cur.append(b)
        else:
            if len(cur) >= min_len:
                out.append((start, cur.decode("ascii", "replace")))
            cur = bytearray()
    return out

base = Path(".")
for lib in ("libsitril.so", "sit-stream.so", "sit-base.so"):
    data = (base / lib).read_bytes()
    print(f"\n######## {lib} size={len(data)} ########")
    interesting = []
    for off, s in strings(data, 5):
        sl = s.lower()
        keys = (
            "buildsim", "getstatus", "verifypin", "radio", "preferred",
            "networkselection", "registration", "cold", "boot", "simstatus",
            "ongetsim", "checkandauto", "requestsim", "setradiopower",
            "getsimstatus", "cardstatus", "pin", "online", "stack",
            "sim_init", "start_stack", "slotstatus", "openchannel",
        )
        if any(k in sl for k in keys):
            interesting.append((off, s))
    # print unique-ish Build* symbols
    builds = [x for x in interesting if "Build" in x[1] or "Do" == x[1][:2] or "On" == x[1][:2] or "Request" in x[1]]
    print(f"interesting={len(interesting)} buildish={len(builds)}")
    for off, s in builds[:80]:
        print(f"  {hex(off)} {s[:120]}")

# nm-like: look for exported BuildSim* in sit-stream
data = Path("sit-stream.so").read_bytes()
# ELF dynsym rough: just grep ascii BuildSim
print("\n=== sit-stream Build* names ===")
for off, s in strings(data, 8):
    if s.startswith("Build") or s.startswith("_ZN") and b"Build" in s.encode():
        if "Sim" in s or "Radio" in s or "Network" in s or "Preferred" in s or "Registration" in s:
            print(f"  {hex(off)} {s[:140]}")

# libsitril call sequence hints
data = Path("libsitril.so").read_bytes()
print("\n=== libsitril sequence-ish strings ===")
for off, s in strings(data, 8):
    if any(k in s for k in (
        "GetSimStatus", "VerifyPin", "SetRadioPower", "GetRadioState",
        "SetPreferred", "NetworkSelection", "GetDataRegistration",
        "GetVoiceRegistration", "OnGetSimStatus", "CheckAndAuto",
        "DoVerifyPin", "RequestSim", "SIM_STATUS", "card state",
        "app_state", "PIN_", "READY", "ONLINE",
    )):
        print(f"  {hex(off)} {s[:140]}")
