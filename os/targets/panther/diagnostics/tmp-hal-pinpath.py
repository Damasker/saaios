#!/usr/bin/env python3
"""Look for HAL init commands sent before/around first GetSimStatus on cold boot."""
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
# Disassemble CheckAndAutoVerifyPin and FillRilCardStatusFromAdapter for pin1==3 path
for name, start, stop in [
    ("CheckAndAutoVerifyPin", 0x156650, 0x156790),
    ("FillRilCardStatusFromAdapter", 0x1553E0, 0x155720),
    ("BuildRilCardStatusApplications", 0x154F00, 0x1553E0),
    ("SimService_OnSimStatusChanged", 0x154940, 0x154BA0),
]:
    print("===", name)
    print(
        subprocess.check_output(
            [
                "aarch64-linux-gnu-objdump",
                "-d",
                f"--start-address={start:#x}",
                f"--stop-address={stop:#x}",
                str(lib),
            ],
            text=True,
            errors="replace",
        )
    )
