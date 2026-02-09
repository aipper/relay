#!/usr/bin/env bash
set -euo pipefail

# Build and push relay-server Docker image to Aliyun ACR.
#
# Usage:
#   bash build.sh
#   REPO_NAME=relay SERVER_PLATFORM=linux/amd64 bash build.sh
#
# Auth:
#   export ACR_USERNAME="aipper@qq.com"
#   export ACR_PASSWORD="<your-acr-password>"   # avoid putting secrets in shell history

REPO_URL="${REPO_URL:-registry.cn-hangzhou.aliyuncs.com}"
NAMESPACE="${NAMESPACE:-aipper}"
REPO_NAME="${REPO_NAME:-relay-server}"
IMAGE_TAG="${IMAGE_TAG:-$(date +"%Y%m%d%H%M")}"

# Single-platform by default; set to "linux/amd64,linux/arm64" if needed.
SERVER_PLATFORM="${SERVER_PLATFORM:-linux/amd64}"

ACR_USERNAME="${ACR_USERNAME:-aipper@qq.com}"

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not found" >&2
  exit 1
fi
if ! docker buildx version >/dev/null 2>&1; then
  echo "error: docker buildx is required (Docker 19.03+)" >&2
  exit 1
fi

if [[ -z "${ACR_PASSWORD:-}" ]]; then
  # Read from TTY so pipelines don't accidentally capture it.
  if [[ -t 0 ]]; then
    read -r -s -p "ACR password for ${ACR_USERNAME}@${REPO_URL}: " ACR_PASSWORD
    echo
  else
    echo "error: ACR_PASSWORD is not set (and no TTY available to prompt)" >&2
    exit 1
  fi
fi

FULL_IMAGE="${REPO_URL}/${NAMESPACE}/${REPO_NAME}:${IMAGE_TAG}"

echo "Login: ${REPO_URL}"
echo "${ACR_PASSWORD}" | docker login --username="${ACR_USERNAME}" --password-stdin "${REPO_URL}"

echo "Build+push: ${FULL_IMAGE} (platform: ${SERVER_PLATFORM})"
docker buildx build \
  --platform "${SERVER_PLATFORM}" \
  --push \
  -t "${FULL_IMAGE}" \
  .

echo "done: ${FULL_IMAGE}"
