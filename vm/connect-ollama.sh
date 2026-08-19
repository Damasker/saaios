#!/usr/bin/env bash
set -euo pipefail
IP=${1:-$(cat "$(dirname "$0")/ollama-ip.txt")}
KEY=${SAAIOS_VM_KEY:-$HOME/.ssh/ruta_cloud}
exec ssh -i "$KEY" -o StrictHostKeyChecking=no mike@$IP
