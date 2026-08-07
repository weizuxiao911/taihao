#!/bin/bash
# 太昊 OS A/B OTA 构建脚本骨架
#
# 流程:
#   1. 构建 A/B 槽镜像(全量 + 可选差分)
#   2. 生成 OTA 包(清单 + 校验和 + 目标槽)
#
# OTA 包格式(骨架):
#   ota-<version>.tar.gz
#   ├── ota.json        清单(版本 / 目标槽 / 校验和 / 回滚信息)
#   ├── system.ext4     A/B 槽 rootfs 镜像
#   └── boot.bin        boot 分区内容(可选)

set -euo pipefail

OTA_VERSION="${OTA_VERSION:-0.1.0}"
ROOTFS_SRC="${1:?rootfs 源目录(如 output/images/rootfs.ext4)}"
OUT_DIR="${2:-$(pwd)/ota-out}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

mkdir -p "$OUT_DIR"

# ===== 构建 A/B 镜像 =====
# A 槽 = 当前构建; B 槽 = 全量镜像(骨架:实际从分区工具产出)
echo "=== OTA 构建 v$OTA_VERSION ==="
cp "$ROOTFS_SRC" "$OUT_DIR/system-A.ext4"

# 生成 B 槽(占位:同 A 槽,实际由构建链产物决定)
cp "$ROOTFS_SRC" "$OUT_DIR/system-B.ext4"

# ===== OTA 包 =====
PKG_DIR="$OUT_DIR/pkg"
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"

# 校验和
SHA_A=$(sha256sum "$OUT_DIR/system-A.ext4" | awk '{print $1}')
SHA_B=$(sha256sum "$OUT_DIR/system-B.ext4" | awk '{print $1}')

cat > "$PKG_DIR/ota.json" <<EOF
{
  "version": "$OTA_VERSION",
  "target_slot": "B",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "checksums": {
    "system_A": "$SHA_A",
    "system_B": "$SHA_B"
  },
  "rollback": {
    "method": "bootloader_counter",
    "max_attempts": 3
  }
}
EOF

cp "$OUT_DIR/system-A.ext4" "$PKG_DIR/system_A.ext4"
cp "$OUT_DIR/system-B.ext4" "$PKG_DIR/system_B.ext4"

OTA_PKG="$OUT_DIR/ota-$OTA_VERSION.tar.gz"
tar -czf "$OTA_PKG" -C "$PKG_DIR" .
echo "OTA 包: $OTA_PKG"
cat "$PKG_DIR/ota.json"
echo "完成"