from pathlib import Path

src = Path("/mnt/c/Users/Admin/Projects/saaios-pixel7-main/tools/flush-tabs.sh")
dst = Path("/tmp/flush-tabs.sh")
dst.write_bytes(src.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n"))
dst.chmod(0o755)
print(dst, "bytes", dst.stat().st_size)
