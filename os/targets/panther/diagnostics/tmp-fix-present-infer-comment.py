#!/usr/bin/env python3
from pathlib import Path
import re

p = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/"
    "os/targets/panther/diagnostics/sit-sim-status.c"
)
t = p.read_text(encoding="utf-8")
pat = r"/\* Infer Present enum published by last STATUS.*?0x0200 does not carry \+0xBF6\. \*/"
new = """/* Infer Present enum published by last STATUS->SET_APP decision (RO MAIN):
   Present 1->PUK(#3), 2->READY(#5), 3->PERSO(#4), else (incl 0)->PIN(#2).
   DETECTED(#1) has no Present gate. 0x0200 byte17 = GET_APP +0xBF4 only —
   does NOT carry +0xBF6. Validated 2026-09-30 (not a mis-read of Present).
   Caveat: while app==PIN, STATUS may skip Present re-eval; label is last
   published decision. HotSwap live: PIN + notin_1_2_3 (Present was 0/else). */"""
nt, n = re.subn(pat, new, t, count=1, flags=re.S)
print("n=", n)
if n:
    p.write_text(nt, encoding="utf-8")
else:
    i = t.find("Infer Present")
    print(repr(t[i : i + 220]) if i >= 0 else "missing")
