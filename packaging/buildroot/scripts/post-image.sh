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

        # 拷全部 VideoCore 固件 + 全部 dtb + overlays/,与官方 RPi OS boot 分区 100% 一致。
        # RPi 固件按文件名优先级查(start4x.elf → start_x.elf → ...),拷不全就
        # "Firmware not found" 卡彩虹屏;dtoverlay=vc4-kms-v3d 依赖 overlays/ 目录,
        # 缺了固件处理 config.txt 时卡住;全量 dtb 供固件按 board 自动选择。
        for f in "$FW_DIR/boot/"*; do
            base="$(basename "$f")"
            case "$base" in
                LICENCE*|COPYING*) continue ;;
            esac
            cp -r "$f" "$STAGING/boot/"
        done

        # 用官方 RPi OS 预编译 kernel8.img(替代我们裁剪的内核),
        # 官方内核已修复 vc4 4B 长期运行 HDMI 崩溃问题,达到官方 RPi OS 一样的
        # 稳定 HDMI CLI 登录体验。rootfs 仍是我们的太昊系统。
        # 官方 kernel8.img 路径:$TAIHAO_OUT/raspios-kernel8.img(从官方 img 提取)
        OFFICIAL_KERNEL="${TAIHAO_OUT:-/home/weizuxiao.guest/taihao-out}/raspios-kernel8.img"
        if [ -f "$OFFICIAL_KERNEL" ]; then
            cp "$OFFICIAL_KERNEL" "$STAGING/boot/kernel8.img"
            rm -f "$STAGING/boot/Image"
            echo "post-image: 用官方 RPi kernel8.img($(ls -la "$OFFICIAL_KERNEL" | awk '{print $5}') 字节)"
        else
            # Fallback: 用我们裁剪的内核(可能 vc4 4B 黑屏)
            mv "$STAGING/boot/Image" "$STAGING/boot/kernel8.img"
            echo "post-image: WARNING 官方内核未找到,用使用官方 kernel8.img;fallback 到太昊裁剪内核(可能 vc4 4B 黑屏)"
        fi

        # config.txt:vc4-kms-v3d(v13 fkms 让 systemd 崩了;v12 kms-v3d 系统完整到 Multi-User)
        cat > "$STAGING/boot/config.txt" <<EOF
# 太昊 OS · $PLATFORM 启动配置
dtparam=audio=on
camera_auto_detect=1
display_auto_detect=1
auto_initramfs=1
dtoverlay=vc4-kms-v3d
max_framebuffers=2
disable_fw_kms_setup=1
arm_64bit=1
disable_overscan=1
arm_boost=1
EOF

        # cmdline.txt:console=tty1 + ttyAMA0 + video=1280x720(回到 v12 验证过的稳定组合)
        cat > "$STAGING/boot/cmdline.txt" <<EOF
console=tty1 console=ttyAMA0,115200 root=/dev/mmcblk0p2 rootfstype=ext4 fsck.repair=yes rootwait quiet loglevel=3
EOF

        # 版本标记:烧录后插回电脑可确认卡上内容(多卡切换时防混淆)
        echo "taihao $PLATFORM build $(date +%Y%m%d-%H%M%S) kernel=$(basename $(readlink -f "$SRC_BOOT/kernel-build/arch/arm64/boot/Image" 2>/dev/null) 2>/dev/null || echo 6.6)" > "$STAGING/boot/taihao.version"

        # boot.vfat:用 mkfs.vfat 显式生成,与官方 RPi OS 完全一致:
        #   -F 32 FAT32; -s 1 每簇 1 扇区(512B 簇); -h 16384 隐藏扇区 = 8MiB 分区偏移。
        # genimage 自动生成的 vfat 隐藏扇区值不对(BPB 0x18 与实际偏移不符),
        # 固件按 BPB 计算读取位置会错位 → "Invalid ELF header: start4.elf" → 卡彩虹屏。
        BOOT_VFAT="$BINARIES_DIR/boot.vfat"
        rm -f "$BOOT_VFAT"
        mkfs.vfat -F 32 -s 1 -h 16384 -C "$BOOT_VFAT" 524288
        MTOOLS_SKIP_CHECK=1 mcopy -sp -i "$BOOT_VFAT" "$STAGING/boot/"* ::/
        echo "post-image: boot.vfat 生成完成 ($(ls -la "$BOOT_VFAT" | awk '{print $5}') 字节)"
        ;;
esac

# genimage 调用
rm -rf "$GENIMAGE_TMP"
echo "post-image: 生成 SD 卡镜像 (platform=$PLATFORM)"

# rootpath=STAGING(boot 文件);inputpath=$BINARIES_DIR(找 rootfs.ext2 直接嵌入,
# 避免 genimage 重新打包 rootfs——它没有 buildroot 的 fakeroot 产物,会生成空 rootfs,
# 且会覆盖 buildroot 的 rootfs.ext4 符号链接、污染 rootfs.ext2 本体)
genimage \
    --rootpath  "$STAGING" \
    --tmppath   "$GENIMAGE_TMP" \
    --inputpath "$BINARIES_DIR" \
    --outputpath "$BINARIES_DIR" \
    --config    "$GENIMAGE_CFG"

# 收尾:重命名 sdcard.img → taihao-{platform}.img,清掉中间产物
mv "$BINARIES_DIR/sdcard.img" "$BINARIES_DIR/$OUT_IMG"
rm -f "$BINARIES_DIR/boot.vfat"

echo "post-image: $(ls -lh "$BINARIES_DIR/$OUT_IMG" | awk '{print $5}') $BINARIES_DIR/$OUT_IMG (PLATFORM=$PLATFORM)"
