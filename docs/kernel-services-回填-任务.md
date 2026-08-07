# 任务:服务层回填与端到端服务集成验证(服务回填批次)

> 执行对象:AI / 人工。承接 `docs/kernel-build-e2e-验收报告.md`「回填位 1」:重建 `src/` 五模块服务,打包进 initramfs 源 rootfs,跑通契约 §2.1 端到端通过标准(5 服务 systemd unit 全 `active`)。

## 一、依据(唯一契约,不引用仓库外文件)

- `<仓库根>/docs/linux-内核裁剪方案.md`(v0.0.8):§2.1 端到端通过标准 = pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge 全套服务 systemd unit `active`;§9 步骤 6
- `<仓库根>/docs/kernel-build-e2e-任务.md` §4.4 服务集成验收(断言位保留,本批次回填)
- `<仓库根>/docs/kernel-build-e2e-验收报告.md` §6/§10 回填位
- `<仓库根>/AGENTS.md` 终端架构认知(两层模型)与开放接入位四槽

## 二、前置条件

| 项 | 要求 | 说明 |
| --- | --- | --- |
| 工具链 | `scripts/kernel-build/` 已交付并验收 | Lima VM 构建 + 宿主 QEMU 验证 |
| 环境 | Lima ARM64 VM(`taihao-build`)+ ccache + Linux 6.6 源码 | 沿用构建集成批次环境 |
| rootfs | mmdebstrap minbase(busybox + systemd,pid1) | 已就绪,回填服务后重打包 |
| 服务源码 | `src/` 五模块 + `examples/skills` 集 | 本批次重建 |

## 三、交付规格

### 3.1 src/ 五模块重建(Rust,与 M3 规格一致)

| 模块 | 职责 | 关键能力 |
| --- | --- | --- |
| `src/pi-agent` | 唯一智能决策层 | Agent Loop / SKILL Registry / Provider(OpenAI 兼容远端端点)/ SKILL 签名校验 / Hooks / Extension 加载 |
| `src/hal-gateway` | 硬件访问网关 | HAL 白名单 + 审计 + mock 驱动(SPI / I2C / GPIO 设备节点) |
| `src/rt-loop` | 执行层回路 | 10~100Hz 指令流下发 / 本体状态回收闭环校调;不承载认知与稳控 |
| `src/comm-center` | 中台调度通信 | SSE / WS / MQTT + Mesh UDP 双链路 + 故障切换 + 消息路由 |
| `src/extension-bridge` | 扩展 / MCP 桥 | JSON-RPC + 白名单 + 权限边界 |

### 3.2 systemd 集成

- 5 个 systemd unit(`pi-agent.service` / `hal-gateway.service` / `rt-loop.service` / `comm-center.service` / `extension-bridge.service`),开机自启,`Restart=on-failure`
- 四层隔离栈沙箱(apparmor 或 systemd 原生 sandboxing 属性)
- SKILL 集:`/usr/share/taihao-os/skills/`(系统级),与 `examples/skills/` 对齐

### 3.3 打包与镜像

- 服务二进制 + unit + SKILL 回填 mmdebstrap rootfs 源目录
- `build-kernel.sh e2e --initramfs-dir <rootfs>` 重建 initramfs(含服务)
- 快照状态盘沿用,服务回填不破坏快照循环

## 四、验收(硬性)

1. **构建链路**:`build-kernel.sh e2e` 产出含服务的 initramfs,Image 仍为 aarch64 ELF
2. **五位全绿**:`qemu-start-e2e.sh` 启动后,guest 内 `systemctl is-active` 五服务全部 `active`,退出码 0
3. **快照不破**:服务运行态下 save-snap → load-snap 循环仍 < 2 s;initramfs 更换后旧快照自动失效重建
4. **rt-loop 指标**:回路频率实测 10~100Hz 区间(guest 内日志或状态上报)
5. **隔离栈**:非白名单 syscall / 路径访问被拒绝(hal-gateway 审计日志 + extension-bridge 权限边界)
6. **回归**:CI 冒烟(`qemu-start-ci.sh`)不受影响,basic.target 达成

## 五、自检(交付前全部通过)

- `cargo build --release`(aarch64 交叉)5 模块零错误
- `bash -n` 相关脚本零错误
- 服务 unit 文件 `systemd-analyze verify` 通过
- 五位 `is-active` 全绿实测记录
- 快照 save / load 循环 ≥ 2 次 + 计时 < 2 s

## 六、交付清单

- `src/` 五模块源码 + `examples/skills/` SKILL 集
- 5 个 systemd unit + 沙箱 profile
- 含服务 initramfs + 实测日志(五位 is-active / 快照计时 / rt-loop 频率)
- 本批次验收记录(回填 e2e 验收报告或独立小节)

## 七、里程碑与顺序

1. `src/` 五模块重建 + 单元自测
2. systemd unit + 隔离栈配置
3. rootfs 回填 + initramfs 重建
4. e2e 启动 → 五位 `is-active` 全绿
5. 快照回归 + rt-loop 指标实测
6. CI 冒烟回归 + 验收记录收尾

## 八、预期

- 理想路径(源码复用 M3 设计):2 ~ 4 h
- 常规路径(隔离栈调试往返):4 ~ 8 h
- 完成标志:五位全绿 + 快照不破 + rt-loop 频率达标 + CI 回归通过