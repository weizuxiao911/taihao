#!/bin/bash
# Buildroot post-image 脚本:生成 QEMU 可启动 SD 卡镜像(M1)
#
# 产出:BINARIES_DIR/taihao-qemu.img
#
# 结构:
#   FAT32 boot 256M  →  Image + initramfs.cpio + dtbs/
#   ext4 root 1G+     →  buildroot rootfs.ext2(已由 BR2_TARGET_ROOTFS_EXT2 生成)
#
# 后续 M2/M3(M4) 在此基础上加平台固件(VideoCore 固件 / U-Boot)+ config.txt

set -euo pipefail

BINARIES_DIR="${BINARIES_DIR:-$1}"
OUT_DIR="$(dirname "$BINARIES_DIR")"
ROOTFS_EXT2="$BINARIES_DIR/rootfs.ext2"
# genimage-qemu.cfg 与 post-image.sh 同目录(VM 视角:/Users/.../taihao/packaging/buildroot/scripts/)
SCRIPT_DIR="${SCRIPT_DIR:-/Users/weizuxiao/Documents/水下机器人/taihao/packaging/buildroot/scripts}"
GENIMAGE_CFG="${GENIMAGE_CFG:-$SCRIPT_DIR/genimage-qemu.cfg}"
GENIMAGE_TMP="${BUILD_DIR:-/tmp}/genimage-qemu.tmp"

# 内核产物(build-kernel.sh e2e 产出):~/taihao-out/arm64-e2e/
SRC_BOOT="${TAIHAO_OUT:-/home/weizuxiao.guest/taihao-out}/arm64-e2e"

if [ ! -f "$ROOTFS_EXT2" ]; then
    echo "post-image: 未找到 rootfs.ext2(检查 BR2_TARGET_ROOTFS_EXT2 配置)" >&2
    exit 1
fi
if [ ! -f "$SRC_BOOT/initramfs.cpio" ] || [ ! -f "$SRC_BOOT/kernel-build/arch/arm64/boot/Image" ]; then
    echo "post-image: 缺 $SRC_BOOT/Image 或 initramfs.cpio(先跑 build-kernel.sh e2e)" >&2
    exit 1
fi

# 临时目录:genimage 在 --rootpath 下组装镜像
#   <rootpath>/boot/  →  boot.vfat(挂载点 /boot)
#   <rootpath>/       →  rootfs.ext4(挂载点 /)
ROOTPATH_TMP="$(mktemp -d)"
trap 'rm -rf "$ROOTPATH_TMP" "$GENIMAGE_TMP"' EXIT

mkdir -p "$ROOTPATH_TMP/boot"
cp "$SRC_BOOT/kernel-build/arch/arm64/boot/Image" "$ROOTPATH_TMP/boot/Image"
cp "$SRC_BOOT/initramfs.cpio"                       "$ROOTPATH_TMP/boot/initramfs.cpio"
[ -d "$SRC_BOOT/kernel-build/arch/arm64/boot/dts" ] \
    && cp -r "$SRC_BOOT/kernel-build/arch/arm64/boot/dts" "$ROOTPATH_TMP/boot/dtbs"

rm -rf "$GENIMAGE_TMP"

echo "post-image: 生成 SD 卡镜像 (genimage)"

genimage \
    --rootpath  "$ROOTPATH_TMP" \
    --tmppath   "$GENIMAGE_TMP" \
    --inputpath "$BINARIES_DIR" \
    --outputpath "$BINARIES_DIR" \
    --config    "$GENIMAGE_CFG"

mv "$BINARIES_DIR/sdcard.img" "$BINARIES_DIR/taihao-qemu.img"
rm -f "$BINARIES_DIR/boot.vfat" "$BINARIES_DIR/rootfs.ext4"

echo "post-image: $(ls -lh "$BINARIES_DIR/taihao-qemu.img" | awk '{print $5}') $BINARIES_DIR/taihao-qemu.img"
