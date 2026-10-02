#!/usr/bin/env python3
"""NV edit SAFETY HARNESS (read-only w.r.t. live NV; tested only on COPIES).

Purpose: de-risk a *hypothetical, operator-authorized* single-field NV edit by
proving a backup -> dry-run -> apply-to-copy -> diff -> restore round-trip works
and never touches live NV. THIS SCRIPT PERFORMS NO LIVE NV/EFS WRITE.

Checksum note: the CP's real NV integrity algorithm is NOT yet confirmed (the NV
store is name-keyed with a runtime-built layout; see MODEM-BLOCKER VERDICT 5).
`checksum()` here is a CRC32 PLACEHOLDER to exercise the dry-run mechanism; it is
NOT the CP's validator and MUST be replaced with the recovered algorithm before
any real write is ever considered.
"""
import binascii, hashlib, os, shutil, sys, tempfile

def sha256(b): return hashlib.sha256(b).hexdigest()
def checksum(b): return binascii.crc32(b) & 0xffffffff   # PLACEHOLDER only

def full_backup(src, dst):
    shutil.copyfile(src, dst)
    with open(dst, "rb") as f: h = sha256(f.read())
    with open(dst + ".sha256", "w") as f: f.write(h)
    return h

def narrow_backup(src, off, length, dst):
    with open(src, "rb") as f:
        f.seek(off); b = f.read(length)
    with open(dst, "wb") as f: f.write(b)
    return b

def dry_run(src, off, new_bytes):
    """Return (old_bytes, chksum_before, chksum_after_if_applied) without writing."""
    data = bytearray(open(src, "rb").read())
    old = bytes(data[off:off+len(new_bytes)])
    before = checksum(data)
    data[off:off+len(new_bytes)] = new_bytes
    after = checksum(data)
    return old, before, after

def apply_to_copy(src, off, new_bytes, dst):
    """Write happens ONLY to dst (a copy), never to src."""
    shutil.copyfile(src, dst)
    with open(dst, "r+b") as f:
        f.seek(off); f.write(new_bytes)

def restore(backup, target):
    shutil.copyfile(backup, target)
    return sha256(open(target, "rb").read())

def byte_diff(a, b):
    return [(i, a[i], b[i]) for i in range(min(len(a), len(b))) if a[i] != b[i]]

def selftest():
    d = tempfile.mkdtemp(prefix="nv-harness-")
    try:
        # synthetic "quarantine" blob
        orig = os.urandom(4096); orig = orig[:100] + b"\x01" + orig[101:]
        src = os.path.join(d, "blob.bin")
        open(src, "wb").write(orig)
        orig_sha = sha256(orig)

        # (a) full backup + hash
        bsha = full_backup(src, os.path.join(d, "blob.full.bak"))
        assert bsha == orig_sha, "full backup hash mismatch"

        # (b) narrow backup of target byte (offset 100, 1 byte)
        nb = narrow_backup(src, 100, 1, os.path.join(d, "blob.byte.bak"))
        assert nb == b"\x01", "narrow backup content wrong"

        # (c) dry-run old->new WITHOUT writing src
        old, cb, ca = dry_run(src, 100, b"\x00")
        assert old == b"\x01", "dry-run old wrong"
        assert open(src, "rb").read() == orig, "dry-run must not modify src"
        assert cb != ca, "checksum should change for a changed byte"

        # (d) apply ONLY to a copy; prove src untouched; diff is exactly 1 byte
        cp = os.path.join(d, "blob.candidate.bin")
        apply_to_copy(src, 100, b"\x00", cp)
        assert open(src, "rb").read() == orig, "apply must not touch src"
        diffs = byte_diff(orig, open(cp, "rb").read())
        assert diffs == [(100, 1, 0)], f"unexpected diff {diffs}"

        # (e) restore the copy from full backup; hash must match original
        rsha = restore(os.path.join(d, "blob.full.bak"), cp)
        assert rsha == orig_sha, "restore round-trip hash mismatch"

        print("PASS nv-edit harness self-test (backup/narrow/dry-run/apply-copy/diff/restore)")
        return 0
    finally:
        shutil.rmtree(d, ignore_errors=True)

def main(argv):
    if len(argv) >= 2 and argv[1] == "selftest":
        return selftest()
    if len(argv) >= 2 and argv[1] == "inspect" and len(argv) >= 3:
        src = argv[2]
        data = open(src, "rb").read()
        print("file", src, "size", len(data), "sha256", sha256(data),
              "crc32", hex(checksum(data)))
        return 0
    if len(argv) >= 5 and argv[1] == "dryrun":
        # dryrun <src> <offset> <hexbytes>  -> old->new + checksum, NO write
        src, off, hexb = argv[2], int(argv[3], 0), bytes.fromhex(argv[4])
        old, cb, ca = dry_run(src, off, hexb)
        print(f"DRYRUN off={hex(off)} width={len(hexb)} old={old.hex()} new={hexb.hex()} "
              f"checksum_before={hex(cb)} checksum_after={hex(ca)} (NO WRITE PERFORMED)")
        return 0
    print("usage: selftest | inspect <file> | dryrun <file> <off> <hexbytes>")
    return 2

if __name__ == "__main__":
    sys.exit(main(sys.argv))
