#!/usr/bin/env python3
import subprocess
from pathlib import Path

IMG = "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/STAGED-ap1a-verizon-aa0dfc6160fc.img"
OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/ap1a-modem-fs")
OUT.mkdir(parents=True, exist_ok=True)

cmds = [
    "stats",
    "ls -l",
    "cd images",
    "ls -l",
    "pwd",
]

# Single interactive session via stdin
script = "\n".join(cmds) + "\nquit\n"
r = subprocess.run(["debugfs", IMG], input=script, capture_output=True, text=True)
print("STDOUT:\n", r.stdout)
print("STDERR:\n", r.stderr[:1000])

# Also try dump via script once we know names — first get listing only
# Parse filenames from ls
names = []
for line in r.stdout.splitlines():
    parts = line.split()
    if parts and parts[-1] not in (".", "..") and not line.startswith("debugfs"):
        # debugfs ls -l format ends with name
        name = parts[-1]
        if name and not name.startswith("("):
            names.append(name)
print("parsed names", names)
