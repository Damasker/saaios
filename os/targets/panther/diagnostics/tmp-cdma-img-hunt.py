import os, sys, hashlib, zipfile, struct
from pathlib import Path

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"No CDMA in InitRapMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"CDMA in InitRapMap",
    b"SupportedRatMap(0x%X)",
    b"TCS_CDMA_SUPPORT",
    b"DS_TCS_GV_CDMA_SUPPORT",
]

def scan(path, label=None):
    p = Path(path)
    if not p.exists():
        print(f"MISSING {label or path}")
        return
    data = p.read_bytes()
    h = hashlib.sha256(data).hexdigest()
    print(f"\n=== {label or p.name} ===")
    print(f"path={p}")
    print(f"size={len(data)} sha256={h}")
    # version-ish strings
    for s in (b"g5300q-", b"B-15", b"A-14", b"B-14", b"A-15"):
        i = data.find(s)
        if i >= 0:
            frag = data[i:i+48].split(b"\0",1)[0]
            try:
                print(f"verish@{i}: {frag.decode('ascii','replace')}")
            except Exception:
                pass
    for n in NEEDLES:
        c = data.count(n)
        print(f"  count[{n.decode('ascii','replace')}]={c}")

bases = [
    r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-a-modem.bin",
    r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin",
    r"C:\Users\Admin\Projects\saaios-som\fw-saaios-probe-b-modem-PATCHED-ready.bin",
]
for b in bases:
    scan(b)

# Prefer already-extracted modem.img candidates
cands = []
for root in [
    r"C:\Users\Admin\Desktop\Mike",
    r"C:\Users\Admin\Projects\saaios-s01",
    r"C:\Users\Admin\Projects\saaios-modem-research",
    r"C:\Users\Admin\Projects\saaios-som",
    r"C:\Users\Admin\Downloads",
]:
    if not os.path.isdir(root):
        continue
    for dirpath, dirnames, filenames in os.walk(root):
        # prune huge/irrelevant
        depth = dirpath[len(root):].count(os.sep)
        if depth > 5:
            dirnames[:] = []
            continue
        for fn in filenames:
            if fn.lower() in ("modem.img", "radio.img") or fn.startswith("radio-panther") or fn.startswith("radio-cheetah") or fn.startswith("radio-lynx"):
                cands.append(os.path.join(dirpath, fn))
print("\nCANDIDATE IMAGES:")
for c in cands:
    print(" ", c, os.path.getsize(c))

zip_path = r"C:\Users\Admin\Desktop\Mike\panther-cp2a.260705.006-factory-ed94a24e.zip"
out_dir = Path(r"C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics\fw")
out_dir.mkdir(parents=True, exist_ok=True)
modem_out = out_dir / "factory-cp2a.260705.006-modem.img"
if not modem_out.exists() and os.path.isfile(zip_path):
    print(f"\nExtracting modem.img from {zip_path} ...")
    with zipfile.ZipFile(zip_path, "r") as z:
        names = z.namelist()
        # outer factory zip contains image-*.zip
        inner = [n for n in names if n.endswith(".zip") and "image-" in n.lower()]
        print("inner:", inner[:5])
        if inner:
            import io, tempfile
            with z.open(inner[0]) as f:
                data = f.read()
            with zipfile.ZipFile(io.BytesIO(data)) as z2:
                mnames = [n for n in z2.namelist() if n.endswith("modem.img") or n.endswith("/modem.img") or n=="modem.img" or n.endswith("radio.img")]
                print("modem names:", mnames)
                if mnames:
                    with z2.open(mnames[0]) as mf:
                        modem_out.write_bytes(mf.read())
                    print("wrote", modem_out, modem_out.stat().st_size)
        else:
            mnames = [n for n in names if n.endswith("modem.img")]
            print("direct modem:", mnames)
            if mnames:
                with z.open(mnames[0]) as mf:
                    modem_out.write_bytes(mf.read())
elif modem_out.exists():
    print("already have", modem_out)

if modem_out.exists():
    scan(str(modem_out), "factory-cp2a modem.img")
