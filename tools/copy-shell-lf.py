from pathlib import Path

src = Path("/mnt/c/Users/Admin/Projects/saaios-pixel7-main/services/saai-shell/src/main.rs")
dst = Path("/home/mike/saaios-pixel7-flash/services/saai-shell/src/main.rs")
data = src.read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
dst.write_bytes(data)
print(f"wrote {len(data)} bytes to {dst}")
