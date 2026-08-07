#!/bin/bash
# 内核构建主入口(含源码拉取、配置合并、编译、initramfs、指标)
#
# 用法:
#   scripts/kernel-build/build-kernel.sh ci|e2e [options]
#
# 选项:
#   --linux-src <path>      Linux 6.6 源码目录(不存在则自动 clone)
#   --commit <tag>          锁定的 commit 或 tag(默认 v6.6)
#   --cross-compile <pfx>   交叉编译前缀(默认 aarch64-linux-gnu-)
#   --no-ccache             禁用 ccache
#   --no-initramfs          不生成 initramfs
#   --initramfs-dir <path>  外部 rootfs 目录(busybox + systemd)
#   --jobs <N>              并行任务数(默认 nproc)
#   --clean                 清理后构建
#   --verbose               详细输出
#
# 产出:
#   out/arm64-{ci,e2e}/Image
#   out/arm64-{ci,e2e}/initramfs.cpio
#   out/arm64-{ci,e2e}/snap/boot-snap.qcow2   状态盘(E2E 用)
#   out/arm64-{ci,e2e}/build.log
#   out/arm64-{ci,e2e}/build.time           cold|warm|<秒>
#
# 退出码:
#   0 = 成功
#   1 = 参数错误
#   2 = 构建失败
#   3 = 指标超限

set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/lib" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"
# shellcheck disable=SC1091
source "$_LIB_DIR/ccache.sh"
# shellcheck disable=SC1091
source "$_LIB_DIR/snapshot.sh"

# ===== initramfs 构建 =====
# 用法:build_minimal_initramfs <src_dir> <out_cpio>
build_minimal_initramfs() {
    local src="$1" out="$2"
    if [ -z "$src" ] || [ ! -d "$src" ]; then
        warn "未提供 initramfs 源目录($src),跳过 initramfs 构建"
        warn "  使用 buildroot 准备 /opt/buildroot/output/target 后用 --initramfs-dir 传入"
        return 1
    fi
    require_cmd cpio
    require_cmd find
    info "从 $src 构建 initramfs → $out"
    # mmdebstrap / debootstrap 等工具生成的 rootfs 含 root:root 目录
    # 非 root 用户遍历会报 Permission denied;用 sudo 兜底,失败回退 chmod
    # cpio 退码 2 = "block size warning"(non-fatal),用 || true 兜底
    (
        cd "$src"
        if command -v sudo >/dev/null 2>&1; then
            sudo find . -print0 2>/dev/null | cpio --null -ov --format=newc 2>/dev/null || true
        else
            chmod -R a+rX . 2>/dev/null || true
            find . -print0 | cpio --null -ov --format=newc 2>/dev/null || true
        fi
    ) > "$out"
    [ -s "$out" ] || die "initramfs 打包失败(输出空)"
    info "initramfs 已生成: $out ($(du -h "$out" | cut -f1))"
}

# ===== 参数 =====
VARIANT=""
USE_CCACHE=1
BUILD_INITRAMFS=1
JOBS=$(nproc)
DO_CLEAN=0
VERBOSE=0
# 锁定 v6.6 tag(tag 解引用到具体 commit hash,不可变)
LOCKED_COMMIT="v6.6"
CROSS_COMPILE="${CROSS_COMPILE:-aarch64-linux-gnu-}"
INITRAMFS_SRC_DIR=""

usage() {
    cat >&2 <<EOF
用法: $0 ci|e2e [options]

选项:
  --linux-src <path>      Linux 6.6 源码目录
  --commit <tag>          锁定的 commit 或 tag(默认 v6.6)
  --cross-compile <pfx>   交叉编译前缀(默认 aarch64-linux-gnu-)
  --no-ccache             禁用 ccache
  --no-initramfs          不生成 initramfs
  --initramfs-dir <path>  外部 rootfs 目录(busybox + systemd)
  --jobs <N>              并行任务数(默认 $(nproc))
  --clean                 清理后构建
  --verbose               详细输出

环境变量:
  KERNEL_BUILD_LINUX_SRC  Linux 源码目录(默认 \$(pwd)/linux-6.6)
  KERNEL_BUILD_OUT         产出根目录
  KERNEL_BUILD_CACHE_DIR   ccache 缓存目录
EOF
    exit 1
}

[ $# -eq 0 ] && usage
VARIANT="$1"
shift

case "$VARIANT" in
    ci)  FRAGMENT="$KERNEL_BUILD_FRAG_DIR/qemu-aarch64-ci.config" ;;
    e2e) FRAGMENT="$KERNEL_BUILD_FRAG_DIR/qemu-aarch64-e2e.config" ;;
    *)   die "未知 variant: $VARIANT(应为 ci|e2e)" ;;
