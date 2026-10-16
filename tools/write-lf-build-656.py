from pathlib import Path

src = Path("/mnt/c/Users/Admin/Projects/saaios-pixel7-main/tools/build-saai-shell-593.sh")
dst = Path("/home/mike/saaios-pixel7-flash/build-saai-shell-656.sh")
data = src.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
dst.write_bytes(data)
dst.chmod(0o755)
print(dst, "bytes", len(data))
