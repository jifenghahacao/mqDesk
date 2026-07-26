#!/bin/bash
# 在 WSL2 的 Ubuntu/Debian 中运行，在 WSL 原生文件系统中编译 MQDesk 的 Linux AppImage 安装包
# 解决 /mnt/d/ 下 AppImage 构建容易因文件系统忙/慢而卡住的问题

set -e

export RUSTUP_UPDATE_ROOT="${RUSTUP_UPDATE_ROOT:-https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup}"
export RUSTUP_DIST_SERVER="${RUSTUP_DIST_SERVER:-https://mirrors.tuna.tsinghua.edu.cn/rustup}"
export NPM_CONFIG_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmmirror.com}"

PROJECT_DIR="/mnt/d/project/RabbitConsumerHub-main"
BUILD_DIR="/root/mqdesk-build"
LOG_FILE="/root/mqdesk-build.log"

echo "[appimage] 开始构建，日志保存到 $LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

if [ -d "$BUILD_DIR" ]; then
  echo "[appimage] 清理旧构建目录..."
  rm -rf "$BUILD_DIR"
fi

echo "[1/5] 同步项目到 WSL 原生构建目录: $BUILD_DIR"
rsync -a --delete \
  --exclude=target \
  --exclude=node_modules \
  --exclude=.git \
  --exclude=apk \
  "$PROJECT_DIR/" "$BUILD_DIR/"

echo "[2/5] 安装前端依赖..."
cd "$BUILD_DIR"
npm install --ignore-scripts

echo "[3/5] 编译 release binary..."
cd "$BUILD_DIR/src-tauri"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release --target x86_64-unknown-linux-gnu

echo "[4/5] 编译 AppImage..."
cargo tauri bundle -b appimage --target x86_64-unknown-linux-gnu

echo "[5/5] 复制 AppImage 回项目目录..."
mkdir -p "$PROJECT_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage"
cp "$BUILD_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage/"*.AppImage \
   "$PROJECT_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage/"

echo ""
echo "✅ Linux AppImage 安装包已生成："
ls -lh "$PROJECT_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage/"*.AppImage
