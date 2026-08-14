#!/bin/bash
# 太昊 OS · QEMU 启动 SD 卡镜像(替代旧版 -kernel + -initrd 直启)
#
# 用 SD 卡 img 模拟真机启动链(同 RPi / RK3588 路径)
# 内核仍走 -kernel + -initrd(Image / initrd 也在 img 内,QEMU 不读,留着复用)
# rootfs 通过 SD 卡 img 挂载
#
# 用法:
#   scripts/run/img启动qemu.sh                  # 默认 taihao-qemu.img
#   scripts/run/img启动qemu.sh --image PATH    # 指定 img 路径
#   scripts/run/img启动qemu.sh --no-snapshot   # 冷启
#
# 退出:
#   qemu 同步退出
#   Ctrl-A X 退出 qemu

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT="$REPO_ROOT/out"
IMG="$OUT/taihao-qemu.img"
STATE="$OUT/arm64-e2e/snap/boot-snap.qcow2"
LOG_DIR="$OUT/arm64-e2e/logs"

USE_SNAPSHOT=0
MEMORY="4G"
CPUS=4

usage() {
    cat >&2 <<EOF
用法: $0 [options]

选项:
  --image PATH    SD 卡 img 路径(默认 $IMG)
  --no-snapshot   禁用快照
  --memory SIZE   qemu 内存(默认 4G)
  --cpus N        qemu CPU 数(默认 4)
  -h | --help     本帮助
EOF
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --image)        IMG="$2"; shift 2 ;;
        --no-snapshot)  USE_SNAPSHOT=0; shift ;;
        --memory)       MEMORY="$2"; shift 2 ;;
        --cpus)         CPUS="$2"; shift 2 ;;
        -h|--help)      usage ;;
        *)              echo "未知选项: $1"; usage ;;
    esac
done

require_file() { [ -f "$1" ] || { echo "缺少文件: $1"; exit 2; }; }
require_cmd()  { command -v "$1" >/dev/null 2>&1 || { echo "缺少命令: $1"; exit 2; }; }

require_cmd qemu-system-aarch64
require_file "$IMG"
mkdir -p "$LOG_DIR"

# 内核 + initrd 路径(从 SD 卡 img 内 boot 分区拆出,也可从 arm64-e2e 拿)
KERN="$OUT/arm64-e2e/Image"
INITRD="$OUT/arm64-e2e/initramfs.cpio"
require_file "$KERN"
require_file "$INITRD"

rm -f "$LOG_DIR/serial-sd.log" "$LOG_DIR/hmp-sock"

echo "=== 太昊 OS · qemu 启动 SD 卡 img ==="
echo "  img:     $IMG ($(du -h "$IMG" | cut -f1))"
echo "  Image:   $KERN"
echo "  initrd:  $INITRD"
echo "  memory:  $MEMORY / cpus: $CPUS"
echo

qemu-system-aarch64 \
    -M virt \
    -cpu cortex-a72 \
    -m "$MEMORY" \
    -smp "$CPUS" \
    -kernel "$KERN" \
    -initrd "$INITRD" \
    -append "console=ttyAMA0 rdinit=/sbin/init loglevel=4 root=/dev/vda2 rootfstype=ext4" \
    -display cocoa \
    -serial mon:stdio \
    -monitor unix:"$LOG_DIR/hmp-sock",server,nowait \
    -no-reboot \
    -drive file="$IMG",if=virtio,format=raw \
    -virtfs local,path="$REPO_ROOT",mount_tag=hostshare,security_model=none,id=hostshare \
    -device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare
