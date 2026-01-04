#!/usr/bin/env bash
set -euo pipefail

echo "[mock-codex] starting"
echo "This is a mock codex session used for e2e testing."
echo
echo "Proceed? [y/N]"

read -r answer || exit 0
answer_lc="$(printf '%s' "$answer" | tr '[:upper:]' '[:lower:]')"
case "$answer_lc" in
  y|yes)
    echo "[mock-codex] approved"
    ;;
  *)
    echo "[mock-codex] denied"
    exit 0
    ;;
esac

echo "Type anything, Ctrl-D to exit."
while IFS= read -r line; do
  echo "[mock-codex] echo: $line"
done
