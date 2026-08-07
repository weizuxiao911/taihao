#!/bin/bash
# 太昊 OS 固件分区布局 + 工厂 dd 烧录
#
# 依据:契约 §8 A/B 双系统分区 + bootloader 切换
# 布局(4 分区):
#   p1  boot     64M   U-Boot / 内核 Image / DTB / 启动参数
#   p2  system_a 1G    A 槽系统
#   p3  system_b 1G    B 槽系统
#   p4  data     --    数据(占剩余空间)
#
# 产物:
#   scripts/partition/partition-layout.txt    分区表描述
#   scripts/partition/partition-layout.img    可 dd 的布局镜像(骨架)

set -euo pipefail

OUT="${1:-partition-layout}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="$(cd "$SCRIPT_DIR" && pwd)"

BOOT_SIZE_MB=64
SYSTEM_SIZE_MB=1024
DEVICE="${DEVICE:-/dev/mmcblk0}"

# ===== 分区表描述 =====
cat > "$OUT_DIR/partition-layout.txt" <<EOF
# 太昊 OS 固件分区布局(契约 §8 A/B 双系统)
# 生成时间:$(date -u +%Y-%m-%dT%H:%M:%SZ)
# 目标设备:$DEVICE

p1  boot      ${BOOT_SIZE_MB}M    U-Boot + Image + DTB + extlinux
p2  system_a  ${SYSTEM_SIZE_MB}M  A 槽系统(ext4)
p3  system_b  ${SYSTEM_SIZE_MB}M  B 槽系统(ext4)
p4  data      rest               数据分区(ext4)
EOF

echo "分区布局描述: $OUT_DIR/partition-layout.txt"
cat "$OUT_DIR/partition-layout.txt"

# ===== 布局镜像(骨架) =====
# 用 parted 生成真实分区表(若可用);不可用时输出纯文本描述
if command -v parted >/dev/null 2>&1 && command -v qemu-img >/dev/null 2>&1; then
    # 生成 4 分区 GPT 镜像(用于 qemu 模拟 / 真机 dd 参考)
    TOTAL_MB=$(( BOOT_SIZE_MB + SYSTEM_SIZE_MB + SYSTEM_SIZE_MB + 256 ))
    IMG="$OUT_DIR/partition-layout.img"
    rm -f "$IMG"
    qemu-img create -f raw "$IMG" "${TOTAL_MB}M" >/dev/null
    parted -s "$IMG" mklabel gpt
    parted -s "$IMG" unit MiB mkpart boot ext4 1 "$((BOOT_SIZE_MB + 1))"
    parted -s "$IMG" unit MiB mkpart system_a ext4 "$((BOOT_SIZE_MB + 1))" "$((BOOT_SIZE_MB + SYSTEM_SIZE_MB + 1))"
    parted -s "$IMG" unit MiB mkpart system_b ext4 "$((BOOT_SIZE_MB + SYSTEM_SIZE_MB + 1))" "$((BOOT_SIZE_MB + 2 * SYSTEM_SIZE_MB + 1))"
    parted -s "$IMG" unit MiB mkpart data ext4 "$((BOOT_SIZE_MB + 2 * SYSTEM_SIZE_MB + 1))" 100%
    parted -s "$IMG" print
    echo "布局镜像: $IMG"
else
    echo "警告: 缺少 parted / qemu-img,仅输出文本布局描述(骨架)"
fi

# ===== 工厂 dd 烧录(骨架,不执行) =====
cat > "$OUT_DIR/dd-flash.sh" <<'DDEOF'
#!/bin/bash
# 工厂 dd 烧录骨架(需 root)
# 用法: bash dd-flash.sh <image.img> <device>
set -euo pipefail
IMG="${1:?镜像路径}"
DEV="${2:?设备路径(如 /dev/mmcblk0)}"
echo "将 $IMG 写入 $DEV(全盘,危险!)"
read -r -p "确认?输入 yes: " ans
[ "$ans" = yes ] || exit 1
dd if="$IMG" of="$DEV" bs=4M status=progress conv=fsync
sync
echo "烧录完成"
DDEOF
chmod +x "$OUT_DIR/dd-flash.sh"

echo "工厂 dd 烧录骨架: $OUT_DIR/dd-flash.sh"
echo "完成"