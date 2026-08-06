#!/bin/bash
# Kconfig merge + savedefconfig 一体脚本
#
# 依据:docs/linux-内核裁剪方案.md v0.0.7
# 用途:把 config/kernel/*.config 基线片段与 Linux 6.6 arm64 defconfig 合并,
#      输出 savedefconfig 收敛后的 .config
#
# 用法:
#   scripts/kconfig/merge_config.sh <variant>     # variant = ci | e2e
#   scripts/kconfig/merge_config.sh ci x86_64    # 跨架构派生(x86_64)
#
# 环境要求:
#   - Linux 6.6 LTS 源码在 $LINUX_SRC(默认 ../linux-6.6)
#   - 源码已 make ARCH=arm64 defconfig(或 x86_64)
#
# 产出:
#   out/<arch>-<variant>/.config                # 合并后原始 .config
#   out/<arch>-<variant>/defconfig              # savedefconfig 收敛
#   out/<arch>-<variant>/diff-from-defconfig    # 与原 defconfig 的差异

set -euo pipefail

# ===== 参数解析 =====
VARIANT="${1:-ci}"
CROSS_ARCH="${2:-}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LINUX_SRC="${LINUX_SRC:-$PROJECT_ROOT/linux-6.6}"
REPO_ROOT="$(cd "$PROJECT_ROOT/.." && pwd)"

# ===== 校验 =====
if [ ! -d "$LINUX_SRC" ]; then
    echo "ERROR: Linux 源码目录不存在:$LINUX_SRC" >&2
    echo "       设置环境变量 LINUX_SRC=/path/to/linux-6.6" >&2
    exit 1
fi

case "$VARIANT" in
    ci)  FRAGMENT="$REPO_ROOT/config/kernel/qemu-aarch64-ci.config" ;;
    e2e) FRAGMENT="$REPO_ROOT/config/kernel/qemu-aarch64-e2e.config" ;;
    *)
        echo "ERROR: 未知 variant '$VARIANT'(应为 ci|e2e)" >&2
        exit 1
        ;;
esac

if [ ! -f "$FRAGMENT" ]; then
    echo "ERROR: 基线片段不存在:$FRAGMENT" >&2
    exit 1
fi

# ===== 架构选择 =====
if [ -n "$CROSS_ARCH" ]; then
    case "$CROSS_ARCH" in
        x86_64)
            ARCH="x86_64"
            DEFCONFIG="$LINUX_SRC/arch/x86/configs/x86_64_defconfig"
            ;;
        arm64|aarch64)
            ARCH="arm64"
            DEFCONFIG="$LINUX_SRC/arch/arm64/configs/defconfig"
            ;;
        *)
            echo "ERROR: 不支持的架构 '$CROSS_ARCH'" >&2
            exit 1
            ;;
    esac
else
    ARCH="arm64"
    DEFCONFIG="$LINUX_SRC/arch/arm64/configs/defconfig"
fi

OUT_DIR="$PROJECT_ROOT/out/$ARCH-$VARIANT"
mkdir -p "$OUT_DIR"

CONFIG_OUT="$OUT_DIR/.config"

echo "==> 合并基线片段"
echo "    源码: $LINUX_SRC"
echo "    架构: $ARCH"
echo "    变体: $VARIANT"
echo "    片段: $FRAGMENT"
echo "    产出: $CONFIG_OUT"

# ===== 1. 准备 defconfig =====
echo "==> [1/4] 拷贝 defconfig → .config"
make -C "$LINUX_SRC" ARCH="$ARCH" CROSS_COMPILE="" \
    O="$OUT_DIR/kernel-build" defconfig 2>&1 | tail -20 || {
        echo "WARN: make defconfig 失败,可能未编译过;尝试直接用 configs/$(basename "$DEFCONFIG")"
        if [ -f "$DEFCONFIG" ]; then
            mkdir -p "$(dirname "$CONFIG_OUT")"
            cp "$DEFCONFIG" "$CONFIG_OUT"
        else
            echo "ERROR: 找不到 defconfig:$DEFCONFIG" >&2
            exit 1
        fi
    }

# 若 defconfig 拷贝失败,直接用 configs 目录
if [ ! -f "$CONFIG_OUT" ] && [ -f "$DEFCONFIG" ]; then
    cp "$DEFCONFIG" "$CONFIG_OUT"
fi

# ===== 2. merge_config.sh 加载片段 =====
echo "==> [2/4] merge_config.sh 合并片段"
KCONFIG_MERGE="$LINUX_SRC/scripts/kconfig/merge_config.sh"
if [ ! -x "$KCONFIG_MERGE" ]; then
    echo "ERROR: 找不到 $KCONFIG_MERGE" >&2
    exit 1
fi

"$KCONFIG_MERGE" -m -O "$OUT_DIR/kernel-build" "$CONFIG_OUT" "$FRAGMENT" 2>&1 | tail -10

# ===== 3. savedefconfig 收敛 =====
echo "==> [3/4] savedefconfig 收敛"
make -C "$LINUX_SRC" ARCH="$ARCH" CROSS_COMPILE="" \
    O="$OUT_DIR/kernel-build" savedefconfig 2>&1 | tail -10 || {
        echo "WARN: savedefconfig 失败,可能旧 config 格式;尝试 syncconfig"
        make -C "$LINUX_SRC" ARCH="$ARCH" CROSS_COMPILE="" \
            O="$OUT_DIR/kernel-build" syncconfig 2>&1 | tail -10
    }

# 拷贝收敛结果
DEFCONFIG_OUT="$OUT_DIR/defconfig"
if [ -f "$OUT_DIR/kernel-build/defconfig" ]; then
    cp "$OUT_DIR/kernel-build/defconfig" "$DEFCONFIG_OUT"
fi

# ===== 4. 与原 defconfig diff =====
echo "==> [4/4] 与原 defconfig diff"
DIFF_OUT="$OUT_DIR/diff-from-defconfig"
if [ -f "$DEFCONFIG" ] && [ -f "$DEFCONFIG_OUT" ]; then
    diff -u "$DEFCONFIG" "$DEFCONFIG_OUT" > "$DIFF_OUT" || true
    echo "    diff 行数: $(wc -l < "$DIFF_OUT")"
fi

# ===== 终态 =====
echo ""
echo "==> 完成"
echo "    .config         : $CONFIG_OUT"
echo "    savedefconfig   : $DEFCONFIG_OUT"
echo "    diff-from-orig  : $DIFF_OUT"
echo ""
echo "==> 自检:savedefconfig 收敛后 .config 应只含契约允许差异"
echo "    检查 #84 kexec:#84 kexec|全裁 → 期望 CONFIG_KEXEC 不出现"
echo "    检查 #87 VPU:#87 VPU|全裁 → 期望 CONFIG_VIDEO_CODEC_* 不出现"