#!/bin/bash
# save-snap / load-snap 封装 + 快照失效判定
#
# 用法:source "$(dirname "${BASH_SOURCE[0]}")/snapshot.sh"
# 协议:HMP 文本协议(通过 -monitor unix:... socket,文本 savevm/loadvm 命令)
# 所有路径通过参数显式传入,避免依赖环境变量默认路径

set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"

# ===== hash 计算 =====
# 注意:state_img 内含 savevm 写入的 vmstate,save 后会变化,不能再作为 hash 因子
# 只用 Image + initramfs 作为快照失效判定的输入
snapshot_compute_hash() {
    local img="$1" initramfs="$2"
    {
        [ -n "$img" ] && [ -f "$img" ] && sha256sum "$img" || echo "noimg"
        [ -n "$initramfs" ] && [ -f "$initramfs" ] && sha256sum "$initramfs" || echo "noinit"
    } | sha256sum | awk '{print $1}'
}

# ===== 失效判定 =====
# 用法:snapshot_is_valid <snap_dir> <snap_name> <img> <initramfs> <state_img>
# state_img 不参与 hash(只 Image + initramfs 决定快照是否适用)
snapshot_is_valid() {
    local snap_dir="$1" snap_name="$2" img="$3" initramfs="$4" state_img="$5"
    local hash_file="$snap_dir/${snap_name}.hashes"
    local current stored
    current=$(snapshot_compute_hash "$img" "$initramfs")
    if [ -f "$hash_file" ]; then
        stored=$(cat "$hash_file")
        [ "$current" = "$stored" ] && return 0
    fi
    return 1
}

# ===== 状态盘创建 =====
snapshot_create_state_img() {
    local state_img="$1"
    require_cmd qemu-img
    mkdir -p "$(dirname "$state_img")"
    if [ ! -f "$state_img" ]; then
        qemu-img create -q -f qcow2 "$state_img" 256M >/dev/null
        info "状态盘已创建: $state_img"
    fi
}

# ===== HMP 通信 =====
# 用法:hmp_command <hmp_socket> <words...>
# 例:hmp_command "$SOCK" savevm boot-snap
# 注:QEMU HMP 单条命令必须在同一行,以空格分隔词;不能用换行
hmp_command() {
    local sock="$1"; shift
    [ -S "$sock" ] || die "HMP socket 不存在: $sock"
    # SIGPIPE 安全:某些 nc 实现可能在 printf 关闭时触发 SIGPIPE,加 || true
    printf '%s\n' "$*" | nc -U -w 5 "$sock" >/dev/null 2>&1 || \
        warn "HMP 命令发送失败:$*"
}

# ===== save / load / reset =====
# 注:snapshot_reset 只清 hash 文件,不删状态盘(qcow2);状态盘独立生命周期管理
# 用法:snapshot_save <snap_dir> <snap_name> <img> <initramfs> <state_img> <hmp_sock>
snapshot_save() {
    local snap_dir="$1" snap_name="$2" img="$3" initramfs="$4" state_img="$5" hmp_sock="$6"
    mkdir -p "$snap_dir"
    [ -n "$state_img" ] && snapshot_create_state_img "$state_img"
    info "保存快照: $snap_name @ $snap_dir"
    hmp_command "$hmp_sock" savevm "$snap_name"
    # hash 只算 Image + initramfs(state_img 内含 savevm 写入,不再作为 hash 因子)
    snapshot_compute_hash "$img" "$initramfs" > "$snap_dir/${snap_name}.hashes"
    info "快照已保存,hash=$(cat "$snap_dir/${snap_name}.hashes")"
}

# 用法:snapshot_load <snap_dir> <snap_name> <img> <initramfs> <state_img> <hmp_sock>
# 返回 0 = 已 load;返回 1 = 失效,已 reset,需重新冷启
snapshot_load() {
    local snap_dir="$1" snap_name="$2" img="$3" initramfs="$4" state_img="$5" hmp_sock="$6"
    if snapshot_is_valid "$snap_dir" "$snap_name" "$img" "$initramfs" "$state_img"; then
        info "快照有效,加载: $snap_name"
        hmp_command "$hmp_sock" loadvm "$snap_name"
    else
        warn "快照失效,丢弃 hash 并重新冷启"
        snapshot_reset "$snap_dir" "$snap_name"
        return 1
    fi
}

# 用法:snapshot_reset <snap_dir> <snap_name>
# 仅清 hash 文件,不动 qcow2 状态盘(qcow2 独立生命周期管理)
snapshot_reset() {
    local snap_dir="$1" snap_name="$2"
    warn "清理快照 hash: $snap_dir/${snap_name}.hashes"
    rm -f "$snap_dir/${snap_name}.hashes"
    info "快照 hash 已清理"
}