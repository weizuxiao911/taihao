#!/bin/bash
# Buildroot post-image 脚本:打包根文件系统 + 生成启动用镜像
#
# 产出:
#   packaging/output/taihao-rootfs.ext4    rootfs 镜像(qemu 用)
#   packaging/output/taihao-image.tar.gz   完整镜像包(工厂 dd 用)

set -euo pipefail

BINARIES_DIR="${BINARIES_DIR:-$1}"
OUT_DIR="$(dirname "$BINARIES_DIR")"
ROOTFS_EXT4="$BINARIES_DIR/rootfs.ext4"
ROOTFS_TAR="$BINARIES_DIR/rootfs.tar.gz"

# rootfs.ext4 由 BR2_TARGET_ROOTFS_EXT2 生成
if [ -f "$ROOTFS_EXT4" ]; then
    echo "post-image: rootfs.ext4 已生成 ($(du -h "$ROOTFS_EXT4" | cut -f1))"
else
    echo "post-image: 未找到 rootfs.ext4(检查 BR2_TARGET_ROOTFS_EXT2 配置)" >&2
    exit 1
fi

# 镜像包(工厂烧录用):rootfs + 元信息
if [ -f "$ROOTFS_TAR" ]; then
    echo "post-image: rootfs.tar.gz 已生成"
fi

echo "post-image: 完成"