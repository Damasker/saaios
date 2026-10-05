#!/system/bin/sh
echo '=== all i2c names ==='
for s in /sys/bus/i2c/devices/*; do
  [ -d "$s/name" ] || [ -f "$s/name" ] || continue
  n=$(cat "$s/name" 2>/dev/null)
  drv=$(basename $(readlink "$s/driver" 2>/dev/null) 2>/dev/null)
  echo "$(basename $s) name=$n driver=$drv"
done

echo '=== lwis sensor names ==='
for d in /dev/lwis-sensor-*; do echo $d; done
ls -l /sys/class/lwis 2>/dev/null | head
for n in /sys/devices/platform/sensor@*; do
  echo "-- $n"
  ls "$n" 2>/dev/null | head
  cat "$n/of_node/name" 2>/dev/null
  tr '\0' ' ' < "$n/of_node/compatible" 2>/dev/null; echo
done

echo '=== vl53 ==='
ls /sys/bus/i2c/drivers 2>/dev/null | grep -iE 'vl53|stm'
cat /sys/class/input/event3/device/name
ls /sys/bus/i2c/devices/*/name | while read f; do
  n=$(cat "$f")
  echo "$n" | grep -qi vl53 || continue
  echo "$f = $n"
done

echo '=== chre / usf sensors ==='
ls /dev | grep -iE 'chre|usf|nanohub|sensorhub'
getprop | grep -iE 'chre|usf|sensor' | grep -viE 'fingerprint|serial|imei' | head -25
ps -A | grep -iE 'chre|usf|sensors@|multihal' | head

echo '=== dumpsys sensors truncated ==='
dumpsys sensorservice 2>/dev/null | grep -E 'android.sensor.|0x[0-9a-f]+ \|' | head -60

echo '=== goodix sysfs ==='
ls /sys/class/goodix_fp 2>/dev/null
ls /sys/devices | grep -i goodix
find /sys -name '*goodix*' 2>/dev/null | head -20
ls /dev/goodix_fp

echo '=== face ==='
getprop | grep -iE 'face.|ultrasonic|soli' | grep -viE 'fingerprint|serial' | head

echo '=== v4l names ==='
for v in /sys/class/video4linux/video*; do
  echo "$(basename $v) name=$(cat $v/name 2>/dev/null)"
done
