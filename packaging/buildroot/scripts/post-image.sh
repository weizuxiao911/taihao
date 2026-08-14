#!/bin/bash
# Buildroot post-image 脚本:生成多平台可启动 SD 卡镜像
#
# 平台:qemu(默认) / rpi4b / rpi3bp
# 产出:$BINARIES_DIR/taihao-{platform}.img
# 切换:PLATFORM 环境变量(默认 qemu)
#
# 共享内核(arm64)+ rootfs,bootloader 平台各自拼。
# genimage 用法:platforms 各自的 genimage-{platform}.cfg
#
# 用法:
#   PLATFORM=qemu  make target-post-image   # 默认
#   PLATFORM=rpi4b make target-post-image
#   PLATFORM=rpi3bp make target-post-image

set -euo pipefail

BINARIES_DIR="${BINARIES_DIR:-$1}"
OUT_DIR="$(dirname "$BINARIES_DIR")"
ROOTFS_EXT2="$BINARIES_DIR/rootfs.ext2"
GENIMAGE_TMP_BASE="${BUILD_DIR:-/tmp}"

# 平台(默认 qemu;buildroot 调用时 $2 = BR2_ROOTFS_POST_SCRIPT_ARGS,
# 用法:make 'BR2_ROOTFS_POST_SCRIPT_ARGS=rpi4b' target-post-image)
PLATFORM="${2:-${PLATFORM:-qemu}}"
case "$PLATFORM" in
    qemu)    OUT_IMG="taihao-qemu.img"  ;;
    rpi4b)   OUT_IMG="taihao-rpi4b.img" ;;
    rpi3bp)  OUT_IMG="taihao-rpi3bp.img" ;;
    *)
        echo "post-image: 未知平台 '$PLATFORM'(支持:qemu|rpi4b|rpi3bp)" >&2
        exit 1
        ;;
esac

# 路径
SCRIPT_DIR="${SCRIPT_DIR:-/Users/weizuxiao/Documents/水下机器人/taihao/packaging/buildroot/scripts}"
SRC_BOOT="${TAIHAO_OUT:-/home/weizuxiao.guest/taihao-out}/arm64-e2e"
GENIMAGE_CFG="$SCRIPT_DIR/genimage-${PLATFORM}.cfg"
GENIMAGE_TMP="$GENIMAGE_TMP_BASE/genimage-${PLATFORM}.tmp"

# 校验
[ -f "$ROOTFS_EXT2" ] || { echo "post-image: 缺 $ROOTFS_EXT2(检查 BR2_TARGET_ROOTFS_EXT2)"; exit 1; }
[ -f "$SRC_BOOT/initramfs.cpio" ] || [ "$PLATFORM" != "qemu" ] \
    || { echo "post-image: 缺 $SRC_BOOT/initramfs.cpio"; exit 1; }
[ -f "$SRC_BOOT/kernel-build/arch/arm64/boot/Image" ] \
    || { echo "post-image: 缺 $SRC_BOOT/Image"; exit 1; }
[ -f "$GENIMAGE_CFG" ] || { echo "post-image: 缺 $GENIMAGE_CFG"; exit 1; }

# 临时 staging 目录
STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING" "$GENIMAGE_TMP"' EXIT

mkdir -p "$STAGING/boot"

# 通用:Image 拷过来,各平台自己改名(RPi 改 kernel8.img,genimage 时处理)
cp "$SRC_BOOT/kernel-build/arch/arm64/boot/Image" "$STAGING/boot/Image"
[ -d "$SRC_BOOT/kernel-build/arch/arm64/boot/dts" ] \
    && cp -r "$SRC_BOOT/kernel-build/arch/arm64/boot/dts" "$STAGING/boot/dtbs"

case "$PLATFORM" in
    qemu)
        # QEMU:只需要 Image + initramfs + dtbs(FAT32 boot,QEMU 不读)
        cp "$SRC_BOOT/initramfs.cpio" "$STAGING/boot/initramfs.cpio"
        ;;

    rpi4b|rpi3bp)
        # RPi:需要 VideoCore 固件 + config.txt + kernel8.img + DTB
        bash "$SCRIPT_DIR/fetch-rpi-firmware.sh"
        FW_DIR="${TAIHAO_OUT:-/home/weizuxiao.guest/taihao-out}/rpi-firmware"

        # 拷全部 VideoCore 固件(start*.elf / fixup*.dat / bootcode.bin)
        # RPi 4B 固件按文件名优先级查(start4x.elf → start_x.elf → ...),
        # 拷不全就有 "Firmware not found" 风险
        for f in "$FW_DIR/boot/"*; do
            [ -f "$f" ] || continue
            base="$(basename "$f")"
            # 排除 *.dtb / *.dtbo / overlays / LICENCE / COPYING(避免 boot 分区塞爆)
            case "$base" in
                *.dtb|*.dtbo) continue ;;
                LICENCE*|COPYING*) continue ;;
                overlays) continue ;;
            esac
            cp "$f" "$STAGING/boot/"
        done

        # Image → kernel8.img(RPi 固件只认这个文件名)
        mv "$STAGING/boot/Image" "$STAGING/boot/kernel8.img"

        # DTB(按平台选一个,RPi 固件优先看 device_tree= 指定的)
        case "$PLATFORM" in
            rpi4b)  cp "$FW_DIR/boot/bcm2711-rpi-4-b.dtb"      "$STAGING/boot/" ;;
            rpi3bp) cp "$FW_DIR/boot/bcm2710-rpi-3-b-plus.dtb"  "$STAGING/boot/" ;;
        esac

        # config.txt(arm_64bit=1 + 内核 + DTB 引用)
        cat > "$STAGING/boot/config.txt" <<EOF
# 太昊 OS · $PLATFORM 启动配置
arm_64bit=1
kernel=kernel8.img
$(case "$PLATFORM" in
    rpi4b)  echo "device_tree=bcm2711-rpi-4-b.dtb" ;;
    rpi3bp) echo "device_tree=bcm2710-rpi-3-b-plus.dtb" ;;
esac)
# initramfs(initrd)由固件加载,作为内核 initrd=
initramfs initramfs.cpio 0x01f00000
EOF
        cp "$SRC_BOOT/initramfs.cpio" "$STAGING/boot/"
        ;;
esac

# genimage 调用
rm -rf "$GENIMAGE_TMP"
echo "post-image: 生成 SD 卡镜像 (platform=$PLATFORM)"

genimage \
    --rootpath  "$STAGING" \
    --tmppath   "$GENIMAGE_TMP" \
    --inputpath "$STAGING/boot" \
    --outputpath "$BINARIES_DIR" \
    --config    "$GENIMAGE_CFG"

# 收尾:重命名 sdcard.img → taihao-{platform}.img,清掉中间产物
mv "$BINARIES_DIR/sdcard.img" "$BINARIES_DIR/$OUT_IMG"
rm -f "$BINARIES_DIR/boot.vfat" "$BINARIES_DIR/rootfs.ext4"

echo "post-image: $(ls -lh "$BINARIES_DIR/$OUT_IMG" | awk '{print $5}') $BINARIES_DIR/$OUT_IMG (PLATFORM=$PLATFORM)"
