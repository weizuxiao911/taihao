#!/bin/bash
# 太昊 OS · qemu 调试镜像(E2E debug · systemd log 全打串口)
#
# 与 qemu运行镜像.sh 的差异:
#   - 内核 cmdline 加 systemd.log_level=debug systemd.log_target=console
#   - 串口输出会非常冗长(systemd 启动期所有内部日志)
#   - 同时输出到本终端和 out/arm64-e2e/logs/serial-e2e-debug.log
#
# 用法:
#   scripts/run/qemu调试镜像.sh
#
# 适用场景:
#   - dbus.service 等 systemd 服务 FAILED 时,定位真实原因
#   - 内核 panic / oops 时,查看内核日志

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT="$REPO_ROOT/out/arm64-e2e"
IMAGE="$OUT/Image"
INITRAMFS="$OUT/initramfs.cpio"
STATE="$OUT/snap/boot-snap.qcow2"
LOG_DIR="$OUT/logs"
HOST_SHARE="$REPO_ROOT"
DEBUG_LOG="$LOG_DIR/serial-e2e-debug.log"
mkdir -p "$LOG_DIR"

command -v qemu-system-aarch64 >/dev/null 2>&1 || { echo "缺 qemu-system-aarch64"; exit 2; }
for f in "$IMAGE" "$INITRAMFS" "$STATE"; do [ -f "$f" ] || { echo "缺文件: $f"; exit 2; }; done

rm -f "$LOG_DIR/hmp-sock" "$DEBUG_LOG"

echo "=== 太昊 OS · qemu 调试镜像 (E2E debug) ==="
echo "  Image:     $IMAGE"
echo "  initramfs: $INITRAMFS"
echo "  debug log: $DEBUG_LOG"
echo

qemu-system-aarch64 \
    -M virt \
    -cpu cortex-a72 \
    -m 4G \
    -smp 4 \
    -kernel "$IMAGE" \
    -initrd "$INITRAMFS" \
    -append "console=ttyAMA0 rdinit=/sbin/init loglevel=4 systemd.log_level=debug systemd.log_target=console" \
    -display cocoa \
    -serial mon:stdio \
    -monitor unix:"$LOG_DIR/hmp-sock",server,nowait \
    -no-reboot \
    -drive file="$STATE",if=virtio,format=qcow2 \
    -virtfs local,path="$HOST_SHARE",mount_tag=hostshare,security_model=none,id=hostshare \
    -device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare \
    2>&1 | tee "$DEBUG_LOG"
