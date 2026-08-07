#!/bin/bash
# 启动存活与指标探测
#
# 用法:source "$(dirname "${BASH_SOURCE[0]}")/probe.sh"
# 行为:
#   - 通过串口日志等待 systemd basic.target 达成
#   - 使用正则匹配多种 systemd 输出格式

set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$_LIB_DIR/common.sh"

# ===== 配置 =====
PROBE_TIMEOUT_SEC="${PROBE_TIMEOUT_SEC:-30}"
PROBE_INTERVAL_SEC="${PROBE_INTERVAL_SEC:-1}"
# systemd 串口实际行(带 ANSI 颜色):
#   [ OK ] Reached target basic.target - Basic System.
# 但 ANSI 序列(\033[0;1;39m) 包裹 basic.target,所以用 .* 兼容
# 加载快照后 systemd 直接跳过 basic.target 进入 multi-user 阶段,所以也匹配 multi-user.target
PROBE_LOG_PATTERN="${PROBE_LOG_PATTERN:-Reached target.*(basic\.target|multi-user\.target)}"

# ===== 探测:basic.target =====
# 用法:probe_basic_target <serial_log_path>
# 返回 0 表示达成;非 0 表示超时
probe_basic_target() {
    local log="$1"
    local elapsed=0
    require_file "$log"
    info "等待 systemd basic.target 达成(超时 ${PROBE_TIMEOUT_SEC}s)"
    while [ $elapsed -lt "$PROBE_TIMEOUT_SEC" ]; do
        if grep -qE "$PROBE_LOG_PATTERN" "$log" 2>/dev/null; then
            info "basic.target 达成(${elapsed}s)"
            return 0
        fi
        sleep "$PROBE_INTERVAL_SEC"
        elapsed=$(( elapsed + PROBE_INTERVAL_SEC ))
    done
    warn "等待 basic.target 超时"
    return 1
}

# ===== 探测:进程存活 =====
probe_pid_alive() {
    local pid="$1"
    [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

# ===== 探测:主机网络可达性 =====
probe_network() {
    local host_port="$1"
    timeout 5 bash -c "exec 3<>/dev/tcp/127.0.0.1/$host_port" 2>/dev/null
}

# ===== 通用 grep =====
probe_log_pattern() {
    local log="$1" pattern="$2"
    grep -qE "$pattern" "$log" 2>/dev/null
}