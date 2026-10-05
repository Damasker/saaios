#!/bin/sh
# ONE race: poll 0x0200 during CardPower 4→1; DETECTED -> Radio ON. ash-safe.
set -eu
IPC=/dev/umts_ipc0
RFS=/dev/umts_rfs0
LOCK=/run/saaios-sit-status.lock

state=$(cat /sys/devices/platform/cpif/modem_state)
[ "$state" = "ONLINE" ] || { echo requires ONLINE; exit 1; }

maj=$(cut -d: -f1 /sys/class/cpif/umts_ipc0/dev)
min=$(cut -d: -f2 /sys/class/cpif/umts_ipc0/dev)
[ -c "$IPC" ] || mknod "$IPC" c "$maj" "$min"
maj=$(cut -d: -f1 /sys/class/cpif/umts_rfs0/dev)
min=$(cut -d: -f2 /sys/class/cpif/umts_rfs0/dev)
[ -c "$RFS" ] || mknod "$RFS" c "$maj" "$min"

exec 9>"$LOCK"
flock -n 9 || { echo status lock busy; exit 1; }

# best-effort rfs drain
( while true; do dd if="$RFS" of=/dev/null bs=256 count=1 2>/dev/null || sleep 0.05; done ) &
RFS_PID=$!
trap 'kill $RFS_PID 2>/dev/null; exit' INT TERM EXIT

T0=$(awk '{print int($1*1000)}' /proc/uptime)
ms() { awk -v t0="$T0" '{print int($1*1000)-t0}' /proc/uptime; }

TOK=400
SAW_ABSENT=0
SAW_DET=0
RADIO_PULSED=0
LAST_APP=x
LAST_CARD=x

# write N raw bytes from hex string (no spaces)
whex() {
  echo "$1" | busybox xxd -r -p > "$IPC"
}

le32hex() {
  t=$1
  printf '%02x%02x%02x%02x' $((t & 255)) $(((t >> 8) & 255)) $(((t >> 16) & 255)) $(((t >> 24) & 255))
}

send_sim() {
  t=$1
  whex "000000020c00$(le32hex "$t")0000"
}
send_card() {
  st=$1; t=$2
  printf 't=%s CardPower state=%s\n' "$(ms)" "$st"
  whex "00004c020d00$(le32hex "$t")0000$(printf '%02x' "$st")"
}
send_radio_on() {
  t=$1
  printf 't=%s RadioPower ON\n' "$(ms)"
  whex "000000081200$(le32hex "$t")0000020000000000"
  RADIO_PULSED=1
}
send_voice() { t=$1; whex "000000070c00$(le32hex "$t")0000"; }
send_data() { t=$1; whex "000001070c00$(le32hex "$t")0000"; }

