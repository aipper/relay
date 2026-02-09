#!/usr/bin/env bash
set -euo pipefail

# iFlow unattended watchdog for AI Workspace (Arch Linux/systemd friendly).
#
# Goal:
# - Run /server-drain in a loop (low output, small batches).
# - If drain reports BLOCKED (exit 3) or crashes, run /server-fix in a fresh iFlow process (bounded).
# - Keep outputs in .agentdocs/tmp/server-test/ to avoid blowing up interactive sessions.
#
# Safety:
# - Intended for test environments only.
# - Requires explicit IFLOW_WATCHDOG_YOLO=1 to run with --yolo.

die() { echo "error: $*" >&2; exit 2; }
need_file() { [[ -f "$1" ]] || die "missing: $1"; }
have() { command -v "$1" >/dev/null 2>&1; }

root="${IFLOW_WATCHDOG_ROOT:-.}"
cd "$root"

need_file "AI_WORKSPACE.md"
need_file "REQUIREMENTS.md"
need_file "tools/server_test_runner.py"
need_file "secrets/test-accounts.json"

have iflow || die "missing: iflow"
have uv || die "missing: uv (runner requires uv)"

csv="issues/server-api-issues.csv"
workspace_dir="$(pwd)"
log_dir="${IFLOW_WATCHDOG_LOG_DIR:-$workspace_dir/.agentdocs/tmp/server-test}"
mkdir -p "$log_dir"

fatal_log="$log_dir/watchdog-fatal.log"
timestamp() { date +"%Y-%m-%dT%H:%M:%S%z"; }
say() { printf "[%s] %s\n" "$(timestamp)" "$*"; }
fatal() { say "fatal: $*" | tee -a "$fatal_log" >&2; exit 2; }

yolo="${IFLOW_WATCHDOG_YOLO:-0}"
if [[ "$yolo" != "1" ]]; then
  fatal "unattended mode requires IFLOW_WATCHDOG_YOLO=1 (this will use iflow --yolo; test env only)"
fi

if ! iflow --version >/dev/null 2>&1; then
  fatal "iflow command failed (check auth/config/environment)"
fi

date_suffix="$(date +%Y%m%d)"
drain_log="${IFLOW_WATCHDOG_DRAIN_LOG:-$log_dir/iflow-drain-$date_suffix.log}"
fix_log="${IFLOW_WATCHDOG_FIX_LOG:-$log_dir/iflow-fix-$date_suffix.log}"
find "$log_dir" -maxdepth 1 -type f -name "iflow-*.log" -mtime +7 -delete 2>/dev/null || true

lock_dir="$log_dir/.watchdog.lock"
pid_file="$lock_dir/pid"
if ! mkdir "$lock_dir" 2>/dev/null; then
  other_pid="$(cat "$pid_file" 2>/dev/null || true)"
  fatal "another watchdog is running (lock: $lock_dir pid=${other_pid:-unknown})"
fi
printf "%s\n" "$$" >"$pid_file" 2>/dev/null || true

cleanup() {
  say "cleanup: stopping child iflow processes (if any)" >>"$fatal_log" 2>/dev/null || true
  pkill -P $$ iflow 2>/dev/null || true
  rm -f "$pid_file" 2>/dev/null || true
  rmdir "$lock_dir" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

drain_timeout="${IFLOW_WATCHDOG_DRAIN_TIMEOUT:-7200}"
fix_timeout="${IFLOW_WATCHDOG_FIX_TIMEOUT:-3600}"
drain_max_turns="${IFLOW_WATCHDOG_DRAIN_MAX_TURNS:-6}"
fix_max_turns="${IFLOW_WATCHDOG_FIX_MAX_TURNS:-12}"
drain_max_tokens="${IFLOW_WATCHDOG_DRAIN_MAX_TOKENS:-15000}"
fix_max_tokens="${IFLOW_WATCHDOG_FIX_MAX_TOKENS:-30000}"

max_cycles="${IFLOW_WATCHDOG_MAX_CYCLES:-200}"
sleep_s="${IFLOW_WATCHDOG_SLEEP_S:-10}"
fix_retry_max="${IFLOW_WATCHDOG_FIX_RETRY_MAX:-3}"
fix_retry_file="$log_dir/.fix-retry-count"

drain_max_endpoints="${IFLOW_DRAIN_MAX_ENDPOINTS:-30}"
drain_sleep_s="${IFLOW_DRAIN_SLEEP_S:-0}"

csv_has_blocked() { [[ -f "$csv" ]] && grep -qE ",BLOCKED," "$csv"; }

run_drain() {
  say "run: /server-drain (max_endpoints=$drain_max_endpoints)"
  IFLOW_DRAIN_MAX_ENDPOINTS="$drain_max_endpoints" \
  IFLOW_DRAIN_SLEEP_S="$drain_sleep_s" \
  iflow --yolo -p "/server-drain" --timeout "$drain_timeout" --max-turns "$drain_max_turns" --max-tokens "$drain_max_tokens" >>"$drain_log" 2>&1
}

run_fix() {
  say "run: /server-fix"
  retry_count="$(cat "$fix_retry_file" 2>/dev/null || echo 0)"
  if [[ "$retry_count" -ge "$fix_retry_max" ]]; then
    fatal "fix reached retry limit ($fix_retry_max); check $fix_log"
  fi
  if iflow --yolo -p "/server-fix" --timeout "$fix_timeout" --max-turns "$fix_max_turns" --max-tokens "$fix_max_tokens" >>"$fix_log" 2>&1; then
    rm -f "$fix_retry_file" 2>/dev/null || true
  else
    echo $((retry_count + 1)) >"$fix_retry_file" 2>/dev/null || true
  fi
}

say "watchdog: start (cycles=$max_cycles sleep_s=$sleep_s)"
say "logs: drain=$drain_log fix=$fix_log"

cycle=1
while [[ "$cycle" -le "$max_cycles" ]]; do
  if csv_has_blocked; then
    say "state: BLOCKED detected in $csv (trigger fix)"
    run_fix
  fi

set +e
run_drain
rc=$?
set -e

if [[ "$rc" == "0" ]]; then
  say "done: drain finished with all DONE/SKIP"
  exit 0
fi

if [[ "$rc" == "3" ]] || csv_has_blocked; then
  say "blocked: drain reported BLOCKED (rc=$rc), trigger fix"
  run_fix
else
  say "warn: drain exited rc=$rc without BLOCKED; will retry"
fi

sleep "$sleep_s" || true
cycle=$((cycle + 1))
done

fatal "reached max cycles ($max_cycles); check logs under $log_dir"
