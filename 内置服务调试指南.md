# 太昊 OS 内置服务调试指南

> 本文档面向太昊 OS 部署后的运行时调试,覆盖 5 个内置服务(pi-agent / comm-center / extension-bridge / hal-gateway / rt-loop)的端口、协议、测试方法、实时观察与常见问题排查。

## 1. 服务总览

| 服务 | TCP 端口 | Unix socket | 协议 | 角色 |
|---|---|---|---|---|
| **pi-agent** | 8081 | `/run/taihao-os/comm-center.sock`(发) | HTTP `GET /` | 认知决策层(Agent Loop,10Hz) |
| **comm-center** | 8084 (SSE) / 8086 (WS) / 7685 (UDP) | `/run/taihao-os/comm-center.sock` | HTTP/SSE/WebSocket/UDP | 调度通信中心(内部总线 + 双链路) |
| **hal-gateway** | 8083 | `/run/taihao-os/hal-gateway.sock` | HTTP | 硬件访问网关(白名单 + 审计) |
| **extension-bridge** | 8085 | `/run/taihao-os/extension-bridge.sock` | HTTP | 扩展 / MCP 桥(JSON-RPC + 权限边界) |
| **rt-loop** | 无端口 | 无 | 内部线程 | 执行层回路(10-100Hz 实时闭环) |

所有服务以用户 `taihao` 运行,运行时目录 `/run/taihao-os/`,状态目录 `/var/lib/taihao-os/`,日志目录 `/var/log/taihao-os/`。

## 2. 基础:服务状态与日志

```bash
# 全部 5 服务 active 状态(systemd)
systemctl status pi-agent comm-center extension-bridge hal-gateway rt-loop

# 单个服务实时日志
journalctl -u pi-agent -f
journalctl -u comm-center -f

# 所有 5 服务最近 200 条日志
journalctl -u pi-agent -u comm-center -u extension-bridge -u hal-gateway -u rt-loop -n 200 --no-pager

# 服务二进制与运行参数
systemctl show pi-agent | grep -E 'ExecStart|User|WorkingDirectory|Environment'
```

## 3. 各服务探活 / 探测命令

### 3.1 pi-agent(端口 8081,HTTP)

```bash
# 基本探活
wget -qO- http://127.0.0.1:8081/
# 预期: {"ok":true,"intents":<N>}

# 实时观察 Agent Loop(intents 数字每秒递增)
watch -n 1 'wget -qO- http://127.0.0.1:8081/'

# 自定义端口(PI_AGENT_PORT env)
PI_AGENT_PORT=9090 systemctl restart pi-agent
wget -qO- http://127.0.0.1:9090/
```

**`intents` 数字含义:** Agent Loop 每 100ms 跑一拍,累加到 `intents_count()`。数字随时间递增即代表 10Hz Agent Loop 正常运转。

### 3.2 comm-center(端口 8084 SSE / 8086 WS / 7685 UDP)

```bash
# SSE 端点(8084)
wget -qO- http://127.0.0.1:8084/

# WebSocket 端点(8086)
wget -qO- http://127.0.0.1:8086/

# Mesh UDP 端点(7685)
busybox nc -u 127.0.0.1 7685 -w 1 < /dev/null

# comm-center 内部总线 socket 检查
ls -la /run/taihao-os/comm-center.sock
# 预期: srw-rw---- 1 taihao taihao ... /run/taihao-os/comm-center.sock
```

### 3.3 hal-gateway(端口 8083,HTTP)

```bash
# 探活
wget -qO- http://127.0.0.1:8083/

# 白名单调用(hal-gateway 仅放行白名单操作,如 spidev/i2c-dev)
# 具体路径和 payload 取决于部署时的白名单配置
# 查看白名单:
cat /etc/taihao-os/hal-whitelist.json 2>/dev/null
```

### 3.4 extension-bridge(端口 8085,HTTP)

