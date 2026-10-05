#!/usr/bin/env python3
from pathlib import Path

t = Path("/home/mike/kernel-work/common-bd23337/include/linux/module.h").read_text()
s = t.find("struct module {")
# find closing of struct module - look for "\n};\n" after start, but nested braces...
depth = 0
i = s
end = None
while i < len(t):
    if t[i] == "{":
        depth += 1
    elif t[i] == "}":
        depth -= 1
        if depth == 0:
            end = i
            break
    i += 1
body = t[s : end + 1]
print("struct module bytes in source chars", len(body))
print("--- last 2500 chars ---")
print(body[-2500:])

# autoconf
ac = Path("/home/mike/kernel-work/common-bd23337/include/generated/autoconf.h")
if ac.exists():
    for ln in ac.read_text().splitlines():
        if "CFI_TRAP" in ln or "ARCH_USES_CFI" in ln or "MODULES_DATA" in ln or "BUILD_ID" in ln:
            print("AC", ln)

# this_module sizes again
print("sizes: ours 0x400 stock 0x440 diff", 0x440 - 0x400)
