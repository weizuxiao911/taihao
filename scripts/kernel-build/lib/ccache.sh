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
CCACHE_COMPRESS="${CCACHE_COMPRESS:-1}"
CCACHE_SLOPPINESS="${CCACHE_SLOPPINESS:-time_macros,include_file_mtime,include_file_ctime,file_macro,locale}"

ccache_init() {
    require_cmd ccache
    mkdir -p "$CCACHE_DIR"
    ccache_dir_check
    ccache_configure
    info "ccache 已初始化: $CCACHE_DIR"
}

ccache_dir_check() {
    local stat
    stat=$(ccache -s 2>&1) || true
    if ! echo "$stat" | grep -q "cache directory"; then
        die "ccache 未指向 $CCACHE_DIR"
    fi
}

ccache_configure() {
    ccache set "basedir" "$CCACHE_BASEDIR" >/dev/null
    ccache set "cache_dir" "$CCACHE_DIR" >/dev/null
    ccache set "max_size" "$CCACHE_MAXSIZE" >/dev/null
    ccache set "compression" "$CCACHE_COMPRESS" >/dev/null
    ccache set "sloppiness" "$CCACHE_SLOPPINESS" >/dev/null
}

ccache_export() {
    export CC="ccache gcc"
    export CXX="ccache g++"
    export CCACHE_DIR
    export CCACHE_BASEDIR
    info "已导出 CC=/CXX= 指向 ccache"
}

ccache_reset() {
    warn "ccache 缓存将被清空"
    ccache -C || true
}

ccache_stats() {
    ccache -s 2>&1
}