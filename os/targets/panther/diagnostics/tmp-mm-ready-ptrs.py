#!/usr/bin/env python3
"""Find pointers to SIM-NOT-READY strings; dump USIM state names nearby."""
from pathlib import Path
import struct

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000


def sva(j):
    return VA + (j - MAIN)


def fo(va):
    return va - VA + MAIN


n = b"SIM NOT READY (usim state : %s)"
j = img.find(n)
va = sva(j)
print(f"str off={hex(j)} VA={hex(va)}")
# little-endian VA pointers
pat = struct.pack("<I", va)
c = 0
k = 0
while True:
    i = img.find(pat, k)
    if i < 0:
        break
    print(f"  ptr @ {hex(i)} fileVA={hex(sva(i))}")
    c += 1
    k = i + 1
    if c > 15:
        break
print(f"nptrs={c}")

n2 = b"SIM is not ready"
print("\nSIM is not ready hits:")
k = 0
while True:
    i = img.find(n2, k)
    if i < 0:
        break
    end = img.find(b"\0", i)
    s = img[i:end].decode("latin1", "replace")
    print(f"  {hex(sva(i))}: {s[:90]!r}")
    k = i + 1

# USIM state names used in that format string
print("\nUSIM state-like strings:")
for n in [
    b"USIM_NULL",
    b"USIM_INIT",
    b"USIM_PIN",
    b"USIM_READY",
    b"USIM_IDLE",
    b"USIM_ACTIVE",
    b"USIM_ABSENT",
    b"PIN_REQUIRED",
    b"SIM_PIN",
    b"CARD_READY",
]:
    print(f"  {n.decode()}: count={img.count(n)}")

# Reject cause 0 strings
print("\nreject-cause 0 / local deny:")
for n in [
    b"RejectCause = 0",
    b"reject cause 0",
    b"Cause=0",
    b"No reject cause",
    b"reject_cause=%d",
    b"RejectCause=%d",
    b"MmCause",
    b"EMM Cause",
    b"local reject",
    b"Internal reject",
]:
    j = img.find(n)
    extra = ""
    if j >= 0:
        extra = img[j : img.find(b"\0", j)].decode("latin1", "replace")[:80]
    print(f"  {n.decode()!r}: {extra!r}")
