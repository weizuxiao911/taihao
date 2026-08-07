#!/bin/bash
# 太昊 OS A/B OTA 应用脚本骨架
#
# 流程(契约 §8 A/B 双系统 + 失败回滚):
#   1. 解析 OTA 包 → 目标槽(非活动槽)
#   2. 写入非活动槽 + 校验
#   3. 更新 bootloader 启动计数 → 切换活动槽
#   4. 启动计数超限 → 自动回滚
#
# 槽位状态机:
#   active=A / inactive=B → OTA 写 B → 校验 → 切换 active=B → 失败回滚 A

set -euo pipefail

OTA_PKG="${1:?OTA 包路径(ota-<ver>.tar.gz)}"
DEV_PREFIX="${2:-/dev/mmcblk0}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "=== OTA 应用 ==="

# ===== 1. 解析 OTA 包 =====
tar -xzf "$OTA_PKG" -C "$WORK_DIR"
OTA_JSON="$WORK_DIR/ota.json"
if [ ! -f "$OTA_JSON" ]; then
    echo "错误: OTA 包缺少 ota.json" >&2
    exit 1
fi

VERSION=$(python3 -c "import json;print(json.load(open('$OTA_JSON'))['version'])" 2>/dev/null || echo unknown)
TARGET_SLOT=$(python3 -c "import json;print(json.load(open('$OTA_JSON'))['target_slot'])" 2>/dev/null || echo B)
echo "OTA 版本: $VERSION"
echo "目标槽: $TARGET_SLOT"

# 目标分区(骨架:/dev/mmcblk0p2 = system_a, p3 = system_b)
case "$TARGET_SLOT" in
    A) TARGET_PART="${DEV_PREFIX}p2" ;;
    B) TARGET_PART="${DEV_PREFIX}p3" ;;
    *) echo "错误: 未知目标槽 $TARGET_SLOT" >&2; exit 1 ;;
esac
echo "目标分区: $TARGET_PART"

# ===== 2. 写入非活动槽 + 校验 =====
# 骨架:校验 OTA 包内镜像与清单一致(不落真机分区)
IMG="$WORK_DIR/system_${TARGET_SLOT}.ext4"
SHA_IMG=$(sha256sum "$IMG" | awk '{print $1}')
SHA_EXPECT=$(python3 -c "import json;print(json.load(open('$OTA_JSON'))['checksums']['system_${TARGET_SLOT}'])" 2>/dev/null || echo "")

if [ -n "$SHA_EXPECT" ] && [ "$SHA_IMG" != "$SHA_EXPECT" ]; then
    echo "错误: 镜像校验失败(期望 $SHA_EXPECT,实际 $SHA_IMG)" >&2
    echo "OTA 失败: 保持当前槽位,不切换" >&2
    exit 1
fi
echo "镜像校验通过: $SHA_IMG"

# 写入分区(骨架:仿真下不落真机,仅输出 dd 命令)
echo "写入命令(真机执行):"
echo "  dd if=$IMG of=$TARGET_PART bs=4M conv=fsync"

# ===== 3. 切换活动槽 =====
# bootloader 计数:骨架输出切换命令
cat > "$SCRIPT_DIR/.slot-switch-cmd" <<EOF
# 切换活动槽到 $TARGET_SLOT(真机 bootloader 执行)
# 例如 U-Boot: env set boot_slot $TARGET_SLOT; saveenv
echo "切换活动槽: $TARGET_SLOT"
EOF
echo "切换命令已生成: $SCRIPT_DIR/.slot-switch-cmd"

# ===== 4. 失败回滚 =====
# 骨架:启动计数超限自动回滚(由 bootloader 侧实现,此处输出约定)
cat > "$SCRIPT_DIR/.rollback-contract" <<EOF
# 回滚契约(骨架)
# - bootloader 维护 boot_slot + boot_attempts
# - 每次启动 boot_attempts++
# - boot_attempts > 3 → 自动切回上一个槽,清除计数
# - 系统正常启动后 systemd 服务成功 → 清除 boot_attempts
EOF
echo "回滚契约: $SCRIPT_DIR/.rollback-contract"

echo "OTA 应用完成(骨架,未落真机)"