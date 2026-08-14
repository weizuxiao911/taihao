#!/bin/bash
# 拉取 RPi VideoCore 固件(运行时从 github,不入库)
#
# 产出:$TAIHAO_OUT/rpi-firmware/boot/{bootcode.bin, start_x.elf, fixup_x.dat, *.dtb, ...}
#
# 来源:https://github.com/raspberrypi/firmware/tree/stable(RPi OS 同源)
# 方式:tarball 下载(比 git clone 快得多,github 限流慢)
# 缓存:命中 $TAIHAO_OUT/rpi-firmware/boot/bootcode.bin 后跳过

set -euo pipefail

TAIHAO_OUT="${TAIHAO_OUT:-/home/weizuxiao.guest/taihao-out}"
FW_DIR="$TAIHAO_OUT/rpi-firmware"
BRANCH="${BRANCH:-stable}"
REPO_TARBALL="https://github.com/raspberrypi/firmware/archive/refs/heads/${BRANCH}.tar.gz"

# 缓存命中:已有 bootcode.bin 就跳过
if [ -f "$FW_DIR/boot/bootcode.bin" ]; then
    echo "fetch-rpi-firmware: 命中缓存 $FW_DIR/boot/bootcode.bin,跳过"
    exit 0
fi

echo "fetch-rpi-firmware: 下载 $REPO_TARBALL"
TMP_TGZ="$(mktemp)"
trap 'rm -f "$TMP_TGZ"' EXIT
curl -fSL -o "$TMP_TGZ" "$REPO_TARBALL" \
    || { echo "fetch-rpi-firmware: 下载失败,检查网络或 BRANCH=$BRANCH"; exit 1; }

mkdir -p "$FW_DIR"
# 解压只 boot/ 目录
tar -xzf "$TMP_TGZ" -C "$FW_DIR" --strip-components=1 --wildcards "firmware-${BRANCH}/boot/*"

[ -f "$FW_DIR/boot/bootcode.bin" ] \
    || { echo "fetch-rpi-firmware: 解压后未找到 bootcode.bin,放弃"; exit 1; }

echo "fetch-rpi-firmware: 落地 $FW_DIR/boot/ ($(du -sh "$FW_DIR/boot" | cut -f1),$(ls "$FW_DIR/boot/" | wc -l) 个文件)"
