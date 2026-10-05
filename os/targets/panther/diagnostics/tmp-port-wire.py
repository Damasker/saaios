#!/usr/bin/env python3
"""Find wire offsets for port logical_slot / port_state in fillSlotStatusDataFromModemData."""
import subprocess
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
# continuation of modern fill around port fields after 0x626e0
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x62880","--stop-address=0x62b20", stream
], text=True, errors="replace"))
