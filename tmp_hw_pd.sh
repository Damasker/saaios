#!/system/bin/sh
echo 'pd0 source'
for f in /sys/class/usb_power_delivery/pd0/source-capabilities/1:fixed_supply/*; do
  [ -f "$f" ] || continue
  echo "  $(basename $f)=$(cat $f)"
done
echo 'pd1 sink'
for d in /sys/class/usb_power_delivery/pd1/sink-capabilities/*; do
  [ -d "$d" ] || continue
  echo "== $(basename $d)"
  for f in "$d"/*; do
    [ -f "$f" ] || continue
    echo "  $(basename $f)=$(cat $f)"
  done
done
echo 'partner'
cat /sys/class/typec/port0-partner/type 2>/dev/null
cat /sys/class/typec/port0-partner/usb_power_delivery_revision 2>/dev/null
echo 'max current pd0'
cat /sys/class/usb_power_delivery/pd0/source-capabilities/1:fixed_supply/maximum_current
