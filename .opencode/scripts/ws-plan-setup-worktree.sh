#!/usr/bin/env bash
# ws-plan: create change context with worktree/submodule support
# Usage: bash ws-plan-setup-worktree.sh <change-id>
set -euo pipefail

change_id="${1:?missing change_id}"

# Guard: refuse to create context on dirty working tree
if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree dirty before ws-plan creates change context"
  echo "hint: commit/stash first, or continue inside an existing change/<change-id> context"
  exit 2
fi

has_commits=0
git rev-parse --verify HEAD >/dev/null 2>&1 && has_commits=1

has_submodules=0
if [[ -f .gitmodules ]] && git config --file .gitmodules --get-regexp '^submodule\..*\.path$' >/dev/null 2>&1; then
  has_submodules=1
fi

if [[ "${has_commits}" -eq 1 ]]; then
  if [[ "${has_submodules}" -eq 1 ]]; then
    aiws change start "${change_id}" --hooks --worktree --submodules
  else
    aiws change start "${change_id}" --hooks --worktree
  fi
else
  aiws change start "${change_id}" --hooks --no-switch
fi
