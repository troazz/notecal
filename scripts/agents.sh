#!/usr/bin/env bash
# Runs one task through coding agents in Herdr, in its own git worktree:
#
#   1. Claude implements the task (edits accepted, commands per .claude/settings.json)
#   2. scripts/check.sh runs in a separate pane
#   3. Codex reviews the diff against main and writes .agent-review.md
#   4. Claude fixes the findings, and the checks run again
#
# It never commits, merges or pushes: it stops with the branch ready for you to
# read. When an agent waits for an approval, it notifies you and resumes after you
# answer in that agent's pane.
#
# Usage (from a pane inside Herdr):
#   scripts/agents.sh <name> "<task>"
#   scripts/agents.sh typing-p95 "Do Task 7's typing-latency work from .claude/plans/m5-release-gate2.plan.md"
#
# Creates branch agent/<name> in ~/Work/notecal-agents/<name>. Remove it afterwards
# with: herdr worktree remove (or git worktree remove <path>).
set -euo pipefail

name=${1:-}
task=${2:-}
if [[ ! $name =~ ^[a-z][a-z0-9-]{0,20}$ || -z $task ]]; then
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi
[[ ${HERDR_ENV:-} == 1 ]] || { echo "Run this from a pane inside Herdr." >&2; exit 2; }
command -v jq >/dev/null || { echo "jq is required." >&2; exit 2; }

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
wt_path="$(dirname "$root")/notecal-agents/$name"
impl="impl-$name"
review="review-$name"
# Share compiled dependencies with the main checkout instead of a cold build.
target_env="CARGO_TARGET_DIR=$root/target"
long=$((3 * 60 * 60 * 1000)) # 3 h: upper bound for one agent turn or a check run

log() { printf '\n[agents] %s\n' "$*"; }
notify() { herdr notification show "notecal: $name" --body "$1" --sound request >/dev/null || true; }

# Waits until the agent is ready for input. While it waits on an approval or a
# question, asks you to answer it in the agent's pane and keeps waiting.
settle() {
  local agent=$1 status
  while true; do
    herdr agent wait "$agent" >/dev/null
    status=$(herdr agent get "$agent" | jq -r '.result.agent.agent_status')
    [[ $status == blocked ]] || return 0
    log "$agent is waiting for you: answer it in its pane."
    notify "$agent is waiting for your approval"
    herdr agent wait "$agent" --until working --until idle --until done >/dev/null
  done
}

# Starts an agent; a startup prompt (folder trust, login) is handed to you by settle.
start_agent() {
  local agent=$1 kind=$2 pane=$3
  shift 3
  herdr agent start "$agent" --kind "$kind" --pane "$pane" --timeout 60000 -- "$@" >/dev/null || true
  settle "$agent"
}

prompt() {
  local agent=$1 text=$2
  herdr agent prompt "$agent" "$text" --wait --timeout "$long" >/dev/null || true
  settle "$agent"
}

# Runs scripts/check.sh in the test pane; returns its exit status.
check() {
  local token="CHECK-$RANDOM$RANDOM" code
  herdr pane run "$test_pane" "$target_env scripts/check.sh; echo \"$token-exit=\$?\"" >/dev/null
  herdr pane wait-output "$test_pane" --regex "$token-exit=[0-9]+" --source recent-unwrapped \
    --timeout "$long" >/dev/null
  code=$(herdr pane read "$test_pane" --source recent-unwrapped --lines 40 |
    grep -o "$token-exit=[0-9]*" | tail -1 | cut -d= -f2 || true)
  code=${code:-1}
  log "checks: $([[ $code == 0 ]] && echo passed || echo "FAILED (exit $code)")"
  return "$code"
}

log "creating worktree $wt_path on branch agent/$name"
mkdir -p "$(dirname "$wt_path")"
created=$(herdr worktree create --cwd "$root" --branch "agent/$name" --path "$wt_path" \
  --label "$name" --no-focus)
test_pane=$(jq -r '.result.root_pane.pane_id' <<<"$created")
split() {
  herdr pane split "$1" --direction "$2" --cwd "$wt_path" --env "$target_env" --no-focus |
    jq -r '.result.pane.pane_id'
}
impl_pane=$(split "$test_pane" right)
review_pane=$(split "$impl_pane" down)

log "implementing ($impl, Claude)"
start_agent "$impl" claude "$impl_pane" --permission-mode acceptEdits
prompt "$impl" "Task: $task

Work in this checkout only (branch agent/$name). Follow AGENTS.md. Run scripts/check.sh until it passes. Do not commit. End with a short summary of what changed and what you verified."

check || prompt "$impl" "scripts/check.sh fails in the test pane. Run it, fix the failures, and run it again until it passes. Do not commit."

log "reviewing ($review, Codex)"
start_agent "$review" codex "$review_pane" --sandbox workspace-write --ask-for-approval never
prompt "$review" "Review the changes on this branch against main: run git diff main (it includes uncommitted work) and read AGENTS.md. Report only real defects: bugs, data loss, broken rules from AGENTS.md, changed behaviour without a test. For each give file:line, the concrete failing case, and the fix. Write the findings to .agent-review.md; if there are none, write exactly NO FINDINGS. Do not edit any other file."

if [[ -f $wt_path/.agent-review.md ]] && ! grep -qx 'NO FINDINGS' "$wt_path/.agent-review.md"; then
  log "fixing review findings"
  prompt "$impl" "A reviewer wrote findings to .agent-review.md. Fix the ones that are real; for any you reject, say why. Run scripts/check.sh until it passes. Do not commit."
  check || true
else
  log "review: $([[ -f $wt_path/.agent-review.md ]] && echo 'no findings' || echo 'no .agent-review.md written; read the reviewer pane')"
fi

notify "done; ready for your review"
cat <<EOF

[agents] Done. Nothing is committed.
  Worktree: $wt_path   (branch agent/$name, Herdr workspace "$name")
  Diff:     git -C $wt_path diff main
  Review:   $wt_path/.agent-review.md
  Agents $impl and $review are still open there for follow-up questions.
EOF
