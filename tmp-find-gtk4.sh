#!/bin/sh
set -eu
echo === gtk4 packages ===
find /tmp /home/mike /data -maxdepth 5 \( -name 'gtk4-demo' -o -name 'org.saaios.test.gtk4*' -o -name 'gtk4-lib-probe' \) 2>/dev/null | head
ls -d /tmp/saaios-b2/dist/panther/packages/* 2>/dev/null || true
ls -d /home/mike/projects/saaios/dist/panther/packages/* 2>/dev/null || true
