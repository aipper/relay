#!/usr/bin/env sh
set -eu

if [ -n "${ADMIN_PASSWORD:-}" ] && [ -z "${ADMIN_PASSWORD_HASH:-}" ]; then
  ADMIN_PASSWORD_HASH="$("/app/relay-server" --hash-password "$ADMIN_PASSWORD")"
  export ADMIN_PASSWORD_HASH
fi

exec /app/relay-server

