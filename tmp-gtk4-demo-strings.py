#!/usr/bin/env python3
from pathlib import Path
import re

p = Path("/tmp/gtk4-probe/bin/gtk4-demo")
d = p.read_bytes()
ss = re.findall(rb"[ -~]{8,}", d)
keys = [s.decode("ascii", "replace") for s in ss if b"entry" in s.lower()]
print("nstrings", len(ss), "entry-hits", len(keys))
for k in keys[:100]:
    print(k[:160])
