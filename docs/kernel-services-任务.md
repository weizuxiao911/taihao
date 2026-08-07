# 任务:服务层落地(src/ 五模块 + systemd 集成 + 端到端验证)

> 执行对象:AI(可自主执行,含工具链安装)。本任务将太昊 OS 的设计规格落地为可运行服务层:实现 `src/` 五模块、配套 systemd unit 与 SKILL 集,打包进最小 rootfs,在已交付的构建 / 仿真链路上完成端到端服务验证(契约 §2.1 通过标准)。

## 一、任务范围

只做**服务层**(Linux 侧的智能决策与调度编排),不碰内核(已交付)与 MCU/稳控层(设计认知,未来批次)。服务层分五模块,遵循 AGENTS.md「终端架构认知(两层模型)」的职责边界。

## 二、依据(设计规格,不引用仓库外文件)

- `<仓库根>/AGENTS.md`:
  - §「终端架构认知(两层模型)」:认知决策层(RK3588 + Pi-Agent + 4-7B,1-10Hz)/ 执行层回路(rt-loop,10-100Hz)/ 稳控反射层(MCU,PID)
  - §「开放接入位四槽」:HAL 集 / SKILL 集 / Provider 集 / Extension·MCP 桥
  - §「中台调度通信模块」:调度 + 通信,厂商配置决定对接对象,OS 出厂不预设
  - §「推理抽象层」:OpenAI 兼容协议抽象,隔离推理后端
- `<仓库根>/docs/linux-内核裁剪方案.md` v0.0.8:§2.1 端到端通过标准
- `<仓库根>/docs/kernel-build-e2e-任务.md`:§4.4 服务集成验收流程

## 三、五模块规格(职责边界内)

### 3.1 pi-agent —— 认知决策层载体

- Agent Loop:感知 → 理解 → 决策 → 动作的符号编排循环,1-10Hz 亚秒级迭代
- SKILL Registry:`SKILL.md` 注册 / 加载 / 签名校验(只执行白名单签名 skill)
- Provider:OpenAI 兼容远端端点;通过「推理抽象层」隔离后端
- Hooks:决策前后的扩展点;Extension 加载(经 extension-bridge 接入)
- 边界:只定"要干什么、要去哪里",不下发底层实时微操;Skill 基于 MCU 硬件原子原语编排,由 rt-loop 执行

### 3.2 rt-loop —— 执行层回路

- 接收 Agent 高层意图,生成轨迹 / 指令流下发 MCU
- 回收本体状态,闭环校调;可测量且稳定运行于 10-100Hz
- 边界:指令下发与状态回收的衔接层,不承载认知(1-10Hz)与稳控(MCU 毫秒级)

### 3.3 hal-gateway —— 硬件访问网关

- 白名单:仅放行已登记 HAL 调用(SPI / I2C / GPIO 等 device node)
- 审计:全量访问日志;mock 驱动:仿真环境无真硬件时行为可测
- 请求源自 agent 决策(pi-agent / skill 协同链中)

### 3.4 comm-center —— 中台调度通信模块

- 通信:协议适配;SSE / WS / MQTT + Mesh UDP 双链路
- 调度:任务编排、消息路由、双链路选择、故障切换
- 对接对象由厂商配置决定,OS 出厂不预设(对接岸基控制系统只是它一个 Provider 实例,不写入品牌特征)

### 3.5 extension-bridge —— 扩展 / MCP 桥

- JSON-RPC 协议;白名单方法集;权限边界(越权拒绝进审计)

## 四、运行时规格

| 项 | 规格 |
| --- | --- |
| 进程模型 | 5 独立服务(systemd unit),开机自启 |
| unit | `pi-agent.service` / `hal-gateway.service` / `rt-loop.service` / `comm-center.service` / `extension-bridge.service` |
| 进程隔离 | systemd sandboxing(`ProtectSystem=strict` / `NoNewPrivileges` / `RestrictAddressFamilies` 等) |
| 权限 | 最小化;hal-gateway 才具设备访问;其余只经文件 socket / 本机地址访问 |
| 数据路径(设计) | `/etc/taihao-os/`(配置)/ `/usr/share/taihao-os/skills/`(系统级 SKILL)/ `/var/lib/taihao-os/`(持久化)/ `/var/log/taihao-os/`(审计) |

## 五、端到端验证链路(复用已交付构建 / 仿真)

| 环节 | 命令 | 通过标准 |
| --- | --- | --- |
| 服务编译 | `cargo build --release`(aarch64 交叉) | 5 模块各目标编译零错误 |
| rootfs 集成 | 服务二进制 + unit + SKILL 装入最小 rootfs 源目录 | 目录树完整 |
| initramfs 重建 | `scripts/kernel-build/build-kernel.sh e2e --initramfs-dir <rootfs>` | Image aarch64 ELF + 含服务 initramfs |
| 镜像启动 | `scripts/kernel-build/qemu-start-e2e.sh` | systemd `basic.target` 达成 |
| 服务校验 | guest 内 `systemctl is-active pi-agent ...` | 五服务全部 `active`(契约 §2.1) |
| 快照回归 | `--save-snap` → 重启 `load-snap` | save / load 成功,重启 < 2 s |
| 快照失效 | 更换 initramfs 后再启动 | 自动判定失效并重建 |
| CI 回归 | `qemu-start-ci.sh`(不含 9p 快照) | `basic.target` 达成,退出码 0 |

## 六、验收门槛(硬性)

1. 五服务上电后 `systemctl is-active` 全部 `active`(契约 §2.1 通过标准)
2. rt-loop 实测稳定 10-100Hz(guest 内日志 / 状态上报佐证)
3. hal-gateway 白名单生效:非白名单访问被拒并出审计
4. extension-bridge 权限边界:越权调用被拒并出日志
5. 快照 save / load 循环在 5 服务运行态下 < 2s,变更镜像/initramfs 自动失效重建
6. CI 冒烟回归通过(不影响既有链路)

## 七、约束(工程纪律)

- 不入库凭证,敏感项走环境变量注入(rootfs 构建期不落盘)
- 运行数据不进 repo,走`.gitignore`
- systemd unit 与完整说明中文;SKILL.md 格式对齐契约
- 不重启基础设施、不修改内核契约基线、不动 QEMU 构建链路(它们是交付态)

## 八、里程碑与顺序

1. `src/` 骨架 + 五模块交叉编译(aarch64)
2. unit + sandboxing 配置
3. rootfs 连同服务二进制打包 → initramfs 重建
4. e2e 启动 → 验证 basic.target → 五服务 active
5. rt-loop 频率 / hal 白名单 / extension 边界实测
6. 快照回归 + CI 回归 + 验收件定稿

## 九、预期

- 串联一次性通过:2 ~ 4 h
- 含调试往返:4 ~ 8 h
- 完成标准:五位 `active` + rt-loop 10-100Hz 实测 + 快照 <2s + CI 回归通过 + 验收报告交付