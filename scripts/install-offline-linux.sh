#!/bin/bash
# MQDesk Linux 离线安装脚本
# 用法：cd 到包含本脚本和所有 .deb 文件的目录，然后执行
#       sudo bash install-offline-linux.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MAIN_DEB="MQDesk_0.1.0_amd64.deb"

cd "$SCRIPT_DIR"

if [ "$EUID" -ne 0 ]; then
  echo "错误：请使用 sudo 或以 root 身份运行本脚本。"
  exit 1
fi

if [ ! -f "$MAIN_DEB" ]; then
  echo "错误：当前目录未找到 $MAIN_DEB"
  echo "请确保已将离线安装包目录完整拷贝到本机。"
  exit 1
fi

DEB_COUNT=$(ls -1 *.deb 2>/dev/null | wc -l)
echo "发现 $DEB_COUNT 个 .deb 安装包"

echo ""
echo "开始安装 MQDesk 及其依赖..."
dpkg -i *.deb

echo ""
echo "修复可能的依赖关系..."
apt-get install -f -y || true

echo ""
echo "✅ MQDesk 离线安装完成"
echo "   可在应用菜单搜索 'MQDesk' 启动"
echo "   或在终端运行：mqdesk"
