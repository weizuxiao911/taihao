#!/bin/bash
# 端到端启动脚本:挂 virtio-9p 共享,支持 savevm/loadvm 快照
#
# 用法:
#   scripts/kernel-build/qemu-start-e2e.sh [options]
#   scripts/kernel-build/qemu-start-e2e.sh save-snap | load-snap | reset-snap
#
# 选项:
#   --image <path>        内核 Image(默认 out/arm64-e2e/Image)
#   --initramfs <path>    initramfs(默认 out/arm64-e2e/initramfs.cpio)
#   --host-share <path>   主机目录共享到 /mnt/host(默认仓库根)
#   --memory <size>       内存(默认 4G)
#   --cpus <N>            CPU 数(默认 4)
#   --snapshot <name>     启动时 load 快照(默认 boot-snap)
#   --no-snapshot         禁用快照
#   --save-snap           启动后保存快照
#   --timeout <sec>       basic.target 超时(默认 60)
#
# 子命令:
#   save-snap             保存当前快照(对运行中的 qemu 发 HMP 命令)
#   load-snap             加载快照
#   reset-snap            删除快照
#
# 退出码:
#   0 = 成功
#   1 = 参数错误
#   2 = qemu 启动失败
#   3 = basic.target 超时

set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/lib" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"
# shellcheck disable=SC1091
source "$_LIB_DIR/probe.sh"
# shellcheck disable=SC1091
source "$_LIB_DIR/snapshot.sh"

# ===== 子命令 =====
SNAP_DIR_E2E="$KERNEL_BUILD_OUT/arm64-e2e/snap"
SNAP_NAME_E2E="boot-snap"
SNAP_STATE_E2E="$SNAP_DIR_E2E/${SNAP_NAME_E2E}.qcow2"
HMP_SOCK_E2E="$KERNEL_BUILD_OUT/arm64-e2e/logs/hmp-sock"
IMG_E2E="$KERNEL_BUILD_OUT/arm64-e2e/Image"
INITRAMFS_E2E="$KERNEL_BUILD_OUT/arm64-e2e/initramfs.cpio"

case "${1:-}" in
    save-snap|load-snap|reset-snap)
        case "$1" in
            save-snap)
                snapshot_save \
                    "$SNAP_DIR_E2E" "$SNAP_NAME_E2E" \
                    "$IMG_E2E" "$INITRAMFS_E2E" \
                    "$SNAP_STATE_E2E" "$HMP_SOCK_E2E"
                exit $? ;;
            load-snap)
                snapshot_load \
                    "$SNAP_DIR_E2E" "$SNAP_NAME_E2E" \
                    "$IMG_E2E" "$INITRAMFS_E2E" \
                    "$SNAP_STATE_E2E" "$HMP_SOCK_E2E"
                exit $? ;;
            reset-snap)
                snapshot_reset "$SNAP_DIR_E2E" "$SNAP_NAME_E2E" "$SNAP_STATE_E2E"
                exit $? ;;
        esac
        ;;
esac

# ===== 参数 =====
IMAGE=""
INITRAMFS=""
HOST_SHARE=""
MEMORY="4G"
CPUS=4
TIMEOUT=60
USE_SNAPSHOT=1
SNAP_NAME="boot-snap"
DO_SAVE_SNAP=0

usage() {
    cat >&2 <<EOF
用法: $0 [options]
       $0 save-snap | load-snap | reset-snap

选项:
  --image <path>        内核 Image
  --initramfs <path>    initramfs
  --host-share <path>   主机共享目录(默认仓库根)
  --memory <size>       内存(默认 4G)
  --cpus <N>            CPU 数(默认 4)
  --snapshot <name>     启动 load 快照(默认 boot-snap)
  --no-snapshot         禁用快照
  --save-snap           启动后保存快照
  --timeout <sec>       basic.target 超时(默认 60)

子命令:
  save-snap             保存当前快照
  load-snap             加载快照
  reset-snap            删除快照

guest 内挂载(virtio-9p mount_tag=hostshare → /mnt/host):
  guest 内执行:mount -t 9p hostshare /mnt/host -o trans=virtio
  或在 systemd unit 里挂载(fstab 或 .mount 单元)
EOF
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --image)        IMAGE="$2"; shift 2 ;;
        --initramfs)    INITRAMFS="$2"; shift 2 ;;
        --host-share)   HOST_SHARE="$2"; shift 2 ;;
        --memory)       MEMORY="$2"; shift 2 ;;
        --cpus)         CPUS="$2"; shift 2 ;;
        --snapshot)     USE_SNAPSHOT=1; SNAP_NAME="$2"; shift 2 ;;
        --no-snapshot)  USE_SNAPSHOT=0; shift ;;
        --save-snap)    DO_SAVE_SNAP=1; shift ;;
        --timeout)      TIMEOUT="$2"; shift 2 ;;
        -h|--help)      usage ;;
        *)              die "未知选项: $1" ;;
    esac
