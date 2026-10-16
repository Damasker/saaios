from pathlib import Path

src = Path("/mnt/c/Users/Admin/Projects/saaios-pixel7-main/tools/panther-tap.c")
dst = Path("/tmp/panther-tap.c")
dst.write_bytes(src.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n"))
print(dst, "bytes", dst.stat().st_size)
