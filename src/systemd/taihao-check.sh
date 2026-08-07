#!/bin/sh
# 太昊 OS 服务集成验证 check 脚本
# 跑 4 阶段验收:is-active + hal 白名单 + extension 越权 + rt-loop 频率
# 输出通过 tee 同时到 /var/log/taihao-os/check.log(文件) + console(qemu 串口 → host serial.log)

set -u

OUT=/var/log/taihao-os/check.log
mkdir -p "$(dirname "$OUT")"

# 选 nc 工具:优先 socat(支持 Unix socket client 模式),其次 netcat-traditional
# busybox nc 不支持 Unix socket client 模式(只支持 server -l -f)
if command -v socat >/dev/null 2>&1; then
    USE_SOCAT=1
else
    USE_SOCAT=0
fi

# 用法:nc_call <sock_path> <timeout_sec>
# 输入:stdin,输出:stdout
nc_call() {
    if [ "$USE_SOCAT" = 1 ]; then
        # socat 语法:socat - UNIX-CONNECT:/path/to/sock,timeout=2
        # stdin 已重定向,直接 exec
        exec 3<>/dev/null  # 无用 trick 防止 set -u 报错
        printf '' | timeout "$2" socat - "UNIX-CONNECT:$1" 2>&1
    elif command -v nc >/dev/null 2>&1; then
        nc -U "$1" -w "$2"
    else
        return 127
    fi
}

run() { tee -a "$OUT"; }

echo "=== 太昊 OS 服务契约验收 1/4: is-active ===" | run
for s in comm-center extension-bridge hal-gateway pi-agent rt-loop; do
    printf "  %-20s " "$s" | run
    systemctl is-active "$s" | run
done
echo | run

echo "=== 2/4: hal-gateway 白名单 ===" | run
HAL_SOCK="/run/taihao-os/hal-gateway.sock"
echo "  socket: $(ls -la $HAL_SOCK 2>&1)" | run
echo "  nc_tool: $(if [ $USE_SOCAT = 1 ]; then echo socat; elif command -v nc >/dev/null 2>&1; then echo nc; else echo none; fi)" | run
for dev_op in "spi0:read" "foo:bar" "i2c0:probe"; do
    dev="${dev_op%%:*}"
    op="${dev_op#*:}"
    req="{\"id\":\"t\",\"actor\":\"check\",\"device\":\"$dev\",\"operation\":\"$op\",\"params\":{}}"
    resp=$(printf '%s\n' "$req" | nc_call "$HAL_SOCK" 2 2>&1)
    rc=$?
    printf "  %-15s %-7s [rc=%d] -> %s\n" "$dev" "$op" "$rc" "$resp" | run
done
echo "  audit log(尾 3 行):" | run
tail -3 /var/log/taihao-os/hal-audit.log 2>/dev/null | sed 's/^/    /' | run
echo | run

echo "=== 3/4: extension-bridge 越权 ===" | run
EXT_SOCK="/run/taihao-os/extension-bridge.sock"
echo "  socket: $(ls -la $EXT_SOCK 2>&1)" | run
for m in "system.heartbeat" "system.kill" "system.list_skills"; do
    req="{\"jsonrpc\":\"2.0\",\"method\":\"$m\",\"id\":1}"
    resp=$(printf '%s\n' "$req" | nc_call "$EXT_SOCK" 2 2>&1)
    rc=$?
    printf "  %-20s [rc=%d] -> %s\n" "$m" "$rc" "$resp" | run
done
echo "  audit log(尾 3 行):" | run
tail -3 /var/log/taihao-os/ext-audit.log 2>/dev/null | sed 's/^/    /' | run
echo | run

echo "=== 4/4: rt-loop 频率 ===" | run
RT_HEALTH=$(curl -s --max-time 2 http://127.0.0.1:8082/ 2>/dev/null || echo "(rt-loop 8082 不可达)")
echo "  $RT_HEALTH" | run
echo | run

echo "=== 验收完成 ===" | run