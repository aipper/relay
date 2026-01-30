#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Linux one-click refresh (dev)

Builds and installs local relay client binaries, then restarts relay-hostd.

This is a thin wrapper around:
  scripts/package-client.sh --install-local

Usage:
  scripts/linux-refresh-local.sh
  scripts/linux-refresh-local.sh --install-dir ~/.relay/bin
  scripts/linux-refresh-local.sh --restart-mode user|system
  scripts/linux-refresh-local.sh --no-restart
  scripts/linux-refresh-local.sh --logs
  scripts/linux-refresh-local.sh --link-path
  scripts/linux-refresh-local.sh --install-systemd-user

Options:
  --install-dir <dir>     Install dir (default: ~/.relay/bin)
  --restart-mode <mode>   user|system (default: user)
  --no-restart            Skip restarting relay-hostd
  --logs                  After install/restart, follow logs (journalctl)
  --link-path             Create/update symlinks in ~/.local/bin for relay + relay-hostd
  --install-systemd-user   If restart-mode=user and unit missing, install+enable relay-hostd.service
EOF
}

INSTALL_DIR=""
RESTART_MODE="user"
NO_RESTART="0"
FOLLOW_LOGS="0"
LINK_PATH="0"
INSTALL_SYSTEMD_USER="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --install-dir) INSTALL_DIR="${2:-}"; shift 2 ;;
    --restart-mode) RESTART_MODE="${2:-}"; shift 2 ;;
    --no-restart) NO_RESTART="1"; shift ;;
    --logs) FOLLOW_LOGS="1"; shift ;;
    --link-path) LINK_PATH="1"; shift ;;
    --install-systemd-user) INSTALL_SYSTEMD_USER="1"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

args=("scripts/package-client.sh" "--install-local" "--restart-mode" "$RESTART_MODE")
if [[ -n "$INSTALL_DIR" ]]; then
  args+=("--install-dir" "$INSTALL_DIR")
fi
if [[ "$NO_RESTART" == "1" ]]; then
  args+=("--no-restart")
fi

bash "${args[@]}"

# Match scripts/package-client.sh default.
if [[ -z "$INSTALL_DIR" ]]; then
  INSTALL_DIR="$HOME/.relay/bin"
fi

HOSTD_BIN="$INSTALL_DIR/relay-hostd"
RELAY_BIN="$INSTALL_DIR/relay"

if [[ ! -x "$HOSTD_BIN" ]]; then
  echo "[linux-refresh-local] error: missing or non-executable: $HOSTD_BIN" >&2
  echo "[linux-refresh-local] hint: re-run with --install-dir, or check build failures above" >&2
  exit 1
fi
if [[ ! -x "$RELAY_BIN" ]]; then
  echo "[linux-refresh-local] error: missing or non-executable: $RELAY_BIN" >&2
  exit 1
fi

# Optional: make relay/relay-hostd discoverable on PATH (common cause of "relay-hostd not found"
# when a different `relay` wrapper is picked up from /usr/bin).
if [[ "$LINK_PATH" == "1" ]]; then
  LINK_DIR="$HOME/.local/bin"
  mkdir -p "$LINK_DIR"
  ln -sf "$RELAY_BIN" "$LINK_DIR/relay"
  ln -sf "$HOSTD_BIN" "$LINK_DIR/relay-hostd"
  echo "[linux-refresh-local] linked: $LINK_DIR/relay -> $RELAY_BIN" >&2
  echo "[linux-refresh-local] linked: $LINK_DIR/relay-hostd -> $HOSTD_BIN" >&2
  if [[ ":$PATH:" != *":$LINK_DIR:"* ]]; then
    echo "[linux-refresh-local] note: $LINK_DIR is not in PATH; add it or export PATH=\"$LINK_DIR:\$PATH\"" >&2
  fi
fi

# Best-effort: verify hostd is actually up by checking the unix socket.
detect_sock() {
  local sock=""
  if [[ -f "$HOME/.relay/hostd.json" ]] && command -v python3 >/dev/null 2>&1; then
    sock="$(python3 - <<'PY'
import json, os
path = os.path.expanduser('~/.relay/hostd.json')
try:
  with open(path, 'r', encoding='utf-8') as f:
    d = json.load(f)
  v = d.get('local_unix_socket')
  if isinstance(v, str) and v.strip():
    print(v.strip())
except Exception:
  pass
PY
)"
  fi
  if [[ -z "$sock" && -f "$HOME/.relay/daemon.state.json" ]] && command -v python3 >/dev/null 2>&1; then
    sock="$(python3 - <<'PY'
