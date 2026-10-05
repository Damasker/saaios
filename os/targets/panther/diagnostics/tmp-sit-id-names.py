#!/usr/bin/env python3
"""Map SIT request ids to stock libsitril builder names (read-only).

For every *Builder::Build*/Get*/Set* symbol, the first MOVZ W immediate that
looks like a SIT id is taken as the request id (same heuristic as
tmp-decode-unsent-stock.py). Usage: tmp-sit-id-names.py libsitril.so ids.txt
"""
import importlib.util
import re
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "dec", Path(__file__).with_name("tmp-decode-unsent-stock.py"))
src = Path(spec.origin).read_text()
helpers = src.split("\nss = parse_dynsym")[0].replace(
    'ril = (DIAG / "libsitril.so").read_bytes()', "").replace(
    'stream = (DIAG / "sit-stream.so").read_bytes()', "")
ns: dict = {}
exec(compile(helpers, "helpers", "exec"), ns)

ril = Path(sys.argv[1]).read_bytes()
syms = ns["parse_dynsym"](ril)
ID_RANGES = ((0x0100, 0x0FFF), (0x4600, 0x46FF))


def first_id(addr: int):
    off = ns["va_to_off"](ril, addr)
    if off is None:
        return None
    chunk = ril[off: off + 0x80]
    for i in range(0, len(chunk) - 3, 4):
        insn = int.from_bytes(chunk[i: i + 4], "little")
        if (insn & 0x7F800000) == 0x52800000 and not (insn >> 21) & 3:
            v = (insn >> 5) & 0xFFFF
            if any(lo <= v <= hi for lo, hi in ID_RANGES):
                return v
    return None


names: dict[int, list[str]] = {}
for name, addr in syms.items():
    m = re.match(r"_ZN(\d+)", name)
    if not m:
        continue
    pos = m.end()
    cls = name[pos: pos + int(m.group(1))]
    mm = re.match(r"(\d+)", name[pos + len(cls):])
    if not cls.endswith("Builder") or not mm:
        continue
    start = pos + len(cls) + mm.end()
    meth = name[start: start + int(mm.group(1))]
    if not re.match(r"(Build|Get|Set|Send)", meth):
        continue
    sid = first_id(addr)
    if sid is not None:
        names.setdefault(sid, []).append(f"{cls.replace('Protocol', '')}::{meth}")

print(f"# symbols={len(syms)} builder_ids={len(names)}")
for line in Path(sys.argv[2]).read_text().splitlines():
    m = re.search(r"0x([0-9a-f]{4})", line)
    if not m or line.startswith("#"):
        print(line)
        continue
    sid = int(m.group(1), 16)
    label = ", ".join(sorted(set(names.get(sid, [])))[:2]) or "?"
    print(f"{line}  {label}")
