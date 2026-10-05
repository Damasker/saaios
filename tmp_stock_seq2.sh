#!/system/bin/sh
echo '=== unique RILJ verbs ==='
logcat -b radio -d 2>/dev/null | grep 'RILJ' | grep -oE '\[UNSL\]< [A-Z0-9_]+|> [A-Z0-9_]+|< [A-Z0-9_]+' | sed 's/^[^ ]* //;s/^> //;s/^< //' | sort -u | head -250

echo '=== key RIL lines ==='
logcat -b radio -d 2>/dev/null | grep -E 'RADIO_POWER|SETUP_DATA_CALL|SET_DATA_PROFILE|SET_INITIAL_ATTACH|SET_ALLOWED_NETWORK|ALLOW_DATA|ENABLE_VONR|SET_NR_DUAL|IS_NR_DUAL|SET_UNSOLICITED|GET_SIM_STATUS|SIM_STATUS|REQUEST_RADIO|DATA_CALL' | grep -viE 'imei|imsi|iccid|address:' | head -120

echo '=== cbd log files ==='
ls -l /data/vendor/log/cbd /data/vendor/log/rfsd /data/vendor/radio 2>/dev/null
echo '--- cbd ---'
cat /data/vendor/log/cbd/* 2>/dev/null | grep -viE 'imei|imsi|iccid' | head -120
echo '--- rfsd ---'
cat /data/vendor/log/rfsd/* 2>/dev/null | grep -viE 'imei|imsi|iccid' | head -120

echo '=== logcat cbd tag ==='
logcat -b all -d -s CBD:D cbd:D CBD:I RFSD:D RFSD:I RILC:I 2>/dev/null | grep -viE 'imei|imsi|iccid' | head -150

echo '=== cbd fds ==='
ls -l /proc/1139/fd 2>/dev/null | grep umts || true
ls -l /proc/1155/fd 2>/dev/null | grep umts || true
ls -l /proc/1158/fd 2>/dev/null | grep umts || true

echo '=== rfsd maps nv ==='
ls -l /proc/1155/cwd /proc/1155/root 2>/dev/null
tr '\0' '\n' < /proc/1155/environ | grep -iE 'nv|efs|rfs' || true
