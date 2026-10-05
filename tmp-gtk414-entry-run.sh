#!/bin/sh
set -eu
sed -i 's/\r$//' /tmp/gtk414-entry.sh
chmod +x /tmp/gtk414-entry.sh
for d in entry text-entry search-entry dialog editable-cells; do
  echo "===== $d ====="
  /tmp/gtk414-entry.sh "$d" || true
done
