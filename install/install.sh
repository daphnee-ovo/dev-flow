#!/bin/bash
# dow installer — one command install
# Usage: curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
set -e

REPO="daphnee-ovo/dev-flow"
BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/dow"
BUNDLE_DIR="$DATA_DIR/bundle"

info()  { printf "\033[0;34m[dow]\033[0m %s\n" "$1"; }
ok()    { printf "\033[0;32m[dow]\033[0m ✓ %s\n" "$1"; }
err()   { printf "\033[0;31m[dow]\033[0m ✗ %s\n" "$1" >&2; }

detect_platform() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${os}" in
    linux)  os="linux" ;;
    darwin) os="darwin" ;;
    *)      err "Unsupported OS: ${os}"; exit 1 ;;
  esac

  case "${arch}" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             err "Unsupported architecture: ${arch}"; exit 1 ;;
  esac

  if [ "$os" = "darwin" ] && [ "$arch" = "aarch64" ]; then
    echo "darwin-arm64"
  elif [ "$os" = "darwin" ] && [ "$arch" = "x86_64" ]; then
    echo "darwin-x86_64"
  else
    echo "${os}-${arch}"
  fi
}

get_latest_version() {
  local url="https://api.github.com/repos/${REPO}/releases/latest"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  else
    err "curl or wget is required"
    exit 1
  fi
}

# Download with progress, retry, and timeout
download() {
  local url="$1" dest="$2"
  local retries=3 attempt=0
  while [ $attempt -lt $retries ]; do
    attempt=$((attempt + 1))
    if [ $attempt -gt 1 ]; then
      info "Retrying download (${attempt}/${retries})..."
      sleep 2
    fi
    if command -v curl >/dev/null 2>&1; then
      if curl -fL --progress-bar --retry 2 --connect-timeout 10 --max-time 120 "$url" -o "$dest"; then
        if [ -s "$dest" ]; then
          return 0
        fi
      fi
    elif command -v wget >/dev/null 2>&1; then
      if wget --show-progress -q --timeout=10 --tries=2 "$url" -O "$dest"; then
        if [ -s "$dest" ]; then
          return 0
        fi
      fi
    fi
    rm -f "$dest"
  done
  return 1
}

main() {
  info "Detecting platform..."
  local platform
  platform="$(detect_platform)"
  info "Platform: ${platform}"

  info "Fetching latest version..."
  local version
  version="$(get_latest_version)"
  if [ -z "$version" ]; then
    err "Failed to get latest version (network issue or no release found)"
    exit 1
  fi
  info "Version: ${version}"

  local filename="dow-${version}-${platform}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${filename}"
  local tmp_dir
  tmp_dir="$(mktemp -d)"

  info "Downloading ${filename}..."
  if ! download "$url" "${tmp_dir}/${filename}"; then
    err "Download failed after 3 retries: ${url}"
    err "Please check your network connection or download manually"
    rm -rf "$tmp_dir"
    exit 1
  fi

  info "Installing..."
  tar -xzf "${tmp_dir}/${filename}" -C "${tmp_dir}"

  mkdir -p "$BIN_DIR"
  if [ -f "${tmp_dir}/bin/dow" ]; then
    cp "${tmp_dir}/bin/dow" "${BIN_DIR}/dow"
  elif [ -f "${tmp_dir}/dow" ]; then
    cp "${tmp_dir}/dow" "${BIN_DIR}/dow"
  else
    err "dow binary not found in tarball"
    exit 1
  fi
  chmod +x "${BIN_DIR}/dow"

  mkdir -p "$BUNDLE_DIR"
  if [ -d "${tmp_dir}/bundle" ]; then
    rm -rf "$BUNDLE_DIR"
    cp -r "${tmp_dir}/bundle" "$BUNDLE_DIR"
  fi

  rm -rf "$tmp_dir"
  ok "dow ${version} installed to ${BIN_DIR}/dow"

  case ":$PATH:" in
    *":${BIN_DIR}:"*) ;;
    *)
      info "Adding ${BIN_DIR} to PATH..."
      local shell_rc=""
      if [ -f "$HOME/.zshrc" ]; then
        shell_rc="$HOME/.zshrc"
      elif [ -f "$HOME/.bashrc" ]; then
        shell_rc="$HOME/.bashrc"
      elif [ -f "$HOME/.profile" ]; then
        shell_rc="$HOME/.profile"
      fi

      if [ -n "$shell_rc" ]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$shell_rc"
        info "Added to ${shell_rc}. Restart your terminal or run: source ${shell_rc}"
      else
        info "Please add to PATH manually: export PATH=\"\$HOME/.local/bin:\$PATH\""
      fi
      export PATH="${BIN_DIR}:$PATH"
      ;;
  esac

  echo ""
  info "Starting setup..."
  "${BIN_DIR}/dow" setup
}

main "$@"
