#!/system/bin/sh
echo '=== wireless / wpc / p9412 / p9221 ==='
getprop | grep -iE 'wireless|wpc|wlc|p9412|p9221|qi |dc_charging' | grep -viE 'serial|imei|fingerprint'
ls /sys/class | grep -iE 'power|usb|tcpc|wireless|dc_chg|charger'
ls /sys/class/power_supply 2>/dev/null
for p in /sys/class/power_supply/*; do
  echo "-- $(basename $p) type=$(cat $p/type 2>/dev/null)"
  for f in online present status voltage_now current_now capacity technology wireless_type wlc_freq adapter_type; do
    [ -e "$p/$f" ] || continue
    echo -n "  $f="
    cat "$p/$f" 2>/dev/null | tr '\n' ' '; echo
  done
done

echo '=== p9412 / p9221 sysfs ==='
ls /sys/bus/i2c/drivers | grep -iE 'p9|wpc|idt|renesas'
find /sys -iname '*p9412*' -o -iname '*p9221*' -o -iname '*wireless*' 2>/dev/null | grep -vE 'cgroup|trace' | head -40
ls /sys/devices/platform | grep -iE 'wlc|wireless|p94|dc_charger|pca9468|hl7132|ln8411'

echo '=== charger chips ==='
lsmod | grep -iE 'p9221|p9412|pca9468|max777|google_charger|google_battery|hl7132|ln8411|wireless'
ps -A | grep -iE 'charger|battery|thermal' | grep -vE 'thermal_zone|kworker' | head

echo '=== nfc already known check ==='
ls /dev | grep -iE 'nfc|st21|ese'
cat /sys/bus/i2c/devices/8-0008/name 2>/dev/null

echo '=== usb pd ==='
ls /sys/class/typec 2>/dev/null
ls /sys/class/udc 2>/dev/null
getprop sys.usb.config
getprop persist.vendor.usb.config

echo '=== leftover platform interesting ==='
ls /sys/devices/platform | grep -iE 'nfc|wlc|wireless|charger|battery|hall|lid|cap|hover|barom|light|als|p941|pca94|dock|pixel'

echo '=== dmesg wlc ==='
dmesg 2>/dev/null | grep -iE 'p9412|p9221|wireless|WPC|qi charger|dc_chg' | grep -viE 'imei|serial' | head -30
