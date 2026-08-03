#!/usr/bin/env bash
# Create A/B slot layout for SaaiOS appliance updates.
# Default root: /var/lib/saaios/ab
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"

mkdir -p "$AB_ROOT"/{A,B,shared}
mkdir -p "$AB_ROOT"/A/{bin,state}
mkdir -p "$AB_ROOT"/B/{bin,state}
mkdir -p "$AB_ROOT"/shared/{audit,logs}

if [[ ! -e "$AB_ROOT/current" ]]; then
  ln -sfn A "$AB_ROOT/current"
fi

if [[ ! -f "$AB_ROOT/current_slot" ]]; then
  echo A >"$AB_ROOT/current_slot"
fi

# Clear BOOT_OK markers on fresh layout (do not wipe existing markers).
touch "$AB_ROOT/A/.keep" "$AB_ROOT/B/.keep"

cat <<EOF
A/B layout ready at $AB_ROOT

  $AB_ROOT/A/bin/     slot A binaries
  $AB_ROOT/B/bin/     slot B binaries
  $AB_ROOT/current -> active slot (symlink)
  $AB_ROOT/shared/    audit + logs shared across slots

Next:
  ./deploy/ab/install-slot.sh <slot> [binary]
  ./deploy/ab/switch.sh <slot>
  ./deploy/ab/boot-ok.sh
EOF
