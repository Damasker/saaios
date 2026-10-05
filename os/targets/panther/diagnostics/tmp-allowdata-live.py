#!/usr/bin/env python3
"""ONE BuildAllowData(1)=0x0710 then recheck SIM/reg/rmnet. No secrets."""
import fcntl
import os
import select
import time

st = open("/sys/devices/platform/cpif/modem_state").read().strip()
print("STATE", st)
if st != "ONLINE":
    raise SystemExit(1)
maj, mn = open("/sys/class/cpif/umts_ipc0/dev").read().strip().split(":")
maj, mn = int(maj), int(mn)
lock = os.open("/run/saaios-sit-status.lock", os.O_CREAT | os.O_RDWR | os.O_CLOEXEC, 0o600)
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
fd = os.open("/dev/umts_ipc0", os.O_RDWR | os.O_NONBLOCK | os.O_CLOEXEC)
stt = os.fstat(fd)
assert os.major(stt.st_rdev) == maj and os.minor(stt.st_rdev) == mn


def frame_size(p):
    if len(p) < 6:
        return 0
    if p[0] > 2:
        return -1
    mnlen = 8 if p[0] == 2 else 12
    ln = p[4] | (p[5] << 8)
    if ln < mnlen:
        return -1
    return 0 if len(p) < ln else ln


def exchange(req, expect_id, token, timeout=8.0):
    os.write(fd, req)
    buf = b""
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        r, _, _ = select.select([fd], [], [], 0.5)
        if not r:
            continue
        chunk = os.read(fd, 65536)
        if not chunk:
            continue
        buf += chunk
        while True:
            n = frame_size(buf)
            if n == 0:
                break
            if n < 0:
                raise SystemExit("malformed")
            fr = buf[:n]
            buf = buf[n:]
            rid = fr[2] | (fr[3] << 8)
            tok = (
                fr[6] | (fr[7] << 8) | (fr[8] << 16) | (fr[9] << 24) if n >= 10 else -1
            )
            if rid == expect_id and tok == token:
                return fr
            print(f"other id={rid:#x} len={n} tok={tok}")
    return None


def get(id_, token, lab):
    req = bytearray(12)
    req[2] = id_ & 0xFF
    req[3] = (id_ >> 8) & 0xFF
    req[4] = 12
    req[6] = token & 0xFF
    fr = exchange(bytes(req), id_, token)
    if not fr:
        print(lab, "TIMEOUT")
        return
    err = fr[10] if len(fr) > 10 else -1
    print(f"{lab} length={len(fr)} error_raw={err}")
    if id_ == 0x200 and len(fr) >= 80:
        print(
            f"  app={fr[17]} pin1={fr[72]} remain={fr[74]} apps={fr[14]} card={fr[12]}"
        )
    if id_ in (0x700, 0x701) and len(fr) >= 16 and err == 0:
        tech = fr[14] if id_ == 0x700 else fr[15]
        print(f"  reg={fr[12]} rej={fr[13]} tech={tech}")
    return fr


print("BEFORE")
get(0x200, 1, "SIM")
get(0x701, 3, "DATA")
get(0x700, 10, "VOICE")

req = bytearray(13)
req[2] = 0x10
req[3] = 0x07
req[4] = 13
req[6] = 180
req[12] = 1
print("SEND AllowData 0x0710 allow=1")
fr = exchange(bytes(req), 0x0710, 180)
if not fr:
    print("AllowData TIMEOUT")
else:
    print(f"AllowData length={len(fr)} error_raw={fr[10] if len(fr) > 10 else -1}")

time.sleep(2)
print("AFTER")
get(0x200, 2, "SIM")
get(0x701, 4, "DATA")
get(0x700, 11, "VOICE")
for i in range(3):
    rx = open(f"/sys/class/net/rmnet{i}/statistics/rx_bytes").read().strip()
    tx = open(f"/sys/class/net/rmnet{i}/statistics/tx_bytes").read().strip()
    print(f"rmnet{i}:{rx}/{tx}")
os.close(fd)
os.close(lock)
print("DONE")
