#!/bin/bash
# 太昊 OS · qemu 运行镜像(E2E 验收版 · cocoa 窗口)
#
# 与 scripts/kernel-build/qemu-start-e2e.sh 的差异:
#   - display=cocoa(本机可见窗口,而非 -display none)
#   - serial=mon:stdio(串口绑定当前终端,而非 file:...)
#
# 用法:
#   scripts/run/qemu运行镜像.sh                  # 默认启动
#   scripts/run/qemu运行镜像.sh --snapshot NAME  # 启动时 load 快照
#   scripts/run/qemu运行镜像.sh --save-snap      # 启动后保存快照
#   scripts/run/qemu运行镜像.sh --memory 8G      # 指定内存
#
# 退出:
#   Ctrl-A X 退出 qemu  |  kill <pid> 强杀
#
# 依赖:
#   - qemu-system-aarch64 (brew install qemu)
#   - 已构建产物:out/arm64-e2e/{Image, initramfs.cpio, snap/boot-snap.qcow2}

set -euo pipefail

# ===== 路径 =====
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT="$REPO_ROOT/out/arm64-e2e"
IMAGE="$OUT/Image"
INITRAMFS="$OUT/initramfs.cpio"
STATE="$OUT/snap/boot-snap.qcow2"
LOG_DIR="$OUT/logs"
HOST_SHARE="$REPO_ROOT"
mkdir -p "$LOG_DIR"

# ===== 参数 =====
USE_SNAPSHOT=1
SNAP_NAME="boot-snap"
DO_SAVE_SNAP=0
MEMORY="4G"
CPUS=4

usage() {
    cat >&2 <<EOF
用法: $0 [options]

选项:
  --snapshot NAME    启动时 load 快照(默认 boot-snap)
  --no-snapshot      禁用快照(冷启)
  --save-snap        启动后保存快照
  --memory SIZE      qemu 内存(默认 4G)
  --cpus N           qemu CPU 数(默认 4)
  --host-share PATH  9p 共享目录(默认仓库根)
  -h | --help        本帮助
EOF
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --snapshot)     USE_SNAPSHOT=1; SNAP_NAME="$2"; shift 2 ;;
        --no-snapshot)  USE_SNAPSHOT=0; shift ;;
        --save-snap)    DO_SAVE_SNAP=1; shift ;;
        --memory)       MEMORY="$2"; shift 2 ;;
        --cpus)         CPUS="$2"; shift 2 ;;
        --host-share)   HOST_SHARE="$2"; shift 2 ;;
        -h|--help)      usage ;;
        *)              echo "未知选项: $1"; usage ;;
    esac
done

# ===== 校验 =====
require_file() { [ -f "$1" ] || { echo "缺少文件: $1"; exit 2; }; }
require_cmd()  { command -v "$1" >/dev/null 2>&1 || { echo "缺少命令: $1"; exit 2; }; }

require_cmd qemu-system-aarch64
require_file "$IMAGE"
require_file "$INITRAMFS"
require_file "$STATE"

# ===== 启动 =====
SERIAL_LOG="$LOG_DIR/serial-e2e.log"
HMP_SOCK="$LOG_DIR/hmp-sock"

rm -f "$SERIAL_LOG" "$HMP_SOCK"
touch "$SERIAL_LOG"

echo "=== 太昊 OS · qemu 运行镜像 (E2E 验收) ==="
echo "  Image:     $IMAGE ($(du -h "$IMAGE" | cut -f1))"
echo "  initramfs: $INITRAMFS ($(du -h "$INITRAMFS" | cut -f1))"
echo "  state:     $STATE"
echo "  共享:       $HOST_SHARE → /mnt/host (guest)"
echo "  内存/CPU:   $MEMORY / $CPUS"
echo "  串口:       本终端(按 Ctrl-A X 退出 qemu)"
echo

QEMU_ARGS=(
    -M virt
    -cpu cortex-a72
    -m "$MEMORY"
    -smp "$CPUS"
    -kernel "$IMAGE"
    -initrd "$INITRAMFS"
    -append "console=ttyAMA0 rdinit=/sbin/init loglevel=4"
    -display cocoa
    -serial mon:stdio
    -monitor unix:"$HMP_SOCK",server,nowait
    -no-reboot
    -drive file="$STATE",if=virtio,format=qcow2
    -virtfs local,path="$HOST_SHARE",mount_tag=hostshare,security_model=none,id=hostshare
    -device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare
)

if [ $USE_SNAPSHOT -eq 1 ] && [ -f "$LOG_DIR/${SNAP_NAME}.hashes" ]; then
    echo "[info] 加载快照: $SNAP_NAME"
    QEMU_ARGS+=(-loadvm "$SNAP_NAME")
fi

qemu-system-aarch64 "${QEMU_ARGS[@]}"
