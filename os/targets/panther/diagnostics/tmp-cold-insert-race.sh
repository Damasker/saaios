#!/bin/sh
# Cold-INSERT race: dense 0x0200 from first ONLINE; Radio ON ASAP; AllowData on DETECTED.
set -eu
IPC=/dev/umts_ipc0
RFS=/dev/umts_rfs0
LOG=/data/saaios/var/cold-insert-race.log
: > "$LOG"
log() { printf '%s\n' "$*" | tee -a "$LOG"; }

maj=$(cut -d: -f1 /sys/class/cpif/umts_ipc0/dev)
min=$(cut -d: -f2 /sys/class/cpif/umts_ipc0/dev)
[ -c "$IPC" ] || mknod "$IPC" c "$maj" "$min"
maj=$(cut -d: -f1 /sys/class/cpif/umts_rfs0/dev)
min=$(cut -d: -f2 /sys/class/cpif/umts_rfs0/dev)
[ -c "$RFS" ] || mknod "$RFS" c "$maj" "$min"

( while true; do timeout 0.05 dd if="$RFS" of=/dev/null bs=256 count=1 2>/dev/null; sleep 0.02; done ) &
RP=$!
trap 'kill $RP 2>/dev/null' EXIT

TOK=1000
LAST_APP=x
LAST_CARD=x
SAW_ABSENT=0
SAW_UNKNOWN=0
SAW_DETECTED=0
SAW_PIN=0
SAW_READY=0
RADIO_ON=0
PIN_DISABLED_IDENT=0

whex(){ echo "$1" | busybox xxd -r -p > "$IPC"; }
le(){ printf '%02x%02x%02x%02x' $(($1&255)) $((($1>>8)&255)) $((($1>>16)&255)) $((($1>>24)&255)); }

send_sim(){ TOK=$((TOK+1)); whex "000000020c00$(le $TOK)0000"; }
send_radio_on(){ TOK=$((TOK+1)); whex "000000081200$(le $TOK)0000020000000000"; RADIO_ON=1; log "RadioPower ON tok=$TOK"; }
send_allow(){ TOK=$((TOK+1)); whex "000010070d00$(le $TOK)000001"; log "AllowData=1 tok=$TOK"; }
send_voice(){ TOK=$((TOK+1)); whex "000000070c00$(le $TOK)0000"; }
send_data(){ TOK=$((TOK+1)); whex "000001070c00$(le $TOK)0000"; }

name_app(){
  case "$1" in
    -1) echo n/a;;
    0) echo UNKNOWN;;
    1) echo DETECTED;;
    2) echo PIN;;
    3) echo PUK;;
    4) echo PERSO;;
    5) echo READY;;
    *) echo "app$1";;
  esac
}

parse_buf(){
  hex=$1
  while [ ${#hex} -ge 24 ]; do
    l0=$(printf %s "$hex"|cut -c9-10); l1=$(printf %s "$hex"|cut -c11-12)
    len=$((0x$l0 + 0x$l1*256))
    [ "$len" -ge 8 ] || return 0
    need=$((len*2))
    [ ${#hex} -ge "$need" ] || return 0
    frame=$(printf %s "$hex"|cut -c1-$need)
    hex=$(printf %s "$hex"|cut -c$((need+1))-)
    id=$((0x$(printf %s "$frame"|cut -c5-6) + 0x$(printf %s "$frame"|cut -c7-8)*256))
    err=$(printf %s "$frame"|cut -c21-22)
    if [ "$id" -eq 512 ] && [ "$err" = 00 ]; then
      card=$((0x$(printf %s "$frame"|cut -c25-26)))
      apps=$((0x$(printf %s "$frame"|cut -c29-30)))
      app=-1; pin1=-1
      [ "$apps" -ge 1 ] && [ "$len" -ge 18 ] && app=$((0x$(printf %s "$frame"|cut -c35-36)))
      [ "$apps" -ge 1 ] && [ "$len" -ge 75 ] && pin1=$((0x$(printf %s "$frame"|cut -c145-146)))
      [ "$card" -eq 0 ] && SAW_ABSENT=1
      [ "$app" -eq 0 ] && SAW_UNKNOWN=1
      [ "$app" -eq 1 ] && SAW_DETECTED=1
      [ "$app" -eq 2 ] && SAW_PIN=1
      [ "$app" -eq 5 ] && SAW_READY=1
      [ "$app" -eq 2 ] && [ "$pin1" -eq 3 ] && PIN_DISABLED_IDENT=1
      if [ "$card" != "$LAST_CARD" ] || [ "$app" != "$LAST_APP" ]; then
        log "SIM card=$card apps=$apps app=$app($(name_app $app)) pin1=$pin1"
        LAST_CARD=$card; LAST_APP=$app
      fi
      if [ "$RADIO_ON" -eq 0 ]; then send_radio_on; fi
      if [ "$SAW_DETECTED" -eq 1 ] || [ "$SAW_READY" -eq 1 ]; then
        send_allow
      fi
    elif [ "$id" -eq 2048 ]; then
      log "RadioRsp err=$err len=$len"
    elif [ "$id" -eq 1808 ]; then
      # 0x0710
      log "AllowDataRsp err=$err len=$len"
    elif [ "$id" -eq 1792 ] || [ "$id" -eq 1793 ]; then
      if [ "$err" = 00 ] && [ "$len" -ge 16 ]; then
        reg=$((0x$(printf %s "$frame"|cut -c25-26)))
        rej=$((0x$(printf %s "$frame"|cut -c27-28)))
        tech=$((0x$(printf %s "$frame"|cut -c31-32)))
        log "reg id=$id registration_raw=$reg reject=$rej tech=$tech"
      else
        log "reg id=$id err=$err len=$len"
      fi
    fi
  done
}

drain_once(){
  hex=$(timeout 0.15 dd if="$IPC" bs=4096 count=1 2>/dev/null | busybox xxd -p | tr -d '\n' || true)
  [ -n "$hex" ] && parse_buf "$hex"
}

# Wait ONLINE up to 120s if not yet
i=0
while [ "$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null)" != ONLINE ]; do
  i=$((i+1))
  [ "$i" -gt 600 ] && { log "TIMEOUT waiting ONLINE"; exit 1; }
  sleep 0.2
done
log "ONLINE up=$(cut -d. -f1 /proc/uptime) — race start"

# Dense poll ~25s
end=$(( $(cut -d. -f1 /proc/uptime) + 25 ))
while [ "$(cut -d. -f1 /proc/uptime)" -lt "$end" ]; do
  send_sim
  drain_once
  sleep 0.05
done

# If radio never accepted earlier, try once more
[ "$RADIO_ON" -eq 0 ] && send_radio_on
drain_once

send_voice; sleep 0.2; drain_once
send_data; sleep 0.2; drain_once
rx=$(cat /sys/class/net/rmnet0/statistics/rx_bytes 2>/dev/null || echo 0)
tx=$(cat /sys/class/net/rmnet0/statistics/tx_bytes 2>/dev/null || echo 0)
log "rmnet0 rx=$rx tx=$tx"
ip -4 -o addr show rmnet0 2>/dev/null | tee -a "$LOG" || log "rmnet0 ipv4: none"

log "RESULT saw_absent=$SAW_ABSENT saw_unknown=$SAW_UNKNOWN saw_detected=$SAW_DETECTED saw_pin=$SAW_PIN saw_ready=$SAW_READY pin_disabled_identical=$PIN_DISABLED_IDENT radio_on=$RADIO_ON last_app=$LAST_APP last_card=$LAST_CARD"
