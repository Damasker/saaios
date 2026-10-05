#!/bin/bash
set -e
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/tmp-radio-oem-ipc-strings.out
: > "$OUT"
for f in \
  /mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-cp2a.260705.006-radio.img \
  /mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-td1a.221105.001-radio.img
do
  echo "== $f ==" | tee -a "$OUT"
  strings -a -n 8 "$f" | grep -E '/dev/oem_ipc|oem_ipc[0-9]|SIM_INIT_REQ|OemHook|oemhook|oem_ipc' | head -60 | tee -a "$OUT"
done
echo "== find vendor imgs ==" | tee -a "$OUT"
find /mnt/c/Users/Admin/Projects/saaios-modem-research /mnt/c/Users/Admin/Projects/saaios /mnt/c/Users/Admin/Downloads \
  -maxdepth 5 \( -iname '*vendor*.img' -o -iname '*oemhook*' -o -iname '*libril*' \) 2>/dev/null | head -40 | tee -a "$OUT"
echo DONE | tee -a "$OUT"
