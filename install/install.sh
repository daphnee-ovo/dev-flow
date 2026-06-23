#!/bin/bash
# dow 安装脚本 — 一条命令安装
# 用法: curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
set -e

REPO="daphnee-ovo/dev-flow"
BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/dow"
BUNDLE_DIR="$DATA_DIR/bundle"

# 颜色输出
info()  { printf "\033[0;34m[dow]\033[0m %s\n" "$1"; }
ok()    { printf "\033[0;32m[dow]\033[0m ✓ %s\n" "$1"; }
err()   { printf "\033[0;31m[dow]\033[0m ✗ %s\n" "$1" >&2; }

# 检测平台
detect_platform() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${os}" in
    linux)  os="linux" ;;
    darwin) os="darwin" ;;
    *)      err "不支持的操作系统: ${os}"; exit 1 ;;
  esac

  case "${arch}" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             err "不支持的架构: ${arch}"; exit 1 ;;
  esac

  # darwin 用 arm64 命名
  if [ "$os" = "darwin" ] && [ "$arch" = "aarch64" ]; then
    echo "darwin-arm64"
  elif [ "$os" = "darwin" ] && [ "$arch" = "x86_64" ]; then
    echo "darwin-x86_64"
  else
    echo "${os}-${arch}"
  fi
}

# 获取最新版本
get_latest_version() {
  local url="https://api.github.com/repos/${REPO}/releases/latest"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  else
    err "需要 curl 或 wget"
    exit 1
  fi
}

# 下载文件（带进度 + 重试 + 超时）
download() {
  local url="$1" dest="$2"
  local retries=3 attempt=0
  while [ $attempt -lt $retries ]; do
    attempt=$((attempt + 1))
    if [ $attempt -gt 1 ]; then
      info "重试下载 (${attempt}/${retries})..."
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
  info "检测平台..."
  local platform
  platform="$(detect_platform)"
  info "平台: ${platform}"

  info "获取最新版本..."
  local version
  version="$(get_latest_version)"
  if [ -z "$version" ]; then
    err "无法获取最新版本（网络问题或无 Release）"
    exit 1
  fi
  info "版本: ${version}"

  # 下载
  local filename="dow-${version}-${platform}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${filename}"
  local tmp_dir
  tmp_dir="$(mktemp -d)"

  info "下载 ${filename}..."
  if ! download "$url" "${tmp_dir}/${filename}"; then
    err "下载失败（重试 3 次后仍失败）: ${url}"
    err "请检查网络连接或手动下载"
    rm -rf "$tmp_dir"
    exit 1
  fi

  # 解压
  info "安装中..."
  tar -xzf "${tmp_dir}/${filename}" -C "${tmp_dir}"

  # 安装二进制
  mkdir -p "$BIN_DIR"
  if [ -f "${tmp_dir}/bin/dow" ]; then
    cp "${tmp_dir}/bin/dow" "${BIN_DIR}/dow"
  elif [ -f "${tmp_dir}/dow" ]; then
    cp "${tmp_dir}/dow" "${BIN_DIR}/dow"
  else
    err "tarball 中未找到 dow 二进制"
    exit 1
  fi
  chmod +x "${BIN_DIR}/dow"

  # 安装 bundle
  mkdir -p "$BUNDLE_DIR"
  if [ -d "${tmp_dir}/bundle" ]; then
    rm -rf "$BUNDLE_DIR"
    cp -r "${tmp_dir}/bundle" "$BUNDLE_DIR"
  fi

  # 清理
  rm -rf "$tmp_dir"
  ok "dow ${version} 已安装到 ${BIN_DIR}/dow"

  # 检查 PATH
  case ":$PATH:" in
    *":${BIN_DIR}:"*) ;;
    *)
      info "将 ${BIN_DIR} 添加到 PATH..."
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
        info "已添加到 ${shell_rc}，请重新打开终端或执行: source ${shell_rc}"
      else
        info "请手动添加到 PATH: export PATH=\"\$HOME/.local/bin:\$PATH\""
      fi
      export PATH="${BIN_DIR}:$PATH"
      ;;
  esac

  echo ""
  # 运行 setup
  info "启动设置引导..."
  "${BIN_DIR}/dow" setup
}

main "$@"
