#!/usr/bin/env python3
# Dump SIT_* name strings in file order within the CP names-table region, to
# derive opcode IDs by position relative to known anchors. Symbol strings only.
import re
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(CP, "rb").read()

# Find the contiguous names-table region: cluster of SIT_* C-strings.
# Collect all SIT_* occurrences, then print runs where they are densely packed.
hits = [(m.start(), m.group().decode()) for m in re.finditer(rb"SIT_[A-Z0-9_]{2,60}\x00", data)]
print("total NUL-terminated SIT_ strings:", len(hits))
# Group into dense runs (gap < 64 bytes between consecutive starts)
runs=[]; cur=[hits[0]]
for prev,(off,name) in zip(hits, hits[1:]):
    if off - prev[0] < 80:
        cur.append((off,name))
    else:
        runs.append(cur); cur=[(off,name)]
runs.append(cur)
runs.sort(key=len, reverse=True)
print("biggest runs:", [(hex(r[0][0]), len(r)) for r in runs[:4]])
# Print the biggest run fully (this is the id-ordered name table)
big = sorted(runs[0], key=lambda x:x[0])
print("==== ordered names table (%d entries) start=%x ====" % (len(big), big[0][0]))
for i,(off,name) in enumerate(big):
    print(f"{i:4d} @{off:x} {name}")
