#!/usr/bin/env bash
# Connect to the SaaiOS appliance VM
set -euo pipefail
IP=${1:-$(cat "$(dirname "$0")/ip.txt")}
KEY=${SAAIOS_VM_KEY:-/$HOME/.ssh/ruta_cloud}
exec ssh -i "$KEY" -o StrictHostKeyChecking=no mike@$IP
