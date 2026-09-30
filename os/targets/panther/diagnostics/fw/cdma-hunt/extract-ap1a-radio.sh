#!/usr/bin/env bash
set -euo pipefail
HUNT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt
PDG=/tmp/payload-dumper-go
ZIP="$HUNT/panther-ota-ap1a.240505.005.a1-83fca43d.zip"
OUT="$HUNT/ap1a-extract"
LOG="$HUNT/ap1a-extract.log"
exec > >(tee "$LOG") 2>&1

rm -rf "$OUT"
mkdir -p "$OUT"
rm -f "$HUNT"/STAGED-panther-ota-ap1a.240505.005.a1-83fca43d-*.img

echo "=== extract payload.bin ==="
python3 - <<'PY'
import zipfile
from pathlib import Path
zpath = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/panther-ota-ap1a.240505.005.a1-83fca43d.zip")
out = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/ap1a-extract/payload.bin")
with zipfile.ZipFile(zpath) as z:
    print("members", z.namelist())
    with z.open("payload.bin") as src, out.open("wb") as dst:
        while True:
            c = src.read(8 * 1024 * 1024)
            if not c:
                break
            dst.write(c)
print("payload", out.stat().st_size)
PY

echo "=== list partitions ==="
"$PDG" -l "$OUT/payload.bin"

echo "=== dump radio ==="
"$PDG" -p radio -o "$OUT" "$OUT/payload.bin" || true
ls -la "$OUT"

if [[ ! -f "$OUT/radio.img" ]]; then
  echo "=== dump modem ==="
  "$PDG" -p modem -o "$OUT" "$OUT/payload.bin" || true
fi
ls -la "$OUT"

echo "=== scan extracted ==="
python3 - <<'PY'
from pathlib import Path
import hashlib
out = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/ap1a-extract")
hunt = out.parent
for p in sorted(out.glob("*.img")):
    data = p.read_bytes()
    h = hashlib.sha256(data).hexdigest()
    no = data.count(b"No CDMA in SupportedRatMap")
    en = data.count(b"EnableCdmaRat")
    gq = data.count(b"g5300q-")
    gg = data.count(b"g5300g-")
    fc = data.count(b"Found CDMA")
    i = data.find(b"g5300q-") if gq else data.find(b"g5300g-")
    ver = data[i : i + 56].split(b"\0", 1)[0] if i >= 0 else b""
    print(f"{p.name}: size={len(data)} sha={h} g5300q={gq} g5300g={gg} no_cdma={no} enable={en} found_cdma={fc} ver={ver!r}")
    interesting = gq > 0 and no == 0
    # Full radio should be ~80MB+
    if interesting and len(data) > 50_000_000:
        staged = hunt / f"STAGED-ap1a-verizon-{h[:12]}.img"
        staged.write_bytes(data)
        print("STAGED", staged)
    elif gq or gg:
        ext = hunt / f"EXTRACT-ap1a-verizon-{h[:12]}.img"
        ext.write_bytes(data)
        print("EXTRACTED", ext, "interesting=", interesting, "size_ok=", len(data) > 50_000_000)
        if interesting and len(data) <= 50_000_000:
            print("NOT staging: too small for full radio")
PY

# free payload.bin space
rm -f "$OUT/payload.bin"
echo DONE
