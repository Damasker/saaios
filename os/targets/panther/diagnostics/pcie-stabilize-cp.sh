#!/bin/sh
# Transport-stability workaround for the Pixel 7 (panther) Shannon s5300 modem.
#
# After RadioPower-ON (SIT 0x0800) the modem PCIe endpoint cannot complete the
# L2/L3 power-down handshake (kernel: "cannot receive L23_READY DLLP packet",
# LTSSM stuck at Detect), and cpif can no longer ring the ap2cp doorbell
# ("PCI not powered on"). Two AP-side runtime-PM policy changes keep the link in
# L0 and recover an already-wedged link:
#   1. Disable root-complex runtime suspend  (RC power/control = on).
#   2. Disable endpoint L1.2 ASPM/PCI-PM     (EP link l1_2_aspm/l1_2_pcipm = 0).
#
# Because the owner dispatches 0x0800 ~10 s after the endpoint enumerates, a
# single early application does not stick; this re-applies both every second for
# a bounded window so it covers the 0x0800 dispatch and recovers the link.
#
# Touches only PCIe ASPM/PM policy. No EFS/NV/SIM/firmware access. Reversible.
set -u

RC=/sys/devices/platform/11920000.pcie
CTRL="$RC/power/control"
EP="$RC/pci0000:00/0000:00:00.0/0000:01:00.0/link"

# Bounded window in seconds (default 120; covers boot + 0x0800 + settle).
WINDOW="${1:-120}"

applied_ctrl=0
applied_l12=0
i=0
while [ "$i" -lt "$WINDOW" ]; do
    if [ -w "$CTRL" ]; then
        echo on > "$CTRL" 2>/dev/null && applied_ctrl=1
    fi
    if [ -e "$EP/l1_2_aspm" ]; then
        echo 0 > "$EP/l1_2_aspm" 2>/dev/null
        echo 0 > "$EP/l1_2_pcipm" 2>/dev/null
        if [ "$applied_l12" -eq 0 ]; then
            applied_l12=1
            echo "PCIE_STABILIZE first_l1_2_disable i=$i aspm=$(cat "$EP/l1_2_aspm" 2>/dev/null) pcipm=$(cat "$EP/l1_2_pcipm" 2>/dev/null) ctrl=$(cat "$CTRL" 2>/dev/null)"
        fi
    fi
    i=$((i + 1))
    sleep 1
done
echo "PCIE_STABILIZE done window=$WINDOW applied_ctrl=$applied_ctrl applied_l1_2=$applied_l12 ctrl=$(cat "$CTRL" 2>/dev/null)"