# Parse one buffer of hex bytes (continuous) for SIT frames.
parse_hex() {
  tag=$1
  hex=$2
  while [ ${#hex} -ge 24 ]; do
    # length LE at offset 4
    l0=$(printf '%s' "$hex" | cut -c9-10)
    l1=$(printf '%s' "$hex" | cut -c11-12)
    len=$(printf '%d' "0x$l0") 
    len=$((len + $(printf '%d' "0x$l1") * 256))
    [ "$len" -ge 8 ] || return 0
    need=$((len * 2))
    [ ${#hex} -ge "$need" ] || return 0
    frame=$(printf '%s' "$hex" | cut -c1-"$need")
    hex=$(printf '%s' "$hex" | cut -c$((need + 1))-)
    id0=$(printf '%s' "$frame" | cut -c5-6)
    id1=$(printf '%s' "$frame" | cut -c7-8)
    id=$(( $(printf '%d' "0x$id0") + $(printf '%d' "0x$id1") * 256 ))
    err=$(printf '%s' "$frame" | cut -c21-22)
    if [ "$id" -eq 512 ]; then
      if [ "$err" != "00" ]; then
        printf 't=%s %s SIM err=%s len=%s\n' "$(ms)" "$tag" "$err" "$len"
        continue
      fi
      card=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c25-26)")
      apps=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c29-30)")
      app=-1; pin1=-1
      if [ "$apps" -ge 1 ] && [ "$len" -ge 18 ]; then
        app=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c35-36)")
      fi
      if [ "$apps" -ge 1 ] && [ "$len" -ge 75 ]; then
        pin1=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c145-146)")
      fi
      [ "$card" -eq 0 ] && SAW_ABSENT=1
      [ "$app" -eq 1 ] && SAW_DET=1
      if [ "$card" != "$LAST_CARD" ] || [ "$app" != "$LAST_APP" ]; then
        printf 't=%s %s card=%s apps=%s app=%s pin1=%s\n' "$(ms)" "$tag" "$card" "$apps" "$app" "$pin1"
        LAST_CARD=$card; LAST_APP=$app
      else
        printf 't=%s %s same card=%s app=%s\n' "$(ms)" "$tag" "$card" "$app"
      fi
      if [ "$SAW_DET" -eq 1 ] && [ "$RADIO_PULSED" -eq 0 ]; then
        printf 't=%s DETECTED — pulse Radio ON\n' "$(ms)"
        TOK=$((TOK + 1))
        send_radio_on "$TOK"
      fi
    elif [ "$id" -eq 588 ]; then
      printf 't=%s %s CardPowerRsp err=%s len=%s\n' "$(ms)" "$tag" "$err" "$len"
    elif [ "$id" -eq 2048 ]; then
      printf 't=%s %s RadioRsp err=%s len=%s\n' "$(ms)" "$tag" "$err" "$len"
    elif [ "$id" -eq 1792 ] || [ "$id" -eq 1793 ]; then
      # 0x0700 / 0x0701
      reg=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c25-26)")
      rej=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c27-28)")
      tech=$(printf '%d' "0x$(printf '%s' "$frame" | cut -c31-32)")
      printf 't=%s %s reg id=%s err=%s registration_raw=%s reject=%s tech=%s\n' \
        "$(ms)" "$tag" "$id" "$err" "$reg" "$rej" "$tech"
    else
      printf 't=%s %s ipc id=%s len=%s\n' "$(ms)" "$tag" "$id" "$len"
    fi
  done
}

drain_ms() {
  tag=$1; waitms=$2
  end=$(( $(ms) + waitms ))
  while [ "$(ms)" -lt "$end" ]; do
    hex=$(dd if="$IPC" bs=1024 count=1 2>/dev/null | busybox xxd -p | tr -d '\n')
    if [ -n "$hex" ]; then
      parse_hex "$tag" "$hex"
    else
      sleep 0.02
    fi
  done
}

poll_once() {
  tag=$1
  TOK=$((TOK + 1))
  send_sim "$TOK"
  drain_ms "$tag" 300
}

burst() {
  tag=$1; dur=$2
  end=$(( $(ms) + dur ))
  while [ "$(ms)" -lt "$end" ]; do
    poll_once "$tag"
  done
}

echo "RACE begin"
# need xxd
if ! busybox xxd -p </dev/null >/dev/null 2>&1; then
  echo "busybox xxd missing"; exit 1
fi

poll_once pre

TOK=$((TOK + 1)); send_card 4 "$TOK"
drain_ms after-down-cmd 800
burst down 2500

TOK=$((TOK + 1)); send_card 1 "$TOK"
drain_ms after-up-cmd 800
burst up 8000

if [ "$SAW_DET" -eq 1 ]; then
  printf 't=%s re-pulse Radio ON\n' "$(ms)"
  RADIO_PULSED=0
  TOK=$((TOK + 1)); send_radio_on "$TOK"
  drain_ms radio-rsp 1000
  burst post-radio 3000
else
  echo "never saw DETECTED(1)"
  TOK=$((TOK + 1)); send_radio_on "$TOK"
  drain_ms radio-rsp 1000
  burst post-miss 2000
fi

echo "--- final ---"
poll_once final
TOK=$((TOK + 1)); send_voice "$TOK"; drain_ms voice 2000
TOK=$((TOK + 1)); send_data "$TOK"; drain_ms data 2000

rx=$(cat /sys/class/net/rmnet0/statistics/rx_bytes 2>/dev/null || echo 0)
tx=$(cat /sys/class/net/rmnet0/statistics/tx_bytes 2>/dev/null || echo 0)
echo "rmnet0 rx=$rx tx=$tx"
ip -4 -o addr show rmnet0 2>/dev/null || echo "rmnet0 ipv4: none"
echo "RESULT saw_absent=$SAW_ABSENT saw_detected=$SAW_DET radio_pulsed=$RADIO_PULSED last_app=$LAST_APP last_card=$LAST_CARD"
