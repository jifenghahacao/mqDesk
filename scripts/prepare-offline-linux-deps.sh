#!/bin/bash
# 准备 MQDesk Linux 离线安装依赖包
# 用法：在能联网的 Ubuntu/Debian/麒麟机器上运行，会生成一个包含所有依赖 .deb 的目录
#       bash scripts/prepare-offline-linux-deps.sh
#
# 说明：
#   - 本脚本通过 apt-rdepends 递归解析 MQDesk .deb 的依赖树，并下载所有 .deb。
#   - 下载完成后，将本目录与 apk/MQDesk_0.1.0_amd64.deb 一起拷贝到离线机器，
#     运行 scripts/install-offline-linux.sh 即可安装。
#   - 必须在目标系统相同版本（如 Ubuntu 24.04 / 麒麟 V10）的联网机器上执行，
#     否则依赖包版本可能不匹配。

set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEB_FILE="$PROJECT_DIR/apk/MQDesk_0.1.0_amd64.deb"
OUTPUT_DIR="$PROJECT_DIR/apk/linux-offline"

if ! command -v dpkg-deb &> /dev/null; then
  echo "错误：未找到 dpkg-deb，本脚本必须在 Debian/Ubuntu/麒麟系统上运行。"
  exit 1
fi

if [ ! -f "$DEB_FILE" ]; then
  echo "错误：未找到 $DEB_FILE"
  echo "请先构建 Linux .deb 包，或从发布页下载。"
  exit 1
fi

if ! command -v apt-rdepends &> /dev/null; then
  echo "[1/3] 安装 apt-rdepends..."
  apt-get update
  apt-get install -y apt-rdepends
fi

mkdir -p "$OUTPUT_DIR"
cd "$OUTPUT_DIR"

echo "[2/3] 解析 MQDesk 依赖树..."
DEPENDS=$(dpkg-deb -f "$DEB_FILE" Depends | tr ',' '\n' | sed 's/([^)]*)//g' | awk '{print $1}' | sort -u)
echo "直接依赖："
echo "$DEPENDS"

echo ""
echo "递归依赖（含推荐依赖）..."
# -p 表示 print；过滤掉以空格开头的反向依赖行，并去重
apt-rdepends -p $DEPENDS 2>/dev/null | grep -v '^ ' | sort -u > packages.list
TOTAL=$(wc -l < packages.list)
echo "共 $TOTAL 个包需要准备"

echo "[3/3] 下载所有依赖包（跳过虚包/系统已预装包）..."
FAILED_LOG="failed-packages.log"
> "$FAILED_LOG"

for pkg in $(cat packages.list); do
  # 跳过明确无法下载的虚包
  if apt-cache show "$pkg" &>/dev/null; then
    if apt-get download "$pkg" &>/dev/null; then
      echo "  ✓ $pkg"
    else
      echo "  ✗ $pkg (下载失败)" >> "$FAILED_LOG"
    fi
  else
    echo "  ✗ $pkg (无候选版本/虚包)" >> "$FAILED_LOG"
  fi
done

cp "$DEB_FILE" .

echo ""
echo "✅ 离线安装包准备完成：$OUTPUT_DIR"
echo "   共 $(ls -1 *.deb | wc -l) 个 .deb 文件"
echo "   总大小：$(du -sh . | cut -f1)"
echo ""
echo "离线机器安装步骤："
echo "   1. 将整个 $OUTPUT_DIR 目录拷贝到离线机器"
echo "   2. cd 到该目录"
echo "   3. 执行：sudo dpkg -i *.deb"
echo "   4. 若仍有依赖缺失：sudo apt-get install -f"
