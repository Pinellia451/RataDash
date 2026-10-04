#!/usr/bin/env bash
set -Eeuo pipefail

PROJECT_NAME="ratadash"
REPOSITORY="Pinellia451/RataDash"
INSTALL_DIR="${RATADASH_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="latest"

usage() {
  cat <<'EOF'
RataDash binary installer

Downloads the matching prebuilt binary from the latest GitHub Release.

Usage:
  install.sh [options]

Options:
  --install-dir DIR  Install the binary into DIR.
  --version VERSION  Install a specific release tag instead of latest.
  -h, --help         Show this help.

Environment:
  RATADASH_INSTALL_DIR  Same as --install-dir.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-dir)
      [[ $# -ge 2 ]] || { echo "error: --install-dir needs a value" >&2; exit 2; }
      INSTALL_DIR="$2"
      shift 2
      ;;
    --version)
      [[ $# -ge 2 ]] || { echo "error: --version needs a value" >&2; exit 2; }
      VERSION="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$(uname -s):$(uname -m)" in
  Darwin:arm64|Darwin:aarch64)
    ASSET="ratadash-darwin-arm64"
    ;;
  Darwin:x86_64|Darwin:amd64)
    ASSET="ratadash-darwin-x86_64"
    ;;
  Linux:x86_64|Linux:amd64)
    ASSET="ratadash-linux-x86_64"
    ;;
  *)
    echo "error: no prebuilt RataDash release for $(uname -s)-$(uname -m)" >&2
    echo "       supported platforms: Linux x86_64, macOS arm64, macOS x86_64" >&2
    exit 1
    ;;
esac

if [[ "${VERSION}" == "latest" ]]; then
  DOWNLOAD_URL="https://github.com/${REPOSITORY}/releases/latest/download/${ASSET}"
else
  DOWNLOAD_URL="https://github.com/${REPOSITORY}/releases/download/${VERSION}/${ASSET}"
fi

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ratadash-install.XXXXXX")"
cleanup() {
  rm -rf "${TEMP_DIR}"
}
trap cleanup EXIT

DOWNLOADED_BINARY="${TEMP_DIR}/${PROJECT_NAME}"
echo "info: detected $(uname -s) $(uname -m)"
echo "info: downloading ${ASSET} (${VERSION})"

if command -v curl >/dev/null 2>&1; then
  curl --fail --location --silent --show-error "${DOWNLOAD_URL}" --output "${DOWNLOADED_BINARY}"
elif command -v wget >/dev/null 2>&1; then
  wget --quiet --output-document="${DOWNLOADED_BINARY}" "${DOWNLOAD_URL}"
else
  echo "error: curl or wget is required to download RataDash" >&2
  exit 1
fi

if [[ ! -s "${DOWNLOADED_BINARY}" ]]; then
  echo "error: downloaded release asset is empty: ${DOWNLOAD_URL}" >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
TEMP_INSTALL="$(mktemp "${INSTALL_DIR}/.${PROJECT_NAME}.tmp.XXXXXX")"
install -m 755 "${DOWNLOADED_BINARY}" "${TEMP_INSTALL}"
mv -f "${TEMP_INSTALL}" "${INSTALL_DIR}/${PROJECT_NAME}"

echo "info: installed ${PROJECT_NAME} to ${INSTALL_DIR}/${PROJECT_NAME}"

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "warn: ${INSTALL_DIR} is not in your PATH"
    case "${SHELL##*/}" in
      zsh)
        echo
        echo 'Add this to $HOME/.zshrc:'
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
      fish)
        echo
        echo 'Run this in fish:'
        echo "  fish_add_path ${INSTALL_DIR}"
        ;;
      *)
        echo
        echo 'Add this to $HOME/.bashrc:'
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
    esac
    ;;
esac

echo "info: run ${PROJECT_NAME} --version to verify"
