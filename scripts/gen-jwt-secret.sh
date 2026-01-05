#!/usr/bin/env bash
set -euo pipefail

# Generates a stable JWT secret (hex) for relay-server config.
# Recommended: store the output in docker/server.env (JWT_SECRET=...).

if command -v openssl >/dev/null 2>&1; then
  openssl rand -hex 32
  exit 0
fi

if command -v python3 >/dev/null 2>&1; then
  python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
  exit 0
fi

echo "error: need 'openssl' or 'python3' to generate a JWT secret" >&2
exit 1

