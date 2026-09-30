#!/usr/bin/env bash
set -euo pipefail
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt
mkdir -p "$OUT"
ZIP="$OUT/panther-ota-td1a.221105.003-32ef0dee.zip"
LOG="$OUT/verizon-td1a-scan.log"
URL="https://dl.google.com/dl/android/aosp/panther-ota-td1a.221105.003-32ef0dee.zip"
EXPECT=2330992256

echo "start $(date)" | tee "$LOG"
sz=0
if [[ -f "$ZIP" ]]; then sz=$(stat -c%s "$ZIP"); fi
if [[ "$sz" -lt "$EXPECT" ]]; then
  echo "curl resume/download -> $ZIP (have=$sz want=$EXPECT)" | tee -a "$LOG"
  curl -L --retry 5 -C - -o "$ZIP" "$URL" 2>&1 | tee -a "$LOG"
fi
echo "size=$(stat -c%s "$ZIP")" | tee -a "$LOG"
python3 "$OUT/../tmp-scan-verizon-ota.py" 2>&1 | tee -a "$LOG"
echo "finished $(date)" | tee -a "$LOG"
