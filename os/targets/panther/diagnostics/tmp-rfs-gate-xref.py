#!/usr/bin/env python3
# Read-only RE of the CP image: map the RFS protocol opcode vocabulary and
# the registration/operational-mode gate data source. Prints offsets/strings
# only (no secret payloads).
import re, sys

IMG = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(IMG, "rb").read()
print("image_len", len(data))

tokens = [
    b"RfCalDate", b"GetRfCalDate", b"RfCal", b"rfcal",
    b"OperatingMode", b"OperationMode", b"OPERATION_MODE", b"OpMode",
    b"operational", b"OperationalMode",
    b"NvRead", b"NvWrite", b"nv_read", b"nv_write",
    b"RfsRead", b"RfsWrite", b"rfs_read", b"rfs_write",
    b"IoRead", b"IoWrite", b"io_read", b"io_write",
    b"EfsRead", b"EfsWrite", b"efs_read", b"efs_write",
    b"RFS", b"rfsd", b"RfsClient", b"rfs_client",
    b"ATTACH", b"Attach", b"attach_allowed", b"ServiceState",
    b"Provision", b"provision", b"ACL", b"PLMN",
    b"REG_DENIED", b"RegDenied", b"reject_cause",
]
for t in tokens:
    idxs = [m.start() for m in re.finditer(re.escape(t), data)]
    if idxs:
        print("TOKEN", t.decode(), "count", len(idxs), "first", [hex(i) for i in idxs[:4]])
