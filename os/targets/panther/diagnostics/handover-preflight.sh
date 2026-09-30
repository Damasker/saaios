#!/bin/sh
# Source availability only. Does not construct/send handover or print identities.
set -eu
if [ "$#" -ne 1 ] || [ "$1" != "check-sources" ]; then
    echo 'Usage: sh handover-preflight.sh check-sources' >&2
    exit 64
fi
missing=0
for p in \
    /sys/firmware/devicetree/base/chosen/config/imei1 \
    /sys/firmware/devicetree/base/chosen/config/imei2 \
    /sys/firmware/devicetree/base/chosen/plat/rfid \
    /sys/firmware/devicetree/base/chosen/plat/hwinfo \
    /sys/firmware/devicetree/base/chosen/config/modem_flag \
    /mnt/vendor/persist/modem/cpsha \
    /sys/devices/platform/cpif/of_node/ap2cp_handover_block_info; do
    if [ -f "$p" ] && [ -r "$p" ]; then
        printf '%s: readable, bytes=' "$p"
        wc -c < "$p"
    else
        printf '%s: unavailable\n' "$p"
        missing=1
    fi
done
if [ -r /proc/bootconfig ] && grep -q '^androidboot.cdt_hwid[[:space:]]*=' /proc/bootconfig; then
    echo 'boot CDT key: present (value withheld, format not validated)'
else
    echo 'boot CDT key: unavailable'
    missing=1
fi
echo 'Availability is not ABI validation or authorization to send a handover block.'
exit "$missing"
