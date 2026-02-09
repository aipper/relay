#!/usr/bin/env bash
set -euo pipefail

# Build and push relay-server Docker image to a Docker Registry.
#
# Default target: self-hosted registry https://registry.aipper.de
#
# Usage:
#   bash build.sh
#   REPO_NAME=relay SERVER_PLATFORM=linux/amd64 bash build.sh
#
# Auth (optional; for basic auth / htpasswd, etc):
#   export REGISTRY_USERNAME="<user>"
#   export REGISTRY_PASSWORD="<password>"   # avoid putting secrets in shell history

REPO_URL="${REPO_URL:-registry.aipper.de}"
NAMESPACE="${NAMESPACE:-aipper}"
REPO_NAME="${REPO_NAME:-relay-server}"
IMAGE_TAG="${IMAGE_TAG:-$(date +"%Y%m%d%H%M")}"

# Single-platform by default; set to "linux/amd64,linux/arm64" if needed.
SERVER_PLATFORM="${SERVER_PLATFORM:-linux/amd64}"

# Backward compatible aliases:
# - old: ACR_USERNAME / ACR_PASSWORD
REGISTRY_USERNAME="${REGISTRY_USERNAME:-${ACR_USERNAME:-}}"
REGISTRY_PASSWORD="${REGISTRY_PASSWORD:-${ACR_PASSWORD:-}}"

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not found" >&2
  exit 1
fi
if ! docker buildx version >/dev/null 2>&1; then
  echo "error: docker buildx is required (Docker 19.03+)" >&2
  exit 1
fi

REPO_URL_LOGIN="${REPO_URL#http://}"
REPO_URL_LOGIN="${REPO_URL_LOGIN#https://}"

if [[ -z "${REGISTRY_USERNAME:-}" ]]; then
  REGISTRY_USERNAME="${USER:-}"
fi

FULL_IMAGE="${REPO_URL_LOGIN}/${NAMESPACE}/${REPO_NAME}:${IMAGE_TAG}"

if [[ -n "${REGISTRY_PASSWORD:-}" || -n "${REGISTRY_USERNAME:-}" ]]; then
  if [[ -z "${REGISTRY_PASSWORD:-}" ]]; then
    # If registry is anonymous, don't prompt; just skip login.
    echo "Login: skipped (no REGISTRY_PASSWORD set)"
  else
    echo "Login: ${REPO_URL_LOGIN}"
    echo "${REGISTRY_PASSWORD}" | docker login --username="${REGISTRY_USERNAME}" --password-stdin "${REPO_URL_LOGIN}"
  fi
else
  echo "Login: skipped (anonymous registry)"
fi

echo "Build+push: ${FULL_IMAGE} (platform: ${SERVER_PLATFORM})"
docker buildx build \
  --platform "${SERVER_PLATFORM}" \
  --push \
  -t "${FULL_IMAGE}" \
  .

echo "done: ${FULL_IMAGE}"
