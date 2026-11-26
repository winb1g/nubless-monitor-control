#!/usr/bin/env bash
# Read-only DDC/CI scan from any host (source) address, for Linux.
#
# Sends a standard "Get VCP Feature" request for every code 0x00-0xFF (except
# the factory and calibration codes) and prints the raw reply bytes. LG's
# service channel uses host address 0x50 instead of the usual 0x51; ddcutil
# builds without --i2c-source-addr can't reach it, so this does it directly.
#
# Only read requests are sent; nothing is set. Needs i2c-tools and i2c-dev:
#   sudo apt install -y i2c-tools && sudo modprobe i2c-dev
# Find the bus number with `sudo ddcutil detect` (I2C bus: /dev/i2c-N).
#
# Usage:  sudo ./vcp-scan-source.sh <bus> [source, default 0x50] > off.txt
# Then change ONE setting in the monitor's menu, scan again, and diff the files.
set -uo pipefail

BUS="${1:?usage: sudo $0 <i2c-bus-number> [source-address, default 0x50]}"
SRC=$(( ${2:-0x50} ))
SKIP=" 04 05 06 08 0a e4 e5 e6 e7 e8 e9 ea eb "

command -v i2ctransfer >/dev/null || { echo "i2ctransfer not found: sudo apt install -y i2c-tools" >&2; exit 1; }

for c in $(seq 0 255); do
  h=$(printf '%02x' "$c")
  [[ "$SKIP" == *" $h "* ]] && continue
  # DDC/CI checksum: XOR of the display's address (0x6E) and every byte sent.
  chk=$(( 0x6E ^ SRC ^ 0x82 ^ 0x01 ^ c ))
  if ! i2ctransfer -y "$BUS" w5@0x37 "$(printf '0x%02x' "$SRC")" 0x82 0x01 "0x$h" "$(printf '0x%02x' "$chk")" 2>/dev/null; then
    echo "$h no-answer"
    continue
  fi
  sleep 0.05
  if reply=$(i2ctransfer -y "$BUS" r11@0x37 2>/dev/null); then
    echo "$h $reply"
  else
    echo "$h no-reply"
  fi
  sleep 0.05
done