```bash
# 探活
wget -qO- http://127.0.0.1:8085/

# 越权测试(应被拒绝,白名单 + 权限边界验证)
curl -X POST http://127.0.0.1:8085/invoke -d '{"action":"unauthorized"}'

# JSON-RPC 调用示例(具体取决于 bridge 配置)
curl -X POST http://127.0.0.1:8085/rpc -H 'Content-Type: application/json' \
  -d '{"method":"echo","params":{"msg":"hello"}}'
```

### 3.5 rt-loop(无端口,内部线程)

```bash
# 频率验证:rt-loop 应在 10-100Hz(每秒 10-100 tick)
journalctl -u rt-loop -n 200 -f | grep -i 'tick\|hz\|rate'

# 验证实时回路是否在跑:看日志中是否有周期性 tick
watch -n 1 'journalctl -u rt-loop -n 5 --no-pager | tail -3'

# rt-loop 进程 CPU 占用
top -bn1 -p $(pgrep -d',' rt-loop)
```

## 4. 自动集成验收: taihao-check

`/usr/sbin/taihao-check` 是 buildroot 中预置的 4 阶段验收脚本:

1. **is-active** — 5 服务 `systemctl is-active` 检查
2. **hal 白名单** — hal-gateway 白名单路径调用
3. **extension 越权** — extension-bridge 越权调用(应被拒绝)
4. **rt-loop 频率** — 统计 rt-loop tick 频率

```bash
# 立即跑一次
systemctl start taihao-check

# 看结果(同时输出到 console + /var/log/taihao-os/check.log)
cat /var/log/taihao-os/check.log

# 跟随日志
tail -f /var/log/taihao-os/check.log

# 跑完后服务会退出,systemctl 状态 inactive(dead)
systemctl status taihao-check
```

**`taihao-check` 退出码:**
- `0` — 4 阶段全部通过
- 非 0 — 至少一项失败,看日志具体哪一项

## 5. 内置 SKILL 与资源目录

```bash
# SKILL 目录(只读)
ls -la /usr/share/taihao-os/skills/

# 配置文件目录(只读)
ls -la /etc/taihao-os/

# 运行时持久化状态(可写)
ls -la /var/lib/taihao-os/

# 日志
ls -la /var/log/taihao-os/

# 运行时 socket 与目录
ls -la /run/taihao-os/
```

## 6. 实时观察 Agent Loop 与回路(最常用调试)

```bash
# 1. pi-agent intents 实时增长(确认 Agent Loop 10Hz 运转)
watch -n 1 'wget -qO- http://127.0.0.1:8081/'

# 2. rt-loop 频率实时观察
journalctl -u rt-loop -f --since "1 min ago"

# 3. 5 服务状态实时观察
watch -n 2 'systemctl is-active pi-agent comm-center extension-bridge hal-gateway rt-loop'

# 4. 端到端:5 服务 + taihao-check + watch 一起
( systemctl start taihao-check && tail -f /var/log/taihao-os/check.log ) & \
watch -n 1 'wget -qO- http://127.0.0.1:8081/ http://127.0.0.1:8083/ http://127.0.0.1:8085/'
```

## 7. 常见问题排查

### 7.1 端口没监听 / wget 超时

```bash
# 1. 确认服务在跑
systemctl status <service>

# 2. 看监听端口
ss -tlnp | grep -E '8081|8083|8084|8085|8086'

# 3. 看服务日志
journalctl -u <service> -n 100 --no-pager
```

### 7.2 `intents` 数字不增长(Agent Loop 卡死)

```bash
# 看 pi-agent 内部状态
journalctl -u pi-agent -n 200 -f

# 看是否依赖项断了
systemctl status comm-center extension-bridge

# 重启 pi-agent
systemctl restart pi-agent
```

### 7.3 hal-gateway 白名单拒绝

```bash
# 查白名单配置
cat /etc/taihao-os/hal-whitelist.json

# 查 hal-gateway 日志
journalctl -u hal-gateway -n 100 -f

# 允许 / 拒绝的请求都记录在 journal
journalctl -u hal-gateway -g 'allow\|deny'
```

### 7.4 extension-bridge 越权

