#!/system/bin/sh
# Hardware inventory around Shannon/CPIF. No IMEI/ICCID/MAC dumps.

echo '=== identity (safe) ==='
getprop ro.product.device
getprop ro.hardware
getprop ro.boot.hardware.revision
getprop ro.boot.hardware.sku
getprop ro.boot.hardware.radio.subtype
getprop ro.revision
getprop ro.boot.hardware.color
getprop ro.soc.model
getprop ro.soc.manufacturer
getprop gsm.version.baseband
getprop persist.vendor.modem.baseband.version
getprop persist.vendor.radio.cp.hw.version
getprop ro.vendor.cbd.modem_type
getprop ro.boot.slot_suffix

echo '=== cpu / soc ==='
cat /proc/cpuinfo 2>/dev/null | grep -E 'Hardware|CPU implementer|CPU part|CPU architecture' | sort -u
getprop ro.board.platform

echo '=== kernel modules radio/pcie/sim ==='
lsmod 2>/dev/null | grep -iE 'cpif|shm_ipc|pcie|modem|gsa|spi|thermal|rmnet|gnss|gps|exynos|shannon|sit' || true

echo '=== cpif sysfs ==='
ls /sys/devices/platform/cpif 2>/dev/null | head -80
echo '-- modem_state --'
cat /sys/devices/platform/cpif/modem_state 2>/dev/null; echo
echo '-- ds_detect --'
cat /sys/devices/platform/cpif/sim/ds_detect 2>/dev/null; echo
echo '-- sim --'
ls /sys/devices/platform/cpif/sim 2>/dev/null
for f in /sys/devices/platform/cpif/sim/*; do
  [ -f "$f" ] || continue
  n=$(basename "$f")
  case "$n" in
    *imei*|*iccid*|*imsi*) continue ;;
  esac
  echo -n "$n="
  cat "$f" 2>/dev/null | tr '\n' ' '; echo
done

echo '=== pcie ==='
ls /sys/bus/pci/devices 2>/dev/null
for d in /sys/bus/pci/devices/*; do
  [ -d "$d" ] || continue
  echo "-- $d"
  cat "$d/vendor" 2>/dev/null; cat "$d/device" 2>/dev/null
  cat "$d/class" 2>/dev/null
  cat "$d/modalias" 2>/dev/null
  ls "$d/driver" 2>/dev/null
  cat "$d/current_link_speed" 2>/dev/null
  cat "$d/current_link_width" 2>/dev/null
  cat "$d/max_link_speed" 2>/dev/null
done
ls /sys/devices/platform | grep -iE 'pcie|cpif|modem|gsa|gnss|spi' || true

echo '=== pcie exynos ==='
ls /sys/devices/platform | grep -i pcie
find /sys/devices/platform -maxdepth 3 -iname '*pcie*' 2>/dev/null | head -40

echo '=== thermal cp ==='
ls /sys/devices/platform | grep -iE 'cp-tm|thermal|modem'
cat /sys/devices/platform/cp-tm1/cp_temp 2>/dev/null; echo
cat /sys/class/thermal/thermal_zone*/type 2>/dev/null | grep -iE 'cp|modem|radio|soc' || true
for z in /sys/class/thermal/thermal_zone*; do
  t=$(cat "$z/type" 2>/dev/null)
  echo "$t" | grep -qiE 'cp|modem|radio|soc|gpu|battery' || continue
  echo -n "$t temp="
  cat "$z/temp" 2>/dev/null
done

echo '=== regulators / power (names only matching radio) ==='
ls /sys/class/regulator 2>/dev/null | head
for r in /sys/class/regulator/regulator.*; do
  n=$(cat "$r/name" 2>/dev/null)
  echo "$n" | grep -qiE 'modem|cp_|mif|pll|rf|pa_|ldo.*cp|vdd_cp|vdd_mif' || continue
  echo -n "$n state="
  cat "$r/state" 2>/dev/null
  echo -n " volt="
  cat "$r/microvolts" 2>/dev/null
done

echo '=== gnss / gps ==='
ls /dev | grep -iE 'gnss|gps|umts_' | head
getprop | grep -iE 'gnss|gps' | grep -viE 'imei|serial' | head -30
ps -A | grep -iE 'gnss|gpsd|lhd|samsung_gnss' || true

echo '=== gsa / trusty / keymaster (names) ==='
ls /dev | grep -iE 'gsa|trusty|tee|citadel|weaver' | head
getprop | grep -iE 'gsa|trusty|tee' | grep -viE 'serial|imei' | head -20
ps -A | grep -iE 'gsa|trusty|teegris|qsee' || true

echo '=== aoc / audio voice ==='
ls /dev | grep -iE 'aoc|abox|snd' | head
ps -A | grep -iE 'aoc|audiohal|tinyalsa' | head
getprop | grep -iE 'aoc|voice.audio' | head

echo '=== wifi coexist ==='
getprop | grep -iE 'wifi.*sar|sar_|coex|wlan.chip' | head
ls /sys/class/net | grep -iE 'wlan|rmnet'

echo '=== eSIM / pSIM sysfs names ==='
ls /sys/class 2>/dev/null | grep -iE 'uicc|sim|ese|pmt'
find /sys -iname '*ds_detect*' 2>/dev/null | head
getprop gsm.sim.state
getprop persist.radio.multisim.config
getprop telephony.active_modems.max_count

echo '=== firmware blobs near modem ==='
ls /vendor/firmware 2>/dev/null | grep -iE 'modem|shannon|g5300|sit|gnss|gps|gsa' | head -40
ls /mnt/vendor/modem_img/images/default 2>/dev/null
ls /vendor/firmware/carrierconfig 2>/dev/null | head
getprop persist.vendor.radio.config.carrier_config_dir

echo '=== packages shannon ==='
pm list packages 2>/dev/null | grep -iE 'shannon|ims|rcs|radio' | head -40

echo '=== dmesg pcie/gsa/sim (no secrets) ==='
dmesg 2>/dev/null | grep -iE 'pcie.*s5300|s5300:|gsa|sim slot|ds_detect|cp-tm|exynos-pcie|link up|ASPM|iommu' | grep -viE 'imei|imsi|iccid' | head -50

echo '=== cpif iodevs count ==='
ls /sys/class/cpif 2>/dev/null | wc -l
ls /sys/class/cpif 2>/dev/null

echo '=== gpio / irq names radio ==='
cat /proc/interrupts 2>/dev/null | grep -iE 'cpif|pcie|modem|gsa|gnss|sim|mbox' | head -40

echo '=== dt compatible radio ==='
if [ -d /sys/firmware/devicetree/base ]; then
  find /sys/firmware/devicetree/base -name compatible 2>/dev/null | while read c; do
    v=$(tr '\0' ' ' < "$c" 2>/dev/null)
    echo "$v" | grep -qiE 'modem|cpif|shannon|gnss|gsa|pcie|s5300|s5100|exynos-cp|uicc|sim' || continue
    echo "$(dirname $c | sed 's|.*/base||'): $v"
  done | head -80
fi
