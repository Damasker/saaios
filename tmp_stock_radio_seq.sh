#!/system/bin/sh
# radio/SIT names only; strip identity
logcat -b radio -d -v time 2>/dev/null | grep -viE 'imei|imsi|iccid|msisdn|imsihash|deviceid|serial' | grep -E 'REQUEST_|RESPONSE_|UNSOL_|INDICATION|RILJ|SIT_|RADIO_POWER|SETUP_DATA|SET_ALLOWED|SET_DATA_PROFILE|ENABLE_VONR|NR_DC|NETWORK_SELECTION|OPERATOR|REGISTRATION|SIGNAL_STRENGTH|SIM_STATUS|DATA_CALL|APN|IA |ATTACH|cbd|rfsd|UDL|NV_NORM|NV_PROT|nv_normal|nv_protected' | head -400

echo '===== unique RILJ ====='
logcat -b radio -d 2>/dev/null | grep -oE 'RILJ\[[^]]*\] [A-Z0-9_]+' | sort -u | head -200

echo '===== unique REQUEST ====='
logcat -b radio -d 2>/dev/null | grep -oE 'REQUEST_[A-Z0-9_]+' | sort -u | head -200

echo '===== unique UNSOL ====='
logcat -b radio -d 2>/dev/null | grep -oE 'UNSOL_[A-Z0-9_]+' | sort -u | head -200

echo '===== cbd/rfsd logcat main ====='
logcat -b main -d -v brief 2>/dev/null | grep -E 'cbd|rfsd|CP_BOOT|UDL|NV_NORM|NV_PROT|nv_normal|nv_protected|s5100sit|modem.bin' | grep -viE 'imei|imsi|iccid' | head -200
