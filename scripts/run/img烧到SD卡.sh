#!/bin/bash
# 太昊 OS · img 烧录到 SD 卡(参数化平台)
#
# 烧 SD 卡前的必备:
#   1. SD 卡插入 Mac,diskutil list 找到设备号(e.g. /dev/disk4)
#   2. 平台对应的 img 已构建:out/taihao-{platform}.img
#
# 用法:
#   scripts/run/img烧到SD卡.sh qemu                # qemu 不真烧,只检查 img
#   scripts/run/img烧到SD卡.sh rpi4b /dev/disk4    # dd 写入 SD 卡
#   scripts/run/img烧到SD卡.sh rpi3bp /dev/diskN
#
# �️ 危险操作:dd 会覆盖目标设备所有数据
#
# 退出:
#   0 = 烧录成功(或者 qemu 平台校验通过)
#   1 = 用户取消
#   2 = 缺依赖 / 校验失败

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT="$REPO_ROOT/out"

PLATFORM="${1:-}"
DEVICE="${2:-}"

usage() {
    cat >&2 <<EOF
用法: $0 <platform> [device]

platform:
  qemu       校验 taihao-qemu.img(不真烧,QEMU 用 img启动qemu.sh)
  rpi4b      烧 taihao-rpi4b.img 到 SD 卡(真机路演)
  rpi3bp     烧 taihao-rpi3bp.img 到 SD 卡(真机路演)

device:
  SD 卡设备(e.g. /dev/disk4 macOS 或 /dev/sdb Linux),仅 rpi4b/rpi3bp 需要
  �️  dd 会覆盖目标设备所有数据!

示例:
  $0 qemu
  $0 rpi4b /dev/disk4
  $0 rpi3bp /dev/diskN
EOF
    exit 1
}

require_file() { [ -f "$1" ] || { echo "缺文件: $1"; exit 2; }; }
require_cmd()  { command -v "$1" >/dev/null 2>&1 || { echo "缺命令: $1"; exit 2; }; }

[ -z "$PLATFORM" ] && usage

case "$PLATFORM" in
    qemu)
        IMG="$OUT/taihao-qemu.img"
        require_file "$IMG"
        echo "img烧到SD卡: qemu 平台,只需校验 img 存在"
        echo "  $IMG ($(du -h "$IMG" | cut -f1))"
        echo "  启动方式: scripts/run/img启动qemu.sh"
        ;;
    rpi4b|rpi3bp)
        IMG="$OUT/taihao-${PLATFORM}.img"
        [ -z "$DEVICE" ] && { echo "缺 device 参数"; usage; }
        require_file "$IMG"
        require_cmd dd

        echo "img烧到SD卡: $PLATFORM → $DEVICE"
        echo "  img:     $IMG ($(du -h "$IMG" | cut -f1))"
        echo "  device:  $DEVICE"
        echo
        echo "⚠️  这一步会覆盖 $DEVICE 上所有数据"
        read -rp "确认继续?(yes/no): " ans
        [ "$ans" = "yes" ] || { echo "取消"; exit 1; }

        # macOS 上 dd /dev/rdiskN 比 /dev/diskN 快(不走 buffer cache)
        case "$DEVICE" in
            /dev/disk*) RAW="/dev/r${DEVICE#/dev/disk}" ;;
            /dev/rdisk*) RAW="$DEVICE" ;;
            /dev/sd*|/dev/mmcblk*) RAW="$DEVICE" ;;
            *) echo "未知 device 类型: $DEVICE(支持 /dev/diskN /dev/rdiskN /dev/sdN /dev/mmcblkN)"; exit 2 ;;
        esac

        echo "  dd if=$IMG of=$RAW bs=4m status=progress ..."
        sudo dd if="$IMG" of="$RAW" bs=4m status=progress conv=fsync
        sync
        echo
        echo "img烧到SD卡: 完成(请弹出 SD 卡:diskutil eject $DEVICE)"
        ;;
    *)
        echo "未知平台: $PLATFORM(支持:qemu|rpi4b|rpi3bp)"
        usage
        ;;
esac
