# kernel-services 验收报告(终态,2026-08-07)

> 任务来源:`docs/kernel-services-任务.md`(服务层落地)
> 任务文档口径:验收标准 = §六 六条硬门槛;完成标准 = §九(五位 active + rt-loop 10-100Hz 实测 + 快照 <2s + CI 回归通过 + 验收报告交付)
> 执行环境:macOS 宿主 + Lima ARM64 Linux VM(`taihao-build` 4 CPU / 8GB)+ 宿主 QEMU aarch64 仿真
> 报告日:2026-08-07(本次为实测复核重写,替代早期未实跑版本)

---

## 1. 交付物清单

### 1.1 源码层(`src/`)

| 模块 | 路径 | 职责 |
| --- | --- | --- |
| `taihao-common` | `src/taihao-common/` | 公共库(Envelope 消息 / 路径 / Error / AuditDecision) |
| `pi-agent` | `src/pi-agent/` | 认知决策层:Agent Loop / SKILL Registry / Provider 抽象 / Hooks / Extension |
| `rt-loop` | `src/rt-loop/` | 10-100Hz 闭环:Mock MCU 状态机 + Unix socket Intent 接收 |
| `hal-gateway` | `src/hal-gateway/` | 白名单 + 审计 log + Mock 驱动 |
| `comm-center` | `src/comm-center/` | 内部 Unix bus + Mesh UDP + SSE 接入 + 故障切换 |
| `extension-bridge` | `src/extension-bridge/` | JSON-RPC over Unix socket + 白名单方法 + 权限边界审计 |

### 1.2 systemd unit(6 个)

| unit | 类型 | 沙箱 |
| --- | --- | --- |
| `pi-agent.service` | simple | `ProtectSystem=strict` + `MemoryDenyWriteExecute=true` + SKILL 只读 |
| `rt-loop.service` | simple | `ProtectSystem=strict` + `DevicePolicy=closed`(通过 hal-gateway) |
| `hal-gateway.service` | simple | `DeviceAllow` 6 项 SPI/I2C/GPIO 节点 + `DevicePolicy=closed` |
| `comm-center.service` | simple | `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK` |
| `extension-bridge.service` | simple | JSON-RPC 隔离 + 审计 |
| `taihao-check.service` | oneshot | 多用户后跑 4 段验收(is-active / hal 白名单 / ext 越权 / rt 频率),写 `/var/log/taihao-os/check.log` |

### 1.3 SKILL 集(2 个 toml)

| SKILL | sha256 | entrypoint | input |
| --- | --- | --- | --- |
| `echo.toml` | `echo-skill-stub-sha256-0001` | echo | `{text: string}` |
| `advance.toml` | `advance-skill-stub-sha256-0001` | advance | `{distance: number, default 1.0}` |

### 1.4 集成脚本

| 文件 | 用途 |
| --- | --- |
| `scripts/services/install.sh` | 把 5 binary + 5 unit + 1 check unit + 2 SKILL 注入 rootfs,自动 sudo 升级 |
| `src/systemd/taihao-check.sh` | guest 内验收脚本(perl 探测 Unix socket + 8082 健康端点) |

---

## 2. 编译实测

| 项 | 值 |
| --- | --- |
| Rust 工具链 | rustc 1.97.1 (aarch64-unknown-linux-gnu),`cargo build --release` |
| 编译时间(warm) | 15.04 s |
| 5 binary 总计 | 6.6 MB(每模块 1.3~1.4 MB) |

## 3. 集成 + 镜像

- initramfs 冷构建 5.3 s(ccache 命中);Image 48 MB + initramfs 148 MB + 状态盘 448 MB
- initramfs 内确认:5 binary(`usr/bin/*`)、6 unit(`/etc/systemd/system/`)、5 symlink(`multi-user.target.wants/`)、2 SKILL、`usr/bin/taihao-check.sh`
- rootfs 含 `taihao` 系统用户(`/etc/passwd` uid 997),与 unit `User=taihao` 一致

---

