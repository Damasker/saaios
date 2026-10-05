#!/usr/bin/env python3
from pathlib import Path
src = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/extract-ap1a-radio.sh")
dst = Path("/tmp/extract-ap1a-radio.sh")
text = src.read_text(encoding="utf-8").replace("\r\n", "\n").replace("\r", "\n")
dst.write_bytes(text.encode("utf-8"))
print("wrote", dst, "cr", dst.read_bytes().count(b"\r"))
