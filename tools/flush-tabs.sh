#!/bin/sh
n=0
while [ "$n" -lt 20 ]; do
  /data/saaios/incoming/panther-tap 135 2250
  /data/saaios/incoming/panther-tap 405 2250
  n=$((n + 1))
done
echo flush_tabs_done
