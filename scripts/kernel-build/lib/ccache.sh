#!/bin/bash
# ccache 统一配置与初始化
#
# 用法:source "$(dirname "${BASH_SOURCE[0]}")/ccache.sh"
# 行为:
#   - 设置 CC="ccache gcc" 等,使 ccache 接管编译
#   - 限制缓存大小(默认 5GB)
#   - 提供缓存目录统一接口

set -euo pipefail

# ===== 公共依赖 =====
_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"

# ===== 默认配置 =====
CCACHE_BASEDIR="${CCACHE_BASEDIR:-$KERNEL_BUILD_CACHE_DIR/ccache}"
CCACHE_DIR="${CCACHE_DIR:-$CCACHE_BASEDIR}"
CCACHE_MAXSIZE="${CCACHE_MAXSIZE:-5G}"
# ccache 4.x 的 compression 选项为 bool(true/false),不是数字
CCACHE_COMPRESS="${CCACHE_COMPRESS:-true}"
CCACHE_SLOPPINESS="${CCACHE_SLOPPINESS:-time_macros,include_file_mtime,include_file_ctime,file_macro,locale}"

ccache_init() {
    require_cmd ccache
    mkdir -p "$CCACHE_DIR"
    ccache_configure
    ccache_dir_check
    info "ccache 已初始化: $CCACHE_DIR"
}

ccache_dir_check() {
    # ccache -s 不打印 cache_dir 路径,直接用目录存在性 + ccache 可达性检查
    [ -d "$CCACHE_DIR" ] || die "ccache 目录不存在: $CCACHE_DIR"
    # ccache -p 显示当前配置,确认 max_size 等生效
    ccache -p >/dev/null 2>&1 || die "ccache -p 失败,配置未生效"
    info "ccache 配置: $(ccache -p 2>&1 | grep -E '^(cache_dir|max_size|compression)' | tr '\n' ' ')"
}

ccache_configure() {
    # ccache 4.x 配置语法:`ccache -o KEY=VALUE`(不能用 `ccache set`,会触发 wrapper 模式报错 "compiler set")
    # ccache 4.x 选项:cache_dir, max_size, compression (bool), sloppiness;basedir 在 4.x 移除
    ccache -o "cache_dir=$CCACHE_DIR" >/dev/null
    ccache -o "max_size=$CCACHE_MAXSIZE" >/dev/null
    ccache -o "compression=$CCACHE_COMPRESS" >/dev/null
    ccache -o "sloppiness=$CCACHE_SLOPPINESS" >/dev/null
}

ccache_export() {
    # 交叉编译场景:CC 必须含交叉前缀,否则 ccache 调用 host gcc
    local cc="${CROSS_COMPILE:-}gcc"
    local cxx="${CROSS_COMPILE:-}g++"
    export CC="ccache ${cc}"
    export CXX="ccache ${cxx}"
    export CCACHE_DIR
    export CCACHE_BASEDIR
    info "已导出 CC=$CC / CXX=$CXX"
}

ccache_reset() {
    warn "ccache 缓存将被清空"
    ccache -C || true
}

ccache_stats() {
    ccache -s 2>&1
}