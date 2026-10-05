#!/system/bin/sh
# Cameras, fingerprint, proximity, IMU, etc. No serials/MACs.

echo '=== fingerprint ==='
ls /sys/class | grep -iE 'fp|fpc|fingerprint|goodix|qbt|uds'
ls /dev | grep -iE 'fp|goodix|qbt|fpc|fps'
getprop | grep -iE 'fingerprint|fod|udfps|goodix|fps_d' | grep -viE 'serial|imei'
ps -A | grep -iE 'fingerprint|goodix|fps_hal|android.hardware.biometrics'
ls /sys/bus/spi/devices 2>/dev/null
for s in /sys/bus/spi/devices/*; do
  echo "--spi $s"
  cat "$s/modalias" 2>/dev/null
  ls -l "$s/driver" 2>/dev/null
done
ls /sys/bus/i2c/devices 2>/dev/null | head -40
for s in /sys/bus/i2c/devices/*; do
  n=$(cat "$s/name" 2>/dev/null)
  echo "$n" | grep -qiE 'fp|goodix|fpc|qbt|touch|prox|light|tmd|vcnl|ams|akm|bmi|lsm|icm|baro|press|mag|accel|gyro|hall|cap|hover|als' || continue
  echo "i2c $(basename $s) name=$n driver=$(basename $(readlink $s/driver 2>/dev/null) 2>/dev/null)"
done

echo '=== input devices ==='
for d in /sys/class/input/event*; do
  n=$(cat "$d/device/name" 2>/dev/null)
  echo "$(basename $d) $n"
done

echo '=== iio ==='
ls /sys/bus/iio/devices 2>/dev/null
for d in /sys/bus/iio/devices/iio:device*; do
  [ -d "$d" ] || continue
  echo "-- $(basename $d) name=$(cat $d/name 2>/dev/null)"
  ls "$d" | grep -E 'in_|name|sampling' | head -20
done

echo '=== camera ==='
ls /dev | grep -iE 'video|media|capture|cam'
ls /sys/class/video4linux 2>/dev/null
ls /sys/class/media 2>/dev/null
getprop | grep -iE 'camera|gcam|libcamera|cam.hal' | grep -viE 'serial|imei' | head -40
ps -A | grep -iE 'provider@|camera.provider|google.camera|libcamera|gcam' | head
ls /vendor/lib64/hw 2>/dev/null | grep -iE 'camera|fingerprint|sensors'
ls /vendor/firmware 2>/dev/null | grep -iE 'cam|fp|goodix|imx|ov|gc0|s5k' | head -40

echo '=== camera modules lsmod ==='
lsmod | grep -iE 'cam|csis|mipi|gxp|lwis|bigwave|smfc|jpeg|isps|gs_camera|dw980|actuator'

echo '=== lwis / gxp ==='
ls /dev | grep -iE 'lwis|gxp|dsp'
ls /sys/class | grep -iE 'lwis|gxp|camera'
find /sys/devices/platform -maxdepth 2 -iname '*lwis*' -o -iname '*gxp*' -o -iname '*cam*' 2>/dev/null | head -40

echo '=== proximity / light / cap ==='
ls /sys/bus/iio/devices/*/name 2>/dev/null
for d in /sys/class/leds/*; do echo LED $(basename $d); done | head -40
getprop | grep -iE 'als|prox|light.sensor|cap.sensor' | head

echo '=== packages sensors ==='
pm list packages 2>/dev/null | grep -iE 'camera|fingerprint|neural|face|aware' | head -30

echo '=== dt compatible sensors/cams ==='
if [ -d /sys/firmware/devicetree/base ]; then
  find /sys/firmware/devicetree/base -name compatible 2>/dev/null | while read c; do
    v=$(tr '\0' ' ' < "$c" 2>/dev/null)
    echo "$v" | grep -qiE 'imx|ov[0-9]|s5k|gc0|camera|csis|mipi-csi|goodix|fpc|qbt|fingerprint|prox|tmd|vcnl|ams,|ak[0-9]|bmi|lsm6|icm|bmp|baro|press|mag|accel|gyro|hall|cap-sense|hover|als|light-sensor|gxp|lwis|dw980|actuator|eeprom' || continue
    echo "$(echo $c | sed 's|.*/base||;s|/compatible||'): $v"
  done
fi