done

# ===== 默认值 =====
IMAGE="${IMAGE:-$KERNEL_BUILD_OUT/arm64-e2e/Image}"
INITRAMFS="${INITRAMFS:-$KERNEL_BUILD_OUT/arm64-e2e/initramfs.cpio}"
HOST_SHARE="${HOST_SHARE:-$KERNEL_BUILD_REPO_ROOT}"

require_file "$IMAGE" "Image"
require_file "$INITRAMFS" "initramfs"
require_dir "$HOST_SHARE" "共享目录"

require_cmd qemu-system-aarch64

# ===== 启动 =====
LOG_DIR="$KERNEL_BUILD_OUT/arm64-e2e/logs"
STATE_IMG="$SNAP_STATE_E2E"

mkdir -p "$LOG_DIR" "$SNAP_DIR_E2E"
SERIAL_LOG="$LOG_DIR/serial.log"

info "=== E2E 启动 ==="
info "  Image: $IMAGE"
info "  initramfs: $INITRAMFS"
info "  共享目录: $HOST_SHARE → /mnt/host"
info "  内存: $MEMORY / CPU: $CPUS"

rm -f "$SERIAL_LOG" "$LOG_DIR/hmp-sock"

# 状态盘存在性检查
if [ ! -f "$STATE_IMG" ]; then
    die "状态盘不存在: $STATE_IMG;先跑 build-kernel.sh e2e 生成"
fi

# 构造 qemu 命令(含 virtio-9p 共享 + 状态盘)
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
    -drive file="$STATE_IMG",if=virtio,format=qcow2
    -virtfs local,path="$HOST_SHARE",mount_tag=hostshare,security_model=none,id=hostshare
    -device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare
)

# 加载快照(若启用)
SNAP_LOADED=0
if [ $USE_SNAPSHOT -eq 1 ] && snapshot_is_valid "$SNAP_DIR_E2E" "$SNAP_NAME" "$IMAGE" "$INITRAMFS" "$STATE_IMG"; then
    info "快照有效,加载: $SNAP_NAME"
    QEMU_ARGS+=(-loadvm "$SNAP_NAME")
    SNAP_LOADED=1
elif [ $USE_SNAPSHOT -eq 1 ]; then
    warn "快照失效,丢弃旧快照+状态盘并重新冷启"
    snapshot_reset "$SNAP_DIR_E2E" "$SNAP_NAME" "$STATE_IMG"
fi

info "qemu 启动中..."
qemu-system-aarch64 "${QEMU_ARGS[@]}" &
QEMU_PID=$!
# trap 只清 hmp-sock,不杀 QEMU(让 QEMU 持续后台运行,支持后续 save-snap/load-snap)
trap 'rm -f "$LOG_DIR/hmp-sock"' EXIT

sleep 2

# ===== 等待 basic.target =====
PROBE_TIMEOUT_SEC="$TIMEOUT"
if ! probe_basic_target "$SERIAL_LOG"; then
    warn "basic.target 未在 ${TIMEOUT}s 内达成"
    # basic.target 未达成时仍保留 QEMU 便于调试;不主动 kill
    info "  qemu PID: $QEMU_PID(调试用,需手动 kill)"
    exit 3
fi

info "basic.target 达成"

# ===== 保存快照 =====
if [ $DO_SAVE_SNAP -eq 1 ]; then
    if [ $SNAP_LOADED -eq 1 ]; then
        warn "已从快照启动,跳过 save-snap(避免覆盖)"
    else
        snapshot_save "$SNAP_DIR_E2E" "$SNAP_NAME" "$IMAGE" "$INITRAMFS" "$STATE_IMG" "$LOG_DIR/hmp-sock"
    fi
fi

info "E2E 启动完成(后台 QEMU 持续运行)"
info "  qemu PID: $QEMU_PID"
info "  串口日志: $SERIAL_LOG"
info "  HMP socket: $LOG_DIR/hmp-sock"
info "  停止 QEMU: kill $QEMU_PID"

exit 0