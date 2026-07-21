#!/usr/bin/env bash
# Capture Logitech Flow traffic on macOS during one scenario.
# Usage: sudo ./capture-macos.sh <scenario-label> [interface]
# Produces <scenario-label>-macos-<epoch>.pcap in the current directory.
set -euo pipefail

LABEL="${1:?usage: sudo ./capture-macos.sh <scenario-label> [interface]}"
IFACE="${2:-en0}"
OUT="${LABEL}-macos-$(date +%s).pcap"

echo "Capturing Flow ports on ${IFACE} -> ${OUT}"
echo "Reproduce scenario '${LABEL}' now. Press Ctrl-C to stop."
exec tcpdump -i "${IFACE}" -w "${OUT}" \
  'port 59866 or port 59867 or port 59868'
