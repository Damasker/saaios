#!/usr/bin/env python3
from pathlib import Path
# Minimal OOT-style isn't available quickly; use a shell+devmem/busybox approach via
# a tiny C that only ioremaps — ship as prebuilt? Better: extend poke with skip_atu=1.
# Write patch to saaios_cp_poke.c for skip_atu + multi-byte RO dump.
src = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/saaios_cp_poke.c")
text = src.read_text(encoding="utf-8")
if "skip_atu" not in text:
    # Insert skip_atu param and branch — do via separate write below
    pass
print("will patch poke for skip_atu")
