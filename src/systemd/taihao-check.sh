#!/bin/sh
# Taihao OS service integration check script
# 跑 4 阶段验收:is-active + hal 白名单 + extension 越权 + rt-loop 频率
# 输出通过 tee 同时到 /var/log/taihao-os/check.log(文件) + console(qemu 串口 → host serial.log)

set -u

OUT=/var/log/taihao-os/check.log
mkdir -p "$(dirname "$OUT")"

# 选 nc 工具:优先 perl(IO::Socket::UNIX,系统内建),其次 socat
# busybox nc 不支持 Unix socket client 模式;netcat-traditional 不一定有
TOOL="none"
if command -v perl >/dev/null 2>&1; then
    TOOL="perl"
elif command -v socat >/dev/null 2>&1; then
    TOOL="socat"
fi

# 用法:nc_call <sock_path> <timeout_sec>
# 输入:stdin,输出:stdout
nc_call() {
    local sock="$1" timeout="$2"
    if [ "$TOOL" = "perl" ]; then
        # perl 端:写 stdin 一行,select-read 直到 timeout 或 EOF,关闭连接
        perl -e '
            use IO::Socket::UNIX;
            use IO::Select;
            use POSIX qw(:sys_wait_h);
            my $sock = $ARGV[0]; my $timeout = $ARGV[1];
            my $s = IO::Socket::UNIX->new(Peer => $sock, Timeout => $timeout, Blocking => 1);
            unless ($s) { print STDERR "perl: cannot connect $sock: $!\n"; exit 1; }
            $s->autoflush(1);
            my $sel = IO::Select->new($s);
            my $deadline = time() + $timeout;
            my $buf = "";
            my $need_write = 1;
            eval {
                local $SIG{ALRM} = sub { die "timeout\n" };
                alarm $timeout;
                while (time() < $deadline) {
                    my @ready = $sel->can_write(0.1);
                    if ($need_write && @ready) {
                        my $in = <STDIN>;
                        $s->syswrite($in) if defined $in;
                        $need_write = 0;
                    }
                    @ready = $sel->has_exception(0.1);
                    last if @ready;
                    @ready = $sel->can_read(0.1);
                    if (@ready) {
                        my $n = $s->sysread($buf, 4096);
                        last if !defined($n) || $n == 0;
                    }
                }
            };
            alarm 0;
            print $buf;
            close $s;
        ' "$sock" "$timeout"
    elif [ "$TOOL" = "socat" ]; then
        timeout "$2" socat - "UNIX-CONNECT:$1" 2>&1
    else
        return 127
    fi
}

run() { tee -a "$OUT"; }

echo "=== Taihao OS service contract check 1/4: is-active ===" | run
for s in comm-center extension-bridge hal-gateway pi-agent rt-loop; do
    printf "  %-20s " "$s" | run
    systemctl is-active "$s" | run
done
echo | run

echo "=== 2/4: hal-gateway 白名单 ===" | run
HAL_SOCK="/run/taihao-os/hal-gateway.sock"
echo "  socket: $(ls -la $HAL_SOCK 2>&1)" | run
echo "  nc_tool: $TOOL" | run
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
# 1) 尝试 8082 健康端点(两次取样,由 cycles 差 / 时间差 算实测 Hz)
rt_http() {
    perl -e '
        use IO::Socket::INET;
        use IO::Select;
        my $port = $ARGV[0]; my $timeout = $ARGV[1];
        my $s = IO::Socket::INET->new(PeerAddr => "127.0.0.1", PeerPort => $port, Proto => "tcp", Timeout => $timeout) or die "connect fail\n";
        $s->autoflush(1);
        my $sel = IO::Select->new($s);
        my $buf = "";
        my @ready = $sel->can_read($timeout);
        if (@ready) { my $n = $s->sysread($buf, 4096); }
        close $s;
        print $buf;
    ' "$1" "$2" 2>/dev/null
}
H1=$(rt_http 8082 2)
if [ -n "$H1" ]; then
    C1=$(echo "$H1" | grep -oE '"cycles":[0-9]+' | cut -d: -f2 | head -1)
    T1=$(date +%s.%N)
    sleep 5
    H2=$(rt_http 8082 2)
    C2=$(echo "$H2" | grep -oE '"cycles":[0-9]+' | cut -d: -f2 | head -1)
    T2=$(date +%s.%N)
    if [ -n "$C1" ] && [ -n "$C2" ]; then
        HZ=$(awk -v c1="$C1" -v c2="$C2" -v t1="$T1" -v t2="$T2" 'BEGIN { printf "%.1f", (c2-c1)/(t2-t1) }')
        echo "  health#1: $H1" | run
        echo "  health#2: $H2" | run
        echo "  实测频率: ${HZ} Hz(取样 5s,目标 10-100Hz)" | run
    else
        echo "  health 解析失败: $H1" | run
    fi
else
    echo "  health: (8082 不可达,改用 journalctl 时间戳统计)" | run
    # 2) 退化:取心跳两行时间戳 + cycle 差,算实测 Hz
    J_RAW=$(journalctl -u rt-loop --no-pager -n 300 2>/dev/null | grep 'rt-loop 心跳' | tail -10)
    if [ -n "$J_RAW" ]; then
        echo "  心跳采样(尾 3 行):" | run
        echo "$J_RAW" | tail -3 | sed 's/^/    /' | run
        FIRST_TS=$(echo "$J_RAW" | head -1 | awk '{print $1" "$2" "$3}')
        LAST_TS=$(echo "$J_RAW" | tail -1 | awk '{print $1" "$2" "$3}')
        FIRST_CYCLE=$(echo "$J_RAW" | head -1 | grep -oE 'cycles=[0-9]+' | cut -d= -f2)
        LAST_CYCLE=$(echo "$J_RAW" | tail -1 | grep -oE 'cycles=[0-9]+' | cut -d= -f2)
        if [ -n "$FIRST_TS" ] && [ -n "$LAST_TS" ] && [ -n "$FIRST_CYCLE" ] && [ -n "$LAST_CYCLE" ]; then
            F1=$(date -d "$FIRST_TS" +%s.%N 2>/dev/null)
            F2=$(date -d "$LAST_TS" +%s.%N 2>/dev/null)
            HZ=$(awk -v c1="$FIRST_CYCLE" -v c2="$LAST_CYCLE" -v t1="$F1" -v t2="$F2" 'BEGIN { if (t2>t1 && (t2-t1)>0) printf "%.1f", (c2-c1)/(t2-t1); else print "N/A" }')
            echo "  首条 cycle=$FIRST_CYCLE @ $FIRST_TS" | run
            echo "  末条 cycle=$LAST_CYCLE @ $LAST_TS" | run
            echo "  实测频率: ${HZ} Hz(目标 10-100Hz)" | run
        else
            echo "  时间戳解析失败(采样数不足)" | run
        fi
    else
        echo "  暂无心跳日志" | run
    fi
fi
echo | run

echo "=== 验收完成 ===" | run