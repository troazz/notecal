#!/bin/bash
# Raycast script command: put this file in your Raycast script folder.
# Needs notecal-cli on PATH.
#
# @raycast.schemaVersion 1
# @raycast.title Calculate
# @raycast.packageName notecal
# @raycast.mode compact
# @raycast.icon 🧮
# @raycast.argument1 { "type": "text", "placeholder": "5 km in miles" }

if result=$(notecal-cli "$1" 2>&1); then
  printf '%s' "$result" | pbcopy
  echo "$result (copied)"
else
  echo "${result#notecal-cli: }"
  exit 1
fi
