#!/bin/bash
# 太昊 OS 固件三分区布局生成脚本（骨架）
# 目标硬件：RK3588 同级别，16GB eMMC
#
# 分区表（GPT）：
#   p1  只读系统分区  A/B 双系统之一（ro，squashfs，升级覆盖）
#   p2  只读系统分区  A/B 双系统之二（ro，squashfs，升级覆盖）
#   p3  全局配置分区  rw，升级保留
#   p4  业务开发分区  rw，厂商可读写，升级保留
#   p5  扩展分区      预留（日志 / OTA 暂存等）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/../output"

EMPTY_IMG="${OUTPUT_DIR}/taihao-partitions.img"
SECTOR=512
DISK_SECTORS=$(( 16 * 1024 * 1024 * 1024 / SECTOR ))   # 16GB

echo "生成三分区磁盘布局骨架: ${EMPTY_IMG} (${DISK_SECTORS} 扇区)"
truncate -s $(( DISK_SECTORS * SECTOR )) "${EMPTY_IMG}"

# GPT 分区表由真机烧录工具（dd 工厂脚本 / OTA 脚本）按上表创建；
# 本骨架仅产出布局说明与占位镜像，供 QEMU/产线联调。
echo "分区布局:"
echo "  p1/p2 系统 A/B (squashfs, ro, 升级覆盖)"
echo "  p3     全局配置 (rw, 升级保留)"
echo "  p4     业务开发 (rw, 厂商可读写, 升级保留)"
echo "  p5     扩展分区 (日志 / OTA 暂存)"
echo "完成: ${EMPTY_IMG}"