esac

while [ $# -gt 0 ]; do
    case "$1" in
        --linux-src)      KERNEL_BUILD_LINUX_SRC="$2"; shift 2 ;;
        --commit)         LOCKED_COMMIT="$2"; shift 2 ;;
        --cross-compile)  CROSS_COMPILE="$2"; shift 2 ;;
        --no-ccache)      USE_CCACHE=0; shift ;;
        --no-initramfs)   BUILD_INITRAMFS=0; shift ;;
        --initramfs-dir)  INITRAMFS_SRC_DIR="$2"; shift 2 ;;
        --jobs)           JOBS="$2"; shift 2 ;;
        --clean)          DO_CLEAN=1; shift ;;
        --verbose)        VERBOSE=1; shift ;;
        -h|--help)        usage ;;
        *)                die "未知选项: $1" ;;
    esac
done

# ===== 路径安全校验 =====
check_safe_path "$KERNEL_BUILD_LINUX_SRC" "Linux 源码目录"
check_safe_path "$KERNEL_BUILD_OUT" "产出目录"

# ===== 交叉编译器校验 =====
require_cmd "${CROSS_COMPILE}gcc"
require_cmd "${CROSS_COMPILE}ld"
info "交叉编译器: ${CROSS_COMPILE}gcc"
# 打印版本(SIGPIPE 安全:不使用 | debug 函数 + head;直接写 stderr)
"${CROSS_COMPILE}gcc" --version 2>&1 | head -1 >&2 || true

# ===== 源码准备(独立于构建计时) =====
ensure_linux_source() {
    local src="$1" commit="$2"
    if [ -d "$src" ]; then
        info "源码目录已存在: $src"
    else
        info "源码目录不存在,自动 clone"
        require_cmd git
        local parent
        parent="$(dirname "$src")"
        mkdir -p "$parent"
        git clone --depth=1 --branch "$commit" \
            https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git \
            "$src" || die "clone Linux $commit 失败"
    fi
    cd "$src"
    git fetch --tags --depth=1 origin "refs/tags/$commit:refs/tags/$commit" 2>/dev/null || \
        git fetch --tags origin 2>/dev/null || true
    git checkout "$commit" 2>/dev/null || die "checkout $commit 失败"
    git rev-parse HEAD
}

# ===== 准备 =====
info "=== 内核构建 ==="
info "  变体: $VARIANT"
info "  交叉编译: $CROSS_COMPILE"
info "  锁定 commit: $LOCKED_COMMIT"
info "  ccache: $([ $USE_CCACHE -eq 1 ] && echo on || echo off)"
info "  initramfs: $([ $BUILD_INITRAMFS -eq 1 ] && echo on || echo off)"

OUT_DIR="$KERNEL_BUILD_OUT/arm64-$VARIANT"
LOG_FILE="$OUT_DIR/build.log"
TIME_FILE="$OUT_DIR/build.time"
SNAP_DIR="$OUT_DIR/snap"
STATE_IMG="$SNAP_DIR/boot-snap.qcow2"

# 路径含空格校验(影响 qemu / monitor socket)
check_safe_path "$OUT_DIR" "OUT_DIR"
check_safe_path "$SNAP_DIR" "snap 目录"

mkdir -p "$OUT_DIR" "$SNAP_DIR"

# ===== --clean 安全清理 =====
# 用 find + delete 逐个删除(避免 rm -rf 误伤)
if [ $DO_CLEAN -eq 1 ]; then
    info "清理产出目录(逐文件)"
    rm -f "$OUT_DIR/Image" "$OUT_DIR/initramfs.cpio" "$LOG_FILE" "$TIME_FILE"
    # 清空 snap 目录内容
    find "$SNAP_DIR" -mindepth 1 -maxdepth 1 -delete 2>/dev/null || true
fi

# ===== 冷/增量判定 =====
[ -f "$OUT_DIR/Image" ] && [ ! $DO_CLEAN -eq 1 ] && MODE="warm" || MODE="cold"
info "构建模式: $MODE"

# ===== 源码拉取(不计入构建计时) =====
info "[1/5] 源码准备(计时外)"
ACTUAL_COMMIT=$(ensure_linux_source "$KERNEL_BUILD_LINUX_SRC" "$LOCKED_COMMIT")
info "  源码 commit: $ACTUAL_COMMIT"