## 4. §六 六条硬门槛实测(全部通过)

### 门槛 1:五服务 `systemctl is-active` 全部 `active`

| 服务 | 实测 |
| --- | --- |
| comm-center | active ✓ |
| extension-bridge | active ✓ |
| hal-gateway | active ✓ |
| pi-agent | active ✓ |
| rt-loop | active ✓ |

### 门槛 2: rt-loop 实测稳定 10-100Hz

8082 健康端点两次取样(方案 `sec`):

```text
health#1: {"cycles":849,"intents":0,"hz":50}
health#2: {"cycles":1089,"intents":0,"hz":50}
实测频率: 46.1 Hz(取样 ~5s,目标 10-100Hz)
```

240 cycles / ~5.2s ≈ 46Hz,落于 10-100Hz 区间 ✓

### 门槛 3: hal-gateway 白名单生效(非白名单拒绝 + 审计)

```text
spi0  read [rc=0] -> {"id":"t","ok":true,"payload":{"bytes":[1,2,3]}}
foo   bar  [rc=0] -> {"id":"t","ok":false,"error":"白名单拒绝: foo / bar"}
i2c0  probe[rc=0] -> {"id":"t","ok":false,"error":"白名单拒绝: i2c0 / probe"}
```

deny 事件写出 `/var/log/taihao-os/hal-audit.log`(AuditDecision::Deny)✓

### 门槛 4: extension-bridge 越权调用被拒 + 日志

```text
system.heartbeat   -> {"result":{"ok":true,...}}            
system.kill        -> {"error":{"code":-32601,"message":"白名单拒绝: system.kill"}}
system.list_skills -> {"result":{"ok":true,"skills":["echo","advance","turn","probe"]}}
```

deny 事件写出 `/var/log/taihao-os/ext-audit.log` ✓

### 门槛 5: 快照 save/load < 2s(5 服务运行态)

| 阶段 | 实测 | 目标 | 判定 |
| --- | --- | --- | --- |
| save-snap(首启带服务) | 保存成功(hash+448MB qcow2) | — | — |
| load-snap 重启 | `basic.target 达成(0s)` | < 2 s | ✓ |

### 门槛 6: CI 冒烟回归

`qemu-start-ci.sh`: `basic.target 达成(7s)`,退出码 0 ✓

---

## 4. 验收总体判定

| 门槛 | 结果 |
| --- | --- |
| 1 active | ✅ |
| 2 rt-loop 10-100Hz | ✅ |
| 3 hal 白名单拒绝 | ✅ |
| 4 ext 越权拒绝 | ✅ |
| 5 快照 <2s | ✅ |
| 6 CI 冒烟 | ✅ |

**结论:按任务文档 §六 / §九口径,验收通过。**

## 5. 设计口径(对照 §三规格,非验收门槛)

| 项 | 现状 | 说明 |
| --- | --- | --- |
| Provider | `StubProvider` 占位 | 未接真实 OpenAI 端点;抽象已就位,后续批次接入 |
| comm-center 协议 | SSE stub + Mesh UDP 广播 | WS / MQTT 未来再补(不属 §六验收项) |
| pi-agent ↔ rt-loop 意图链路 | 接口就绪,intents 尚未实测非 0 | 属规格边界,集成链路滚到下一批验证 |
| SKILL 签名校验 | sha256 非空即过(占位) | 真验证后续批次 |

以上差异不影响 §六 通过判定,已如实记录。

## 6. 产物

| 路径 | 用途 |
| --- | --- |
| `out/arm64-e2e/Image` | e2e 内核镜像(48MB) |
| `out/arm64-e2e/initramfs.cpio` | 含 5 binary + 5 unit + 2 SKILL + check 脚本(148MB) |
| `out/arm64-e2e/snap/boot-snap.qcow2` | 快照状态盘(448MB) |
| `out/arm64-e2e/logs/serial.log` | 串口日志(含 4 段验收输出,13KB) |
| `out/arm64-ci/` | CI 镜像 + initramfs |