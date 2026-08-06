#!/bin/bash
# 公共函数:日志分级、错误处理、路径工具
#
# 用法:source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# 约定:
#   - 错误输出到 stderr
#   - 失败立即退出(set -euo pipefail)
#   - 不引入外部依赖

set -euo pipefail

# ===== 日志分级 =====
_LOG_LEVEL="${_LOG_LEVEL:-info}"

_log() {
    local level="$1"; shift
    case "$_LOG_LEVEL" in
        debug) [ "$level" = debug ] && printf '[%s] %s\n' "$level" "$*" >&2 ;;
        info)  case "$level" in debug) ;; *) printf '[%s] %s\n' "$level" "$*" >&2 ;; esac ;;
        warn)  case "$level" in debug|info) ;; *) printf '[%s] %s\n' "$level" "$*" >&2 ;; esac ;;
        error) case "$level" in debug|info|warn) ;; *) printf '[%s] %s\n' "$level" "$*" >&2 ;; esac ;;
    esac
}

die() {
    _log error "$*"
    exit 1
}

warn() { _log warn "$*"; }
info() { _log info "$*"; }
debug() { _log debug "$*"; }

# ===== 路径工具 =====
SCRIPT_DIR_KBUILD="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_BUILD_LIB_DIR="$SCRIPT_DIR_KBUILD"
KERNEL_BUILD_ROOT="$(cd "$KERNEL_BUILD_LIB_DIR/.." && pwd)"
KERNEL_BUILD_SCRIPTS_DIR="$(cd "$KERNEL_BUILD_ROOT/.." && pwd)"
# 仓库根 = scripts/ 的上级(包含 docs/ config/ examples/ packaging/ 等)
KERNEL_BUILD_PROJECT_ROOT="$(cd "$KERNEL_BUILD_SCRIPTS_DIR/.." && pwd)"
KERNEL_BUILD_REPO_ROOT="$KERNEL_BUILD_PROJECT_ROOT"

# 默认路径(可被环境变量覆盖)
KERNEL_BUILD_LINUX_SRC="${KERNEL_BUILD_LINUX_SRC:-$KERNEL_BUILD_PROJECT_ROOT/linux-6.6}"
KERNEL_BUILD_OUT="${KERNEL_BUILD_OUT:-$KERNEL_BUILD_PROJECT_ROOT/out}"
KERNEL_BUILD_FRAG_DIR="${KERNEL_BUILD_FRAG_DIR:-$KERNEL_BUILD_PROJECT_ROOT/config/kernel}"
KERNEL_BUILD_CACHE_DIR="${KERNEL_BUILD_CACHE_DIR:-$KERNEL_BUILD_PROJECT_ROOT/.cache}"

# ===== 通用校验 =====
require_cmd() {
    local cmd="$1"
    command -v "$cmd" >/dev/null 2>&1 || die "缺少命令: $cmd"
}

require_file() {
    local f="$1" desc="${2:-文件}"
    [ -f "$f" ] || die "$desc 不存在: $f"
}

require_dir() {
    local d="$1" desc="${2:-目录}"
    [ -d "$d" ] || die "$desc 不存在: $d"
}

# ===== 安全操作 =====
# 沙箱化临时目录(自动清理)
mktmpdir() {
    local prefix="${1:-kbuild}"
    mktemp -d -t "${prefix}.XXXXXX"
}

# 路径含空格安全(quoted expansion 即可,这里提供 warning)
check_safe_path() {
    local p="$1" name="${2:-路径}"
    case "$p" in
        *' '*|*'	'*) die "$name 含空格/制表符,无法安全处理: $p" ;;
    esac
}

# ===== 时间计量 =====
timer_start() {
    TIMER_START_NS=$(date +%s%N)
}

timer_elapsed_sec() {
    local now end
    now=$(date +%s%N)
    end=$(( (now - TIMER_START_NS) / 1000000 ))
    echo $(( end / 1000 )).$(printf '%03d' $(( end % 1000 )))
}