from pathlib import Path

d = Path("/tmp/gtk4-probe/bin")
for p in sorted(d.iterdir()):
    print("bin", repr(p.name), p.stat().st_size)
src = next(p for p in d.iterdir() if p.name.startswith("gtk414-popo"))
dst = d / "gtk414-popover"
if src != dst:
    if dst.exists():
        dst.unlink()
    src.replace(dst)
print("dst", repr(dst.name), dst.stat().st_size)

sh = Path("/tmp/rebuild-gtk414-popover.sh")
print("sh exists", sh.is_file())
if sh.is_file():
    text = sh.read_bytes()
    print("sh cr", b"\r" in text, "len", len(text))
    for line in text.splitlines():
        if b"OUT=" in line or b"gtk414" in line:
            print("line", repr(line))
