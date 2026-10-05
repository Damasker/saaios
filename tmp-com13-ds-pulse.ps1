param([int]$WaitSeconds = 55)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo ===DS_DETECT_EXP===
echo BEFORE
echo STATE:$(cat /sys/devices/platform/cpif/modem_state)
echo -n 'ds_before='; cat /sys/devices/platform/cpif/sim/ds_detect; echo
echo -n 'mod_before='; cat /sys/module/cpif/parameters/ds_detect; echo
echo ALIVE_BEFORE
cat /data/saaios/var/tray-bearer.alive 2>/dev/null || echo NO_ALIVE
echo SIT_BEFORE
SIT=/data/saaios/bin/sit-sim-status; [ -x "$SIT" ] || SIT=/tmp/sit-sim-status
# non-blocking: may be lock-busy under tray-watch
timeout 8 $SIT query-sim-status 2>/dev/null | tr '\n' ' ' || echo 'sit_busy_or_timeout'
echo
echo CHASE_SIM_BEFORE
grep -E 'SIM snapshot|TRANSITION|EDGE|READY|ABSENT|PRESENT|batch=' /data/saaios/var/tray-bearer.log 2>/dev/null | tail -n 8

# ONE careful pulse: documented dual-SIM slot count 2->1->2.
# store() only assigns module static; mailbox ds_det written in init_control_messages only.
echo PULSE
printf '1\n' > /sys/devices/platform/cpif/sim/ds_detect
echo write1_rc=$?
echo -n 'ds_mid='; cat /sys/devices/platform/cpif/sim/ds_detect; echo
echo -n 'mod_mid='; cat /sys/module/cpif/parameters/ds_detect; echo
sleep 1
printf '2\n' > /sys/devices/platform/cpif/sim/ds_detect
echo write2_rc=$?
echo -n 'ds_after='; cat /sys/devices/platform/cpif/sim/ds_detect; echo
echo -n 'mod_after='; cat /sys/module/cpif/parameters/ds_detect; echo
echo STATE_AFTER:$(cat /sys/devices/platform/cpif/modem_state)
echo DMESG
dmesg 2>/dev/null | grep -iE 'set ds_detect|ds_det|Dual SIM' | tail -n 8 || true
sleep 3
echo SIT_AFTER
timeout 8 $SIT query-sim-status 2>/dev/null | tr '\n' ' ' || echo 'sit_busy_or_timeout'
echo
echo CHASE_SIM_AFTER
tail -n 15 /data/saaios/var/tray-bearer.log 2>/dev/null
echo RMNET
for n in 0 1 2 3; do
  [ -d /sys/class/net/rmnet$n ] || continue
  echo "rmnet$n rx=$(cat /sys/class/net/rmnet$n/statistics/rx_bytes) tx=$(cat /sys/class/net/rmnet$n/statistics/tx_bytes) ipv4=$(ip -4 addr show rmnet$n 2>/dev/null | sed -n 's/.*inet \([^ ]*\).*/\1/p' | head -1)"
done
echo ALIVE_AFTER
cat /data/saaios/var/tray-bearer.alive 2>/dev/null || echo NO_ALIVE
echo ===DS_DETECT_DONE===
'@
