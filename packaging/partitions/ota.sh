#!/bin/bash
# 太昊 OS OTA 升级脚本（A/B 双系统分区 + 失败回滚）— 骨架
# 用法: ota.sh <新系统镜像.squashfs> [--activate-after]

set -euo pipefail

DEV_SYS_A="/dev/disk/by-partlabel/sys-a"
DEV_SYS_B="/dev/disk/by-partlabel/sys-b"
ACTIVE_FILE="/var/lib/taihao-os/ota/active-slot"

NEW_IMG="${1:?缺少新系统镜像参数}"
ACTIVATE_AFTER="${2:-}"

mkdir -p /var/lib/taihao-os/ota

current_slot() {
    if [[ -f "${ACTIVE_FILE}" ]]; then
        cat "${ACTIVE_FILE}"
    else
        echo "a"
    fi
}

target_slot() {
    if [[ "$(current_slot)" == "a" ]]; then
        echo "b"
    else
        echo "a"
    fi
}

echo "当前槽位: $(current_slot) → 目标槽位: $(target_slot)"

# 1. 写入非活动槽位
if [[ "$(target_slot)" == "a" ]]; then
    dd if="${NEW_IMG}" of="${DEV_SYS_A}" bs=4M conv=fsync
else
    dd if="${NEW_IMG}" of="${DEV_SYS_B}" bs=4M conv=fsync
fi
echo "系统镜像已写入 $(target_slot)"

# 2. 激活新槽位（下次启动引导新系统）
if [[ -n "${ACTIVATE_AFTER}" ]]; then
    echo "$(target_slot)" > "${ACTIVE_FILE}"
    echo "已激活 $(target_slot)，下次重启生效"
fi

# 3. 失败回滚：引导加载程序检测新槽位启动失败时，自动回退旧槽位
#    （真机实现按引导加载程序（U-Boot）AB 策略落地；全局配置与业务分区
#     在升级中保留，不受槽位切换影响）
echo "升级完成。若新系统启动失败，引导加载程序自动回滚到 $(current_slot)。"