import json, os
path = os.path.expanduser('~/.relay/daemon.state.json')
try:
  with open(path, 'r', encoding='utf-8') as f:
    d = json.load(f)
  v = d.get('sock')
  if isinstance(v, str) and v.strip():
    print(v.strip())
except Exception:
  pass
PY
)"
  fi
  if [[ -z "$sock" ]]; then
    sock="$HOME/.relay/relay-hostd.sock"
  fi
  echo "$sock"
}

wait_for_sock() {
  local sock="$1"
  local i
  for i in $(seq 1 50); do
    [[ -S "$sock" ]] && return 0
    sleep 0.1
  done
  return 1
}

install_systemd_user_unit_if_missing() {
  # Only relevant for restart-mode=user.
  if [[ "$RESTART_MODE" != "user" ]]; then
    return 0
  fi
  if [[ "$INSTALL_SYSTEMD_USER" != "1" ]]; then
    return 0
  fi
  if ! command -v systemctl >/dev/null 2>&1; then
    echo "[linux-refresh-local] warning: systemctl not found; cannot install user unit" >&2
    return 0
  fi
  if ! systemctl --user show-environment >/dev/null 2>&1; then
    echo "[linux-refresh-local] warning: systemctl --user is not available (no user systemd session?)" >&2
    echo "[linux-refresh-local] hint: loginctl enable-linger $USER (or run with --restart-mode system)" >&2
    return 0
  fi
  if systemctl --user cat relay-hostd.service >/dev/null 2>&1; then
    return 0
  fi

  # Determine where systemd --user loads units from.
  local systemd_user_config_home
  systemd_user_config_home="$(systemctl --user show-environment 2>/dev/null | awk -F= '$1=="XDG_CONFIG_HOME"{print $2; exit}')"
  if [[ -z "$systemd_user_config_home" ]]; then
    systemd_user_config_home="$HOME/.config"
  fi
  local unit_dir="$systemd_user_config_home/systemd/user"
  mkdir -p "$unit_dir"

  mkdir -p "$HOME/.relay"
  local env_path="$HOME/.relay/hostd.env"
  # Keep it minimal; hostd will auto-init ~/.config/abrelay/hostd.json if missing.
  cat >"$env_path" <<EOF
LOCAL_UNIX_SOCKET=$HOME/.relay/relay-hostd.sock
SPOOL_DB_PATH=$HOME/.relay/hostd-spool.db
HOSTD_LOG_PATH=$HOME/.relay/hostd.log
EOF
  chmod 0600 "$env_path" 2>/dev/null || true

  local unit_path="$unit_dir/relay-hostd.service"
  cat >"$unit_path" <<EOF
[Unit]
Description=relay-hostd (user)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=%h/.relay/hostd.env
ExecStart=$HOSTD_BIN
Restart=always
RestartSec=2
NoNewPrivileges=true

[Install]
WantedBy=default.target
EOF

  systemctl --user daemon-reload
  systemctl --user enable --now relay-hostd.service
  echo "[linux-refresh-local] installed+enabled: relay-hostd.service ($unit_path)" >&2
}

sock_path="$(detect_sock)"
if [[ "$NO_RESTART" != "1" ]]; then
  install_systemd_user_unit_if_missing || true
  if ! wait_for_sock "$sock_path"; then
    echo "[linux-refresh-local] warning: hostd socket not found after restart: $sock_path" >&2
    echo "[linux-refresh-local] note: if you installed systemd unit, check status:" >&2
    if [[ "$RESTART_MODE" == "user" ]]; then
      echo "  systemctl --user status relay-hostd" >&2
    else
      echo "  systemctl status relay-hostd" >&2
    fi
    echo "[linux-refresh-local] note: if you are using a global npm/bun relay, ensure PATH prefers $INSTALL_DIR or pass relay --sock explicitly" >&2
  fi
fi

if [[ "$FOLLOW_LOGS" != "1" ]]; then
  exit 0
fi

if ! command -v journalctl >/dev/null 2>&1; then
  echo "journalctl not found; cannot follow logs" >&2
  exit 0
fi

if [[ "$RESTART_MODE" == "user" ]]; then
  if command -v systemctl >/dev/null 2>&1 && systemctl --user cat relay-hostd.service >/dev/null 2>&1; then
    exec journalctl --user -u relay-hostd -f
  fi
else
  if command -v systemctl >/dev/null 2>&1 && systemctl cat relay-hostd.service >/dev/null 2>&1; then
    echo "note: system logs may require sudo" >&2
    exec journalctl -u relay-hostd -f
  fi
fi

echo "relay-hostd systemd unit not found; skip log follow" >&2
