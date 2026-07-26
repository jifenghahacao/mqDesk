#!/bin/bash
# 在 WSL2 的 Ubuntu/Debian 中运行，自动编译 MQDesk 的 Linux .deb 安装包
# 用法：cd /mnt/d/project/RabbitConsumerHub-main && bash scripts/build-linux-deb.sh

set -e

# 中国大陆网络加速（不影响其他环境）
export RUSTUP_UPDATE_ROOT="${RUSTUP_UPDATE_ROOT:-https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup}"
export RUSTUP_DIST_SERVER="${RUSTUP_DIST_SERVER:-https://mirrors.tuna.tsinghua.edu.cn/rustup}"
export NPM_CONFIG_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmmirror.com}"

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "[1/5] 更新系统并安装依赖..."
sudo apt-get update
sudo apt-get install -y \
  curl \
  build-essential \
  libssl-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  pkg-config \
  file

echo "[2/5] 安装/更新 Node.js 22..."
if ! command -v node &> /dev/null || [ "$(node -v | cut -d'v' -f2 | cut -d'.' -f1)" != "22" ]; then
  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
  sudo apt-get install -y nodejs
fi

echo "[3/5] 安装/更新 Rust..."
source "$HOME/.cargo/env" 2>/dev/null || true
if ! command -v cargo &> /dev/null; then
  RUSTUP_INIT_URL="${RUSTUP_DIST_SERVER}/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
  echo "下载 rustup-init: $RUSTUP_INIT_URL"
  curl -fsSL --ipv4 --connect-timeout 30 --max-time 300 "$RUSTUP_INIT_URL" -o /tmp/rustup-init
  chmod +x /tmp/rustup-init
  /tmp/rustup-init -y --default-toolchain stable
  source "$HOME/.cargo/env"
fi
rustup update
rustup target add x86_64-unknown-linux-gnu

echo "[4/5] 安装前端依赖并构建..."
cd "$PROJECT_DIR"
npm install
npm run build

echo "[5/5] 编译 Tauri Linux .deb 包..."
cd "$PROJECT_DIR/src-tauri"
cargo install tauri-cli --version "^2.0" --locked
cargo tauri build --target x86_64-unknown-linux-gnu

echo ""
echo "✅ Linux 安装包已生成："
find "$PROJECT_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb" -name "*.deb" -type f
