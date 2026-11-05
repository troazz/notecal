#!/usr/bin/env bash
# Cold start for Gate 1: launches a release build N times and reports the
# time from launch to the editor's first focus (logged by `mark_ready`).
# Usage: scripts/measure-start.sh [binary] [runs]
#   binary defaults to target/release/notecal (macOS: the .app's
#   Contents/MacOS/notecal); runs defaults to 5.
set -euo pipefail

bin=${1:-target/release/notecal}
runs=${2:-5}
log=$(mktemp)
trap 'rm -f "$log"' EXIT

now_ms() { python3 -c 'import time; print(int(time.time() * 1000))'; }

results=()
for i in $(seq 1 "$runs"); do
  : > "$log"
  start=$(now_ms)
  "$bin" 2> "$log" &
  pid=$!
  for _ in $(seq 1 100); do
    grep -q 'notecal: ready' "$log" && break
    sleep 0.1
  done
  ready=$(sed -n 's/.*epoch_ms=\([0-9]*\).*/\1/p' "$log" | head -n1)
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  if [[ -z "$ready" ]]; then
    echo "run $i: no ready line within 10 s" >&2
    continue
  fi
  results+=($((ready - start)))
  echo "run $i: $((ready - start)) ms"
  sleep 1
done

if ((${#results[@]})); then
  median=$(printf '%s\n' "${results[@]}" | sort -n | awk '{a[NR]=$1} END {print a[int((NR + 1) / 2)]}')
  echo "median of ${#results[@]} runs: $median ms"
fi
