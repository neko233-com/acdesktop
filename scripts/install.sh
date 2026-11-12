#!/usr/bin/env bash
# acdesktop 一键安装脚本 —— macOS arm64
#
# 用法：
#   curl -fsSL https://raw.githubusercontent.com/neko233/acdesktop/main/scripts/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/neko233/acdesktop/main/scripts/install.sh | bash -s -- --version v1.2.0
#   bash scripts/install.sh --install-dir /usr/local/bin
#
# 设计原则：
#   - 零 Node 依赖，直接下载原生二进制
#   - 单次下载、单次校验、原子替换
#   - 响应快：用 GitHub Release API 拿版本号，checksum 并行

set -euo pipefail

REPO="neko233/acdesktop"
GH_API="https://api.github.com/repos/${REPO}"
GH_RELEASES="https://github.com/${REPO}/releases"

# ---------- 参数 ----------
INSTALL_DIR="${ACDESKTOP_INSTALL_DIR:-$HOME/.local/bin}"
REQUESTED_VERSION=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)    REQUESTED_VERSION="$2"; shift 2 ;;
    --install-dir) INSTALL_DIR="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,8p' "$0"; exit 0 ;;
    *) echo "未知参数: $1"; exit 1 ;;
  esac
done

# ---------- 平台检测 ----------
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) TARGET="aarch64-apple-darwin" ;;
  *)
    echo "✗ acdesktop 一键安装脚本仅支持 macOS arm64" >&2
    echo "  当前: $(uname -s)/$(uname -m)" >&2
    echo "  请下载对应 release archive: ${GH_RELEASES}" >&2
    exit 1 ;;
esac

BINARY_NAME="acdesktop"
HELPER_NAME="acdesktop-macos-helper"

# ---------- 获取版本号 ----------
if [[ -n "$REQUESTED_VERSION" ]]; then
  TAG="$REQUESTED_VERSION"
  [[ "$TAG" == v* ]] || TAG="v${TAG}"
else
  # 直接拿最新 release tag —— 用 curl head 比完整 GET 更快
  TAG=$(curl -fsSLI -o /dev/null -w '%{redirect_url}' \
    "${GH_RELEASES}/latest" | sed 's|.*/tag/||')
fi
VERSION="${TAG#v}"

TARBALL="acdesktop-v${VERSION}-${TARGET}.tar.gz"
BASE_URL="${GH_RELEASES}/download/${TAG}"
TARBALL_URL="${BASE_URL}/${TARBALL}"
CHECKSUMS_URL="${BASE_URL}/checksums.txt"

# ---------- 下载并校验 ----------
download_and_verify() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  echo "→ 下载 ${TARBALL} …"
  # 并行下载 tarball + checksums
  curl -fsSL --retry 3 --retry-delay 1 -o "${tmp}/${TARBALL}"   "${TARBALL_URL}"   &
  curl -fsSL --retry 3 --retry-delay 1 -o "${tmp}/checksums.txt" "${CHECKSUMS_URL}" &
  wait

  echo "→ SHA256 校验 …"
  local expected
  expected=$(grep "  ${TARBALL}" "${tmp}/checksums.txt" | awk '{print $1}')
  if [[ -z "$expected" ]]; then
    echo "✗ checksums.txt 中找不到 ${TARBALL}" >&2
    exit 1
  fi
  local actual
  actual=$(shasum -a 256 "${tmp}/${TARBALL}" | awk '{print $1}')
  if [[ "$actual" != "$expected" ]]; then
    echo "✗ SHA256 不匹配！" >&2
    echo "  期望: ${expected}" >&2
    echo "  实际: ${actual}"   >&2
    exit 1
  fi

  echo "→ 解压 …"
  mkdir -p "${INSTALL_DIR}"
  tar -xzf "${tmp}/${TARBALL}" -C "${tmp}"

  # ---------- 原子替换 ----------
  local bin_dest="${INSTALL_DIR}/${BINARY_NAME}"
  local helper_dest="${INSTALL_DIR}/${HELPER_NAME}"
  local bin_tmp="${bin_dest}.install.$$"
  local helper_tmp="${helper_dest}.install.$$"

  install -m 0755 "${tmp}/${BINARY_NAME}"       "${bin_tmp}"
  install -m 0755 "${tmp}/${HELPER_NAME}" "${helper_tmp}"

  mv -f "${bin_tmp}"    "${bin_dest}"
  mv -f "${helper_tmp}" "${helper_dest}"

  chmod 0755 "${bin_dest}" "${helper_dest}"
}

install_and_notify() {
  download_and_verify

  echo ""
  echo "✓ acdesktop v${VERSION} 安装成功 → ${INSTALL_DIR}"
  echo ""

  # 检查 PATH
  if [[ ":${PATH}:" == *":${INSTALL_DIR}:"* ]]; then
    echo "✓ ${INSTALL_DIR} 已在 PATH 中，可直接运行 acdesktop"
  else
    local shell_rc=""
    case "${SHELL:-}" in
      */zsh)  shell_rc="$HOME/.zshrc" ;;
      */bash) shell_rc="$HOME/.bashrc" ;;
      *)      shell_rc="$HOME/.profile" ;;
    esac
    echo "⚠ ${INSTALL_DIR} 不在 PATH 中，需要手动添加："
    echo ""
    echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ${shell_rc}"
    echo "  source ${shell_rc}"
  fi

  echo ""
  echo "运行 acdesktop version 验证安装。"
}

install_and_notify
