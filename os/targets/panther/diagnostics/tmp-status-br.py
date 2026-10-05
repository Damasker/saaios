#!/usr/bin/env python3
from pathlib import Path
from capstone import Cs, CS_ARCH_ARM, CS_MODE_THUMB

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000
start, end = 0x14FB300, 0x14FB640
off = MAIN + (start - VA)
md = Cs(CS_ARCH_ARM, CS_MODE_THUMB)
md.detail = True
blob = img[off : off + (end - start)]
for insn in md.disasm(blob, start):
    m = insn.mnemonic
    if m in ("b", "beq", "bne", "bcs", "bcc", "bmi", "bpl", "bvs", "bvc",
             "bhi", "bls", "bge", "blt", "bgt", "ble", "cbz", "cbnz",
             "bl", "beq.w", "bne.w") or m.startswith("b") or m.startswith("cb"):
        print(f"0x{insn.address:08x}: {insn.mnemonic:8s} {insn.op_str}")
    if "cmp" in m or "ldrb" in m or "strb" in m:
        print(f"0x{insn.address:08x}: {insn.mnemonic:8s} {insn.op_str}")
