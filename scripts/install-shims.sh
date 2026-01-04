#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
relay shims installer

Installs command shims so you can run `codex` / `claude` / `iflow` directly in any project dir,
and have them start a relay run (via hostd) with cwd = current directory.

Important:
  - Your machine likely already has real `codex` / `claude` / `iflow` binaries.
  - These shims intentionally shadow those names in PATH.
  - To avoid recursion (hostd calling the shim again), we record the real binary paths into:
      ~/.relay/bin-map.json

Usage:
  scripts/install-shims.sh [--dir ~/.local/bin] [--tools codex,claude,iflow]
  scripts/install-shims.sh --uninstall [--dir ~/.local/bin] [--tools codex,claude,iflow]

Options:
  --dir <path>       Install directory (default: ~/.local/bin)
  --tools <list>     Comma-separated tool list (default: codex,claude,iflow)
  --auto-path        Ensure install dir is in PATH by updating shell rc (with backup)
  --uninstall        Remove installed shims (does not delete bin-map.json)
EOF
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

INSTALL_DIR="${HOME}/.local/bin"
TOOLS="codex,claude,iflow"
UNINSTALL="0"
AUTO_PATH="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --dir) INSTALL_DIR="${2:-}"; shift 2 ;;
    --tools) TOOLS="${2:-}"; shift 2 ;;
    --uninstall) UNINSTALL="1"; shift ;;
    --auto-path) AUTO_PATH="1"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$INSTALL_DIR" ]]; then
  echo "--dir requires a value" >&2
  exit 2
fi

need mkdir
need cat

mkdir -p "$INSTALL_DIR"

RELAY_DIR="${HOME}/.relay"
BIN_MAP="${RELAY_DIR}/bin-map.json"
mkdir -p "$RELAY_DIR"
touch "$BIN_MAP"
chmod 600 "$BIN_MAP" >/dev/null 2>&1 || true

IFS=',' read -r -a tool_arr <<<"$TOOLS"

MARKER_BEGIN="# >>> relay shim PATH >>>"
MARKER_END="# <<< relay shim PATH <<<"

is_shim() {
  local path="$1"
  [[ -f "$path" ]] && grep -F "relay shim (installed by scripts/install-shims.sh)" "$path" >/dev/null 2>&1
}

realpath_node() {
  node -e 'const fs=require("fs"); try { console.log(fs.realpathSync(process.argv[1])); } catch { process.exit(2); }' "$1" 2>/dev/null || true
}

clean_path_without_dir() {
  local dir="$1"
  local out=""
  local first="1"
  IFS=':' read -r -a parts <<<"${PATH:-}"
  for p in "${parts[@]}"; do
    [[ -z "$p" ]] && continue
    [[ "$p" == "$dir" ]] && continue
    if [[ "$first" == "1" ]]; then
      out="$p"
      first="0"
    else
      out="${out}:$p"
    fi
  done
  echo "$out"
}

resolve_real_binary() {
  local tool="$1"
  local resolved=""
  resolved="$(command -v "$tool" 2>/dev/null || true)"

  if [[ -z "$resolved" ]]; then
    echo ""
    return 0
  fi

  # If PATH already points to our install dir and it's a shim, try to find the next one by
  # temporarily removing INSTALL_DIR from PATH.
  if is_shim "$resolved"; then
    local cleaned
    cleaned="$(clean_path_without_dir "$INSTALL_DIR")"
    local alt
    alt="$(PATH="$cleaned" command -v "$tool" 2>/dev/null || true)"
    if [[ -n "$alt" && "$alt" != "$resolved" ]] && ! is_shim "$alt"; then
      resolved="$alt"
    fi
  fi

  # Record the final resolved target (avoid recording a symlink path that we are about to overwrite).
  local rp
  rp="$(realpath_node "$resolved")"
  if [[ -n "$rp" ]]; then
    echo "$rp"
  else
    echo "$resolved"
  fi
}

load_map() {
  if [[ -f "$BIN_MAP" ]]; then
    cat "$BIN_MAP"
  else
    echo "{}"
  fi
}

write_map() {
  local json="$1"
  cat >"$BIN_MAP" <<<"$json"
}

