#!/usr/bin/env bash
set -Eeuo pipefail

PROJECT_NAME="ratadash"
INSTALL_DIR="${RATADASH_INSTALL_DIR:-${HOME}/.local/bin}"
BINARY_PATH=""

usage() {
  cat <<'EOF'
RataDash binary installer

This script only copies a prebuilt RataDash binary. It does not clone,
download, compile, or install Rust.

Usage:
  ./install.sh [options]

Options:
  --install-dir DIR  Install the binary into DIR.
  --binary PATH      Copy this binary instead of auto-detecting release/.
  -h, --help         Show this help.

Environment:
  RATADASH_INSTALL_DIR  Same as --install-dir.
  RATADASH_BINARY       Same as --binary.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-dir)
      [[ $# -ge 2 ]] || { echo "error: --install-dir needs a value" >&2; exit 2; }
      INSTALL_DIR="$2"
      shift 2
      ;;
    --binary)
      [[ $# -ge 2 ]] || { echo "error: --binary needs a value" >&2; exit 2; }
      BINARY_PATH="$2"
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

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="${RATADASH_BINARY:-${BINARY_PATH}}"

if [[ -z "${BINARY_PATH}" ]]; then
  case "$(uname -s):$(uname -m)" in
    Darwin:arm64|Darwin:aarch64)
      BINARY_PATH="${SCRIPT_DIR}/release/ratadash-darwin-arm64"
      ;;
    Darwin:x86_64|Darwin:amd64)
      BINARY_PATH="${SCRIPT_DIR}/release/ratadash-darwin-x86_64"
      ;;
    Linux:x86_64|Linux:amd64)
      BINARY_PATH="${SCRIPT_DIR}/release/ratadash-linux-x86_64"
      ;;
    Linux:aarch64|Linux:arm64)
      BINARY_PATH="${SCRIPT_DIR}/release/ratadash-linux-arm64"
      ;;
    *)
      echo "error: unsupported platform $(uname -s)-$(uname -m); use --binary PATH" >&2
      exit 1
      ;;
  esac
fi

if [[ ! -f "${BINARY_PATH}" ]]; then
  echo "error: prebuilt binary not found: ${BINARY_PATH}" >&2
  echo "       build or copy a release binary first, or pass --binary PATH" >&2
  exit 1
fi
if [[ ! -x "${BINARY_PATH}" ]]; then
  echo "error: binary is not executable: ${BINARY_PATH}" >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
TEMP_BINARY="${INSTALL_DIR}/.${PROJECT_NAME}.tmp.$$"
install -m 755 "${BINARY_PATH}" "${TEMP_BINARY}"
mv -f "${TEMP_BINARY}" "${INSTALL_DIR}/${PROJECT_NAME}"

echo "info: installed ${PROJECT_NAME} to ${INSTALL_DIR}/${PROJECT_NAME}"

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "warn: ${INSTALL_DIR} is not in your PATH"
    case "${SHELL##*/}" in
      zsh) SHELL_RC="\$HOME/.zshrc" ;;
      fish) SHELL_RC="\$HOME/.config/fish/config.fish" ;;
      *) SHELL_RC="\$HOME/.bashrc" ;;
    esac
    echo
    echo "Add this to ${SHELL_RC}:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo
    echo "Then restart your shell or run:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac

echo "info: run ${PROJECT_NAME} --version to verify"
