from pathlib import Path
p = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/tray-bearer-chase.sh")
b = p.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
p.write_bytes(b)
print("ok", b.startswith(b"#!/bin/sh\n"), "dirname" in p.read_text())
