#!/bin/bash
# 太昊 OS · qemu 冒烟启动(CI 烟测 · 无窗口 · 等 basic.target)
#
# 这是 scripts/kernel-build/qemu-start-ci.sh 的薄包装,统一从 scripts/run/ 入口调用
#
# 用法:
#   scripts/run/qemu冒烟启动.sh
#
# 退出:
#   0 = basic.target 达成
#   3 = basic.target 超时
#
# 适用场景:
#   - CI 流水线
#   - 只想验证内核能起来,不需窗口交互

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

exec bash "$REPO_ROOT/scripts/kernel-build/qemu-start-ci.sh" "$@"