# 校验:tag 解引用的 commit 必须 == HEAD;这样 LOCKED_COMMIT="v6.6" 也强校验
if [ "$LOCKED_COMMIT" = "v6.6" ] || git show-ref --tags/"$LOCKED_COMMIT" >/dev/null 2>&1; then
    local_tag_commit=$(git -C "$KERNEL_BUILD_LINUX_SRC" rev-parse "refs/tags/$LOCKED_COMMIT"^{commit} 2>/dev/null || echo "")
    if [ -n "$local_tag_commit" ] && [ "$local_tag_commit" != "$ACTUAL_COMMIT" ]; then
        die "tag $LOCKED_COMMIT 指向 $local_tag_commit,但 HEAD=$ACTUAL_COMMIT(上游变动!)"
    fi
fi

# ===== 构建计时 =====
info "[2/5] 配置合并 + 编译(计时内)"
timer_start

# ===== ccache =====
[ $USE_CCACHE -eq 1 ] && {
    ccache_init || { warn "ccache 初始化失败,改用无 ccache 构建"; USE_CCACHE=0; }
    ccache_export
}

# ===== 配置合并 =====
MERGE_OUT="$OUT_DIR/kernel-build"
mkdir -p "$MERGE_OUT"

MERGE_SCRIPT="$KERNEL_BUILD_LINUX_SRC/scripts/kconfig/merge_config.sh"
require_file "$MERGE_SCRIPT" "merge_config.sh"

CONFIG_BASE="$MERGE_OUT/.config"
make -C "$KERNEL_BUILD_LINUX_SRC" ARCH=arm64 CROSS_COMPILE="$CROSS_COMPILE" \
    O="$MERGE_OUT" defconfig >"$LOG_FILE" 2>&1 || \
    die "defconfig 失败(见 $LOG_FILE)"

"$MERGE_SCRIPT" -m -O "$MERGE_OUT" "$CONFIG_BASE" "$FRAGMENT" \
    >"$LOG_FILE.config" 2>&1 || die "merge_config 失败"

# ===== 状态盘创建(E2E 用,独立于 BUILD_INITRAMFS) =====
if [ "$VARIANT" = "e2e" ]; then
    snapshot_create_state_img "$STATE_IMG"
fi

# ===== 编译 Image =====
MAKE_ARGS=(
    make
    -C "$KERNEL_BUILD_LINUX_SRC" ARCH=arm64 CROSS_COMPILE="$CROSS_COMPILE"
    O="$MERGE_OUT"
    "-j$JOBS"
    Image dtbs
)
[ $VERBOSE -eq 1 ] && MAKE_ARGS+=(V=1)

"${MAKE_ARGS[@]}" >"$LOG_FILE.build" 2>&1 || {
    tail -30 "$LOG_FILE.build" >&2
    die "内核构建失败"
}

cp "$MERGE_OUT/arch/arm64/boot/Image" "$OUT_DIR/Image"

# ===== initramfs =====
if [ $BUILD_INITRAMFS -eq 1 ]; then
    info "[3/5] 构建 initramfs"
    if [ -z "$INITRAMFS_SRC_DIR" ]; then
        die "initramfs 源目录未指定(--initramfs-dir <path>);
提示:用 buildroot 预构建 /opt/buildroot/output/target(或同等 rootfs 目录),
     内含 busybox 与 systemd 二进制,作为 pid 1。"
    fi
    build_minimal_initramfs "$INITRAMFS_SRC_DIR" "$OUT_DIR/initramfs.cpio"
fi

# ===== 校验 Image =====
info "[4/5] 校验 Image"
file "$OUT_DIR/Image" | grep -qiE "ARM64|aarch64" || \
    die "Image 不是 aarch64(见 file 命令)"

ELAPSED=$(timer_elapsed_sec)
info "[5/5] 计时: $ELAPSED s(模式: $MODE,不计源码拉取)"

# ===== 指标记录 =====
printf '%s|%s\n' "$MODE" "$ELAPSED" > "$TIME_FILE"

# 超限检测
case "$MODE" in
    cold) [ "$(awk "BEGIN {print ($ELAPSED < 180) ? 1 : 0}")" = "1" ] || {
        warn "冷构建超限: $ELAPSED s(目标 < 180 s)"
        exit 3
    } ;;
    warm) [ "$(awk "BEGIN {print ($ELAPSED < 30) ? 1 : 0}")" = "1" ] || {
        warn "增量构建超限: $ELAPSED s(目标 < 30 s)"
        exit 3
    } ;;
esac

info "=== 完成 ==="
info "  Image: $OUT_DIR/Image"
info "  initramfs: $OUT_DIR/initramfs.cpio"
info "  状态盘: $STATE_IMG(E2E)"
info "  时间: $ELAPSED s ($MODE)"

exit 0