```bash
# 查权限边界配置
cat /etc/taihao-os/extension-policy.json 2>/dev/null

# 越权请求应返回 403,日志记录
journalctl -u extension-bridge -n 200 -f
```

### 7.5 rt-loop 频率异常

```bash
# 应在 10-100Hz(每 10-100ms 一次 tick)
journalctl -u rt-loop -n 200 --no-pager

# 期望看到周期性 tick 日志(间隔 ~10-100ms)
journalctl -u rt-loop --output=cat | tail -100

# 如果没有 tick 或卡住:可能依赖项断了或回路 hang
systemctl status comm-center hal-gateway
```

### 7.6 内置服务全部 fail

```bash
# 看 5 服务是否都被 masked / 依赖损坏
systemctl list-unit-files | grep -E 'pi-agent|comm-center|extension-bridge|hal-gateway|rt-loop'

# 看 default.target
systemctl get-default
systemctl list-dependencies multi-user.target | grep -E 'taihao|pi-agent|comm-center'

# 重新拉起
systemctl daemon-reload
systemctl reset-failed
systemctl start pi-agent comm-center extension-bridge hal-gateway rt-loop
```

## 8. 配置文件位置速查

| 路径 | 内容 |
|---|---|
| `/usr/bin/{pi-agent,comm-center,extension-bridge,hal-gateway,rt-loop}` | 5 服务二进制 |
| `/usr/sbin/taihao-login` | 自定义登录(替换 agetty) |
| `/usr/sbin/taihao-check` | 4 阶段集成验收脚本 |
| `/etc/taihao-os/` | 全局配置(白名单 / 权限边界 / 服务参数) |
| `/usr/share/taihao-os/skills/` | SKILL 集合(只读) |
| `/var/lib/taihao-os/` | 运行时持久化 |
| `/var/log/taihao-os/` | 审计 + 集成验收日志 |
| `/run/taihao-os/` | 运行时 socket 与目录(系统重启清空) |
| `/etc/systemd/system/console-getty.service.d/10-taihao-login.conf` | tty1 登录替换为 taihao-login |
| `/etc/systemd/system/serial-getty@.service.d/10-taihao-login.conf` | 串口登录替换 |
| `/etc/systemd/system/getty@.service.d/10-taihao-login.conf` | ttyN 通用登录替换 |

## 9. 环境变量覆盖(可调端口 / 配置)

| 服务 | 环境变量 | 默认值 |
|---|---|---|
| pi-agent | `PI_AGENT_PORT` | 8081 |
| comm-center | `SSE_PORT` / `WS_PORT` / `MESH_UDP_PORT`(源码常量) | 8084 / 8086 / 7685 |
| hal-gateway | `HAL_GATEWAY_PORT` | 8083 |
| extension-bridge | `EXT_BRIDGE_PORT` | 8085 |
| rt-loop | 无 | 内部 |

```bash
# 示例:把 pi-agent 端口改为 9090
systemctl edit pi-agent
# 添加:
# [Service]
# Environment=PI_AGENT_PORT=9090
systemctl daemon-reload
systemctl restart pi-agent
wget -qO- http://127.0.0.1:9090/
```

## 10. 一行速查合集(粘到终端)

```bash
# 一键查看 5 服务 + 端点 + intents + taihao-check
(
  echo "=== systemctl is-active ===" ;
  for s in pi-agent comm-center extension-bridge hal-gateway rt-loop; do
    printf "  %-20s %s\n" "$s" "$(systemctl is-active $s 2>&1)" ;
  done ;
  echo "=== /run/taihao-os/ sockets ===" ;
  ls -la /run/taihao-os/ ;
  echo "=== ports ===" ;
  ss -tlnp 2>/dev/null | grep -E '8081|8083|8084|8085|8086|7685' ;
  echo "=== pi-agent intents ===" ;
  wget -qO- http://127.0.0.1:8081/ ;
  echo ;
  echo "=== taihao-check.log tail ===" ;
  tail -20 /var/log/taihao-os/check.log 2>/dev/null
)
```