if [[ "$UNINSTALL" == "1" ]]; then
  for t in "${tool_arr[@]}"; do
    t="$(echo "$t" | xargs)"
    [[ -z "$t" ]] && continue
    target="${INSTALL_DIR}/${t}"
    if [[ -f "$target" ]]; then
      rm -f "$target"
      echo "[shim] removed: $target"
    else
      echo "[shim] not found: $target"
    fi
  done

  # Remove PATH block if it was added by us.
  maybe_update_path() { :; }
  maybe_remove_path_block() {
    local shell="${SHELL:-}"
    local rc=""
    if [[ "$shell" == */zsh ]]; then
      rc="${HOME}/.zshrc"
    elif [[ "$shell" == */bash ]]; then
      rc="${HOME}/.bashrc"
    else
      rc="${HOME}/.zshrc"
    fi
    [[ -f "$rc" ]] || return 0
    if ! grep -F "$MARKER_BEGIN" "$rc" >/dev/null 2>&1; then
      return 0
    fi
    local ts
    ts="$(date +%Y%m%d%H%M%S)"
    cp "$rc" "${rc}.bak.${ts}"
    local tmp
    tmp="$(mktemp)"
    awk -v b="$MARKER_BEGIN" -v e="$MARKER_END" '
      $0==b {skip=1; next}
      $0==e {skip=0; next}
      skip!=1 {print}
    ' "$rc" >"$tmp"
    mv "$tmp" "$rc"
    echo "[shim] removed PATH block from: $rc (backup: ${rc}.bak.${ts})"
  }
  maybe_remove_path_block
  exit 0
fi

need node

MAP_JSON="$(load_map)"

for t in "${tool_arr[@]}"; do
  t="$(echo "$t" | xargs)"
  [[ -z "$t" ]] && continue

  target="${INSTALL_DIR}/${t}"

  # Resolve current tool binary BEFORE we install/overwrite the shim.
  resolved="$(resolve_real_binary "$t")"
  if [[ -z "$resolved" ]]; then
    echo "[shim] warning: cannot find existing $t in PATH; will still install shim, but hostd may need RELAY_<TOOL>_BIN or ~/.relay/bin-map.json filled later" >&2
  else
    if is_shim "$resolved"; then
      echo "[shim] $t already points to a relay shim: $resolved"
    else
      # Update bin-map.json with the real path (best-effort).
      MAP_JSON="$(
        node -e '
          const fs=require("fs");
          const tool=process.argv[1];
          const real=process.argv[2];
          let m={};
          try { m=JSON.parse(fs.readFileSync(0,"utf8")||"{}"); } catch {}
          if (typeof m!=="object" || !m) m={};
          m[tool]=real;
          process.stdout.write(JSON.stringify(m,null,2)+"\n");
        ' "$t" "$resolved" <<<"$MAP_JSON"
      )"
      echo "[shim] recorded real $t -> $resolved"
    fi
  fi

  cat >"$target" <<EOF
#!/usr/bin/env bash
set -euo pipefail

# relay shim (installed by scripts/install-shims.sh)
#
# This shim shadows the real \`$t\` command and starts a relay run in the current directory.
# The real binary path is recorded in ~/.relay/bin-map.json so hostd can avoid recursion.

if [[ \$# -gt 0 ]]; then
  # Best-effort argv passthrough: this loses shell-quoting, but works for common flag-style args.
  cmd="$t \$*"
  exec relay "$t" --cwd "\$PWD" --cmd "\$cmd"
fi

exec relay "$t" --cwd "\$PWD"
EOF

  chmod +x "$target"
  echo "[shim] installed: $target"
done

write_map "$MAP_JSON"
echo "[shim] bin map: $BIN_MAP"

maybe_update_path() {
  local dir="$1"

  # Already in PATH?
  case ":$PATH:" in
    *":$dir:"*) return 0 ;;
  esac

  local shell="${SHELL:-}"
  local rc=""
  if [[ "$shell" == */zsh ]]; then
    rc="${HOME}/.zshrc"
  elif [[ "$shell" == */bash ]]; then
    rc="${HOME}/.bashrc"
  else
    # Best-effort: default to zsh on macOS.
    rc="${HOME}/.zshrc"
  fi

  mkdir -p "$(dirname "$rc")"
  touch "$rc"

  if grep -F "$MARKER_BEGIN" "$rc" >/dev/null 2>&1; then
    return 0
  fi

  local ts
  ts="$(date +%Y%m%d%H%M%S)"
  cp "$rc" "${rc}.bak.${ts}"

  cat >>"$rc" <<EOF

$MARKER_BEGIN
# Added by relay installer on ${ts}
export PATH="${dir}:\$PATH"
$MARKER_END
EOF

  echo "[shim] updated PATH in: $rc (backup: ${rc}.bak.${ts})"
}

if [[ "$AUTO_PATH" == "1" ]]; then
  maybe_update_path "$INSTALL_DIR"
fi

cat <<EOF

Next:
  - Ensure \`$INSTALL_DIR\` is in your PATH.
    Example:
      export PATH="\$HOME/.local/bin:\$PATH"

Note:
  - These shims require a running relay-hostd (background) and a discoverable unix socket.
    Recommended: run packaged \`up.sh\` or \`relay daemon start\` (Bun CLI) first.
  - Default socket path (if not overridden by LOCAL_UNIX_SOCKET): \$HOME/.relay/relay-hostd.sock
EOF
