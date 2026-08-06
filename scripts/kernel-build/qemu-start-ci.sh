#!/bin/bash
# CI 冒烟启动脚本:不带 9p/virtio-fs/状态盘/快照,等待 systemd basic.target 达成
#
# 用法:
#   scripts/kernel-build/qemu-start-ci.sh [options]
#
# 选项:
#   --image <path>       内核 Image(默认 out/arm64-ci/Image)
#   --initramfs <path>   initramfs.cpio(默认 out/arm64-ci/initramfs.cpio)
#   --memory <size>      qemu 内存大小(默认 2G)
#   --cpus <N>           qemu CPU 数(默认 2)
#   --timeout <sec>      basic.target 等待超时(默认 30s)
#
# 退出码:
#   0 = basic.target 达成
#   1 = 参数错误
#   2 = qemu 启动失败
#   3 = basic.target 超时

set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/lib" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"
# shellcheck disable=SC1091
source "$_LIB_DIR/probe.sh"

# ===== 参数 =====
IMAGE=""
INITRAMFS=""
MEMORY="2G"
CPUS=2
TIMEOUT=30

usage() {
    cat >&2 <<EOF
用法: $0 [options]

选项:
  --image <path>       内核 Image(默认 out/arm64-ci/Image)
  --initramfs <path>   initramfs(默认 out/arm64-ci/initramfs.cpio)
  --memory <size>      qemu 内存(默认 2G)
  --cpus <N>           CPU 数(默认 2)
  --timeout <sec>      basic.target 超时(默认 30)

环境变量:
  KERNEL_BUILD_OUT     产出根目录

注:CI 不挂 9p / virtio-fs(契约红线 4);不创建状态盘 / 快照。
EOF
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --image)        IMAGE="$2"; shift 2 ;;
        --initramfs)    INITRAMFS="$2"; shift 2 ;;
        --memory)       MEMORY="$2"; shift 2 ;;
        --cpus)         CPUS="$2"; shift 2 ;;
        --timeout)      TIMEOUT="$2"; shift 2 ;;
        -h|--help)      usage ;;
        *)              die "未知选项: $1" ;;
    esac
done

# ===== 默认值 =====
IMAGE="${IMAGE:-$KERNEL_BUILD_OUT/arm64-ci/Image}"
INITRAMFS="${INITRAMFS:-$KERNEL_BUILD_OUT/arm64-ci/initramfs.cpio}"

require_file "$IMAGE" "Image"
require_file "$INITRAMFS" "initramfs"

require_cmd qemu-system-aarch64

# ===== 启动 =====
LOG_DIR="$KERNEL_BUILD_OUT/arm64-ci/logs"
mkdir -p "$LOG_DIR"
SERIAL_LOG="$LOG_DIR/serial.log"

info "=== CI 冒烟启动 ==="
info "  Image: $IMAGE"
info "  initramfs: $INITRAMFS"
info "  内存: $MEMORY / CPU: $CPUS / 超时: $TIMEOUT"

rm -f "$SERIAL_LOG" "$LOG_DIR/hmp-sock"

# CI 固定 -nographic(无图形界面),不挂 9p/virtio-fs,无状态盘
QEMU_ARGS=(
    -M virt
    -cpu cortex-a72
    -m "$MEMORY"
    -smp "$CPUS"
    -kernel "$IMAGE"
    -initrd "$INITRAMFS"
    -append "console=ttyAMA0 earlycon=pl011,0x0900000 root=/dev/ram rdinit=/bin/systemd systemd.unified_cgroup_hierarchy=1 loglevel=4"
    -nographic
    -serial "file:$SERIAL_LOG"
    -monitor unix:$LOG_DIR/hmp-sock,server,nowait
    -no-reboot
)

info "qemu 启动中..."
qemu-system-aarch64 "${QEMU_ARGS[@]}" &
QEMU_PID=$!
# trap 只清 hmp-sock;CI 用例启动后立即 kill(等 basic.target 达成)
trap 'rm -f "$LOG_DIR/hmp-sock"' EXIT

# ===== 等待 basic.target =====
PROBE_TIMEOUT_SEC="$TIMEOUT"
if probe_basic_target "$SERIAL_LOG"; then
    info "CI 冒烟通过"
    kill "$QEMU_PID" 2>/dev/null || true
    exit 0
else
    warn "CI 冒烟失败:basic.target 未达成"
    kill "$QEMU_PID" 2>/dev/null || true
    tail -20 "$SERIAL_LOG" >&2 || true
    exit 3
fi