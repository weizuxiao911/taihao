#!/bin/bash
# 太昊 OS 可启动镜像打包:4 分区 GPT 盘 + 内容烧录
#
# 布局(boot / system_a / system_b / data):
#   p1 boot      内核 Image(+DTB 预留)
#   p2 system_a  rootfs 内容(可启动槽)
#   p3 system_b  预留(OTA 目标槽)
#   p4 data      预留(厂商数据)
#
# 用法:
#   bash pack-image.sh <kernel-Image> <rootfs.ext2> <out.img>
#
# 启动方式(qemu,无需 U-Boot):
#   qemu-system-aarch64 -M virt -cpu cortex-a72 -m 1G -smp 2 \
#     -kernel <Image> \
#     -drive file=<out.img>,if=virtio,format=raw \
#     -append "console=ttyAMA0 root=/dev/vda2 rw rdinit=/sbin/init loglevel=4" \
#     -display none -serial stdio -no-reboot

set -euo pipefail

KERNEL="${1:?内核 Image 路径}"
ROOTFS="${2:?rootfs.ext2 路径}"
IMG="${3:?输出镜像路径}"

BOOT_SIZE_MB=64
SYSTEM_SIZE_MB=1024
DATA_SIZE_MB=256
TOTAL_MB=$(( BOOT_SIZE_MB + SYSTEM_SIZE_MB + SYSTEM_SIZE_MB + DATA_SIZE_MB ))

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || { echo "缺少命令: $1" >&2; exit 1; }
}
require_cmd qemu-img
require_cmd parted
require_cmd kpartx
require_cmd mkfs.ext4
require_cmd debugfs

[ -f "$KERNEL" ] || { echo "内核不存在: $KERNEL" >&2; exit 1; }
[ -f "$ROOTFS" ] || { echo "rootfs 不存在: $ROOTFS" >&2; exit 1; }

echo "=== 太昊 OS 可启动镜像打包 ==="
echo "内核: $KERNEL"
echo "rootfs: $ROOTFS"
echo "输出: $IMG"

# ===== 1. 创建 4 分区 GPT 盘 =====
rm -f "$IMG"
qemu-img create -f raw "$IMG" "${TOTAL_MB}M" >/dev/null
parted -s "$IMG" mklabel gpt
parted -s "$IMG" unit MiB mkpart boot ext4 1 "$((BOOT_SIZE_MB + 1))"
parted -s "$IMG" unit MiB mkpart system_a ext4 "$((BOOT_SIZE_MB + 1))" "$((BOOT_SIZE_MB + SYSTEM_SIZE_MB + 1))"
parted -s "$IMG" unit MiB mkpart system_b ext4 "$((BOOT_SIZE_MB + SYSTEM_SIZE_MB + 1))" "$((BOOT_SIZE_MB + 2 * SYSTEM_SIZE_MB + 1))"
parted -s "$IMG" unit MiB mkpart data ext4 "$((BOOT_SIZE_MB + 2 * SYSTEM_SIZE_MB + 1))" 100%
parted -s "$IMG" print

# ===== 2. 映射分区 =====
kpartx -av "$IMG" >/dev/null
sleep 1
LOOP_DEV=$(losetup -j "$IMG" | head -1 | cut -d: -f1)
LOOP_PREFIX="${LOOP_DEV##*/}"
P1="/dev/mapper/${LOOP_PREFIX}p1"
P2="/dev/mapper/${LOOP_PREFIX}p2"
P3="/dev/mapper/${LOOP_PREFIX}p3"

cleanup() {
    sync
    rm -rf /mnt/th-boot /mnt/th-sysa /mnt/th-sysb
    kpartx -d "$IMG" 2>/dev/null || true
}
trap cleanup EXIT

# ===== 3. 格式化分区 =====
echo "--- 格式化分区 ---"
mkfs.ext4 -q -L boot "$P1"
mkfs.ext4 -q -L system_a "$P2"
mkfs.ext4 -q -L system_b "$P3"

# ===== 4. 烧录 boot 分区(内核 Image) =====
echo "--- 烧录 boot 分区 ---"
mkdir -p /mnt/th-boot
mount "$P1" /mnt/th-boot
cp "$KERNEL" /mnt/th-boot/Image
umount /mnt/th-boot
echo "boot: Image 已烧录"

# ===== 5. 烧录 system_a(rootfs 内容,挂载复制) =====
echo "--- 烧录 system_a(rootfs 内容) ---"
mkdir -p /mnt/th-sysa /mnt/th-sysb
mount "$P2" /mnt/th-sysa
# 挂载 rootfs.ext2,复制内容到 system_a(非文件拷贝)
ROOTFS_DEV=$(losetup -f)
losetup "$ROOTFS_DEV" "$ROOTFS"
mkdir -p /mnt/th-rootfs-src
mount "$ROOTFS_DEV" /mnt/th-rootfs-src
cp -a /mnt/th-rootfs-src/. /mnt/th-sysa/
umount /mnt/th-rootfs-src
losetup -d "$ROOTFS_DEV"
rmdir /mnt/th-rootfs-src
sync
umount /mnt/th-sysa
echo "system_a: rootfs 内容已烧录"

# ===== 6. system_b 预留(空 ext4) =====
mount "$P3" /mnt/th-sysb
umount /mnt/th-sysb
echo "system_b: 预留槽(空)"

# ===== 验证 =====
echo "--- 验证 ---"
kpartx -d "$IMG" 2>/dev/null || true
sleep 1
kpartx -av "$IMG" >/dev/null
mkdir -p /mnt/th-check
mount "$P1" /mnt/th-check
echo "boot 内容: $(ls /mnt/th-check/)"
umount /mnt/th-check
mount "$P2" /mnt/th-check
echo "system_a /usr/bin 服务: $(ls /mnt/th-check/usr/bin/ 2>/dev/null | grep -cE 'pi-agent|comm-center|hal-gateway|rt-loop|extension')"
echo "system_a /sbin/init: $(ls /mnt/th-check/sbin/init 2>/dev/null || echo 缺失)"
umount /mnt/th-check
rmdir /mnt/th-check

echo "=== 完成: $IMG ($(du -h "$IMG" | cut -f1)) ==="
echo "启动命令:"
echo "  qemu-system-aarch64 -M virt -cpu cortex-a72 -m 1G -smp 2 \\"
echo "    -kernel $KERNEL \\"
echo "    -drive file=$IMG,if=virtio,format=raw \\"
echo "    -append 'console=ttyAMA0 root=/dev/vda2 rw rdinit=/sbin/init loglevel=4' \\"
echo "    -display none -serial stdio -no-reboot"