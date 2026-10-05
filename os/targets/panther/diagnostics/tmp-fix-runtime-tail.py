#!/usr/bin/env python3
from pathlib import Path

p = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/"
    "MODEM-RUNTIME-2026-09-24.md"
)
t = p.read_text(encoding="utf-8", errors="replace")
key = "### SET#5 / Present bypass hunt"
idx = t.rfind(key)
block = """### SET#5 / Present bypass hunt (2026-09-30 evening) — **none**

Whole-MAIN B (`449eeab3…`): sole SET#5 still `0x14fb5c6` + `LDRB +0xBF6` CMP#2;
sole +0xBF6 STRB STATUS copy; sole PresentObj(#636c) Present=2 = FN_A;
wide/STM/BF4 stores are ctor/enum noise; mid-STATUS BL hits are false friends.
Scripts: `tmp-set5-bypass-hunt.py`, `tmp-set5-alt-entries.py`. See MODEM-BLOCKER.

**Live (COM13):** ONLINE; `app=PIN pin1=2 present_infer=notin`; rmnet 0/0; no IPv4.

**Stock-EU paradox:** unresolved under current MAIN RE. No signed-elicitable
bypass → **no live try**. Next = external signed CDMA-RatMap CP **or** policy
exception (EFS TCS / rild / unsigned MAIN). Do not reseat.
"""
# Fix Next paragraph immediately before the section if present
pre = t[:idx] if idx >= 0 else t.rstrip() + "\n\n"
# Normalize common garbles in the stock-EU Next lines near the end
for a, b in [
    ("Present\ufffd2 as READY", "Present!=2 as READY"),
    ("Present?2 as READY", "Present!=2 as READY"),
    ("\ufffdrestore\ufffd Present=2.", '"restore" Present=2.'),
    ("?restore? Present=2.", '"restore" Present=2.'),
]:
    if a in pre[-800:]:
        pre = pre[:-800] + pre[-800:].replace(a, b)
t = pre + block
p.write_text(t, encoding="utf-8")
print("ok bytes", p.stat().st_size)
print(t[-500:])
