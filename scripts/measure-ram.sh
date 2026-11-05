#!/usr/bin/env bash
# Idle RAM for Gate 1 (Linux): sums PSS of the notecal process and all of
# its descendants (WebKitGTK runs separate web and network processes).
# Usage: scripts/measure-ram.sh [pid]   (default: the running `notecal`)
set -euo pipefail

root=${1:-$(pgrep -x notecal | head -n1 || true)}
if [[ -z "$root" ]]; then
  echo "notecal is not running" >&2
  exit 1
fi

descendants() {
  local pid=$1 child
  echo "$pid"
  for child in $(pgrep -P "$pid"); do
    descendants "$child"
  done
}

total=0
printf '%-8s %10s  %s\n' PID PSS_KiB COMMAND
for pid in $(descendants "$root"); do
  pss=$(awk '/^Pss:/ {print $2}' "/proc/$pid/smaps_rollup" 2>/dev/null || echo 0)
  total=$((total + pss))
  printf '%-8s %10s  %s\n' "$pid" "$pss" "$(tr '\0' ' ' < "/proc/$pid/cmdline" | cut -c1-60)"
done
printf 'TOTAL    %10s KiB  (%s MiB)\n' "$total" "$((total / 1024))"
