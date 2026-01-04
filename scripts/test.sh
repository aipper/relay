#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
relay automated test runner

Usage:
  scripts/test.sh [--fast] [--e2e-only] [--no-e2e] [--no-clippy] [--no-fmt]

Options:
  --fast       Skip E2E (fmt + clippy + cargo test only).
  --e2e-only   Run only scripts/e2e.sh.
  --no-e2e     Do not run scripts/e2e.sh.
  --no-clippy  Skip cargo clippy.
  --no-fmt     Skip cargo fmt --check.
EOF
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

run() {
  echo "+ $*"
  "$@"
}

skip_e2e=false
only_e2e=false
skip_clippy=false
skip_fmt=false

for arg in "$@"; do
  case "$arg" in
    -h|--help) usage; exit 0 ;;
    --fast) skip_e2e=true ;;
    --e2e-only) only_e2e=true ;;
    --no-e2e) skip_e2e=true ;;
    --no-clippy) skip_clippy=true ;;
    --no-fmt) skip_fmt=true ;;
    *) echo "unknown arg: $arg" >&2; usage; exit 2 ;;
  esac
done

need cargo

if [[ "$only_e2e" == "true" ]]; then
  run bash scripts/e2e.sh
  exit 0
fi

if [[ "$skip_fmt" != "true" ]]; then
  run cargo fmt --check
fi

if [[ "$skip_clippy" != "true" ]]; then
  run cargo clippy --all-targets --all-features
fi

run cargo test --workspace

if [[ "$skip_e2e" != "true" ]]; then
  run bash scripts/e2e.sh
fi

echo "ok"
