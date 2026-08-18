# AGENTS.md

> 太昊 OS (Taihao OS) 的 AI 协作维护规范。
> 本文件不是产品文档，不写设计逻辑；只写工程维护约束与 AI 接管契约。

## 项目定位

太昊 OS：智能终端 / 机器人设备端 OS；三段式骨架（Linux Core + Pi Agent + 开放接入位）。完整设计见 [`README.md`](./README.md)。

## 协作分工

- **用户**：控设计 + 决策
- **AI（我）**：主执行
- 任何需要用户判断的决策点一律走 `question` 工具；不在行内问、不靠猜
- 不擅自做超出已确认范围的动作（特别是仓库迁移、删除、强制更新、远端写入、commit / push / tag / 发布）
- 用户口头/文字上下文只作认知输入，不写入文档

## 文档职责分层

- `README.md`：人看，太昊 OS 的项目门面
- `AGENTS.md`（本文件）：AI 看，工程维护约束 + AI 接管契约
- 其他 docs / 设计文档 / 调研产物：**暂不维护**；将来如需补充，等用户明示再开

## 不引用任何外部文件

未经用户明示：

- 不引用本仓库外的文件（其他项目、其他工作区目录、外部 URL）
- 不引用本仓库未来可能存在的子模块文档、设计文档、调研产物
- 唯一的内部引用是 README↔AGENTS 互指（两份门面互为入口）

## 文档格式规约

所有设计文档（如 `内置服务设计.md` 及后续同类文档）必须按以下模板编写，AI 维护者**自动遵循，无需用户每次重复**。

### 元信息表（文档头）

```markdown
| 字段 | 值 |
|---|---|
| 版本 | v{大}.{小}.{修订} |
| 范围 | （本文档覆盖的内容） |
| 受众 | （谁看本文） |
| 变更内容 | （修订摘要，AI 自动维护） |
```

**版本号规则**：
- 起始版本：`v0.0.1`（初始草稿）
- **大版本 / 小版本**变更必须由用户单次授权才能进行
- **修订版本**由 AI 自动 +1 维护（每次修订提交）
- 修订必须更新"变更内容"行（简述改动）

### 章节结构

```markdown
# 文档名

（上面元信息表）

## 1. 总体设计
### 1.1 背景与目的
### 1.2 设计原则
### 1.3 ...（规范 / 机制 / 模板等）

## 2. xxx 服务
### 2.1 设计思路
### 2.2 ...（详细设计）

## N. 参考
```

**规则**：
- 单一 H1（文档名）
- `## 1. 总体设计` 固定为第 1 章，下分 1.1 / 1.2 / ... 写规范 / 原则 / 机制
- `## 2. xxx 服务` 起按服务展开，每服务一节（编号递增）
- 图表用 Mermaid（无 `style` / `classDef` / Emoji）
- 内容以**当前最终态**方式描述，不写"v0.1 / 上一版 / 整改"等历史化措辞

## 铁律

1. **元规则**：所有文档以**当前最终态**方式描述，不写"v0.1 / 上一版 / 整改 / 重定位 / 补丁"等历史化措辞；变更日志除外
2. **一次性大重写**：修改任何文件必须一次性大重写到终态，禁止通过叠加补丁逐步逼近
3. **README 一致性**：修改本文件时核对 README.md 对应章节同步；修改 README 时核对本文件对应章节同步
4. **Mermaid**：图表使用 Mermaid 代码块，默认主题；不写 `style` / `classDef` / Emoji
5. **凭证安全**：严禁把凭证、API Key、Token、运行时数据写入仓库；敏感数据走 `.gitignore` 排除路径
6. **中文优先**：文档、接口说明、用户可见文案以中文为主；首次出现的英文缩略语需给出全称

## 命名 + 术语定义

### 品牌

- 展示名：**太昊 OS**
- 仓库名：`taihao`
- 拼音规则：全小写、无分隔符
- 许可证：Apache-2.0

### 技术选型

| 项 | 选择 | 理由 |
| --- | --- | --- |
| 目标硬件 | RK3588 同级别（Cortex-A76/A55 + NPU ≥ 6TOPS） | 16GB eMMC + 4GB RAM 约束；NPU 支持本地 7B 推理 |
| 架构 | arm64（真机）+ amd64（开发 / QEMU 仿真） | 双架构 |
| 内核 | Linux 6.6 LTS + PREEMPT_RT | 实时回路 10~100Hz；RT patch 维护良好 |
| 基座 OS | Buildroot | 16GB eMMC 体积约束；RKNN / ONNX / llama.cpp / Mosquitto 可全编入；无 apt 依赖 |
| 烧录 | dd（工厂）+ OTA（部署后） | A/B 双系统分区 + 失败回滚 |
| 仿真 | QEMU amd64 + aarch64 | CI 强制；无需硬件可验证 |

### 三段式骨架

| 术语 | 含义 |
| --- | --- |
| **Linux Core** | OS 底座层；裁剪内核 + 四层隔离栈 + HAL 网关 + 高频实时回路 + 三分区 |
| **Pi Agent** | 唯一智能决策层；Agent Loop / Skill Registry / Extension / Hooks；AGENT 内核采用 `badlogic/pi-mono`（仅技术内核选型参考，不构成品牌词） |
| **开放接入位** | 垂直差异承载位；四槽见下 |

### 开放接入位四槽

| 槽 | 承载内容 |
| --- | --- |
| **HAL 集** | 硬件驱动（spidev / i2c-dev / V4L2 / SocketCAN / pwmchip / serial / 水声通信机 / …） |
| **SKILL 集** | 策略 / 任务 / 自治能力 SKILL.md 包；SKILL.md 格式兼容 Anthropic Agent Skills 规范 |
| **Provider 集** | 模型 / 工具 / 调度通信中心的 Provider；任意 OpenAI 兼容端点 |
| **Extension·MCP 桥** | 扩展与 MCP 桥接，接入自研边缘能力 |

### 调度通信中心（comm-center）

- 位置：Pi Agent 与外部系统之间，作为接入位实现
- 职责：调度（任务编排、消息路由、双链路选择、故障切换）+ 通信（协议适配）
- 对接对象：厂商配置决定，OS 出厂不预设

### 不使用 / 不绑定的词

- 不把"对接岸基控制系统"作为品牌特征写入文档——这只是调度通信中心的一个 Provider 实例
- 不绑定具体终端形态
- 不引用其他项目的命名（taichu / taixu / taiyee / taiyee-pptx-skill / taishi 等）
- 不写"起点 / 终点"废话——OS 镜像是什么就是什么，不区分起始与目标形态

## 终端架构认知（两层模型，仅作认知，未来实现时遵循）

> 终端侧职责严格分层：认知决策与实时稳控分离，中间由执行层回路衔接。

| 层 | 载体 | 职责 | 循环频率 | 边界 |
| --- | --- | --- | --- | --- |
| **认知决策层（大脑皮层）** | RK3588 同级别 + Pi-Agent + 4-7B 多模态模型 | 环境感知 / 任务理解 / Skill 符号编排 / 高层运动意图 | 1-10Hz（亚秒级迭代） | 只定"要干什么、要去哪里"，不碰底层实时微操；Skill 跑在 Linux 层，基于 MCU 硬件原子原语编排 |
| **执行层回路（rt-loop）** | Linux 侧 | 接收 Agent 高层意图，生成轨迹 / 指令流下发 MCU，回收本体状态做闭环校调 | 10-100Hz | 指令下发与状态回收的衔接层，不承载认知与稳控 |
| **稳控反射层（小脑 + 躯体反射）** | MCU 端控单元 | PID + 可选极小微模型（Pi0.5/0.6 类）；扰动补偿 / 姿态稳控 / 急停 | 毫秒-十毫秒级高速闭环 | 不看图像、不懂业务任务；微模型失效降级回传统 PID，接口不变，上层无感 |

- **职责边界**：底层反射不评判高层目标合理性；业务逻辑与目标校验留给 Pi-Agent 框架。瞬时扰动底层消化，策略层面变更等待下一轮 Agent 推理周期
- **推理抽象层**：采用 OpenAI 兼容协议抽象，隔离不同推理后端，支持多模型服务商接入与开放生态

## 目录职责 + 文件/路径含义

仓库当前状态：

```
taihao/
├── README.md          # 项目门面（人）
├── AGENTS.md          # 本文件（AI）
├── LICENSE            # Apache-2.0
├── .gitignore
├── assets/            # 效果图
├── 上位SBC内核裁剪完整方案.html # 工作机制四页图（总体架构 / 运行机制 / 安全与数据 / 接入位速查）
├── config/kernel/     # 内核裁剪基线片段（qemu-aarch64-{ci,e2e}.config）
├── docs/              # 设计文档（内核裁剪方案 v0.0.8 + kconfig 派生产物 + 任务/验收文档）
├── src/               # 服务层源码（pi-agent / rt-loop / hal-gateway / comm-center / extension-bridge + taihao-common + systemd + skills）
├── packaging/         # Buildroot 基座（buildroot/ 配置；output/ 不入库）
└── scripts/
    ├── kconfig/       # merge_config.sh（内核片段合并 / savedefconfig 验证）
    ├── kernel-build/  # 构建 / 仿真脚本（build-kernel.sh + qemu-start-{ci,e2e}.sh + lib）
    └── services/      # 服务层集成脚本（install.sh）
```

未来新增目录 / 文件时的职责边界：

- 太昊 OS 源码进 `src/`（Pi Agent runtime / HAL gateway / 接入位接口 / 调度通信中心）
- Buildroot 配置进 `packaging/buildroot/`（overlays / configs / package recipes）
- 构建产物 `packaging/output/` 不入库（已在 .gitignore 排除 `*.img` / `*.iso`）
- 运行时路径（仅作认知，未来实现时遵循）：
  - `/etc/taihao-os/` — 全局配置
  - `/usr/share/taihao-os/skills/` — 系统级 SKILL
  - `/etc/taihao-os/skills/` — 厂商级 SKILL
  - `/var/lib/taihao-os/` — 运行时持久化数据
  - `/var/log/taihao-os/` — 审计日志

## 修改 / 提交 / PR 规范

### 修改流程

1. 读 README.md 与本文件确认当前约束
2. 用 `git status` / `git diff` / `git log --oneline -5` 复核当前状态
3. **未确认前不动手**——尤其是重命名、删除、迁移、强制更新、清理未跟踪文件、跳过钩子、重写历史
4. 一次性大重写到终态

### 暂存粒度

- 单次 commit 只表达一个语义单元
- 暂存使用部分文件粒度，禁止 `git add .` / `git add -A`
- 不夹带 `node_modules/` / `dist/` / `.cache/` / `.env` / 凭证类文件

### Commit Message 格式

`<类型>(<范围>): <主题>`

- 类型：`feat` / `fix` / `refactor` / `docs` / `chore` / `test` / `build` / `ci`
- 范围：模块或文件名（如 `readme` / `agents` / `core` / `pi-agent` / `hal` / `skill` …）
- 主题：一行，中文优先，不超过 50 字

### 远端操作

- 不主动 `push`、不主动 `commit`（除非用户明示）
- 不创建 PR、不合并、不打 tag、不发布、不修改远程设置
- 不强推、不硬重置、不清理未跟踪文件、不跳过钩子、不修改 Git 配置

### 重写历史

- 已推送的 commit 历史不重写
- 未推送且用户明示"重写历史"时方可（如本仓库初始化阶段）

## 模拟器 / 仿真 / 真机工具栈

| 工具 | 角色 | 必要性 | 安装 |
| --- | --- | --- | --- |
| **QEMU** | 测试 / 体验 OS 镜像（aarch64 + x86_64，可编程集成、可调试） | 必装 | `brew install qemu` / `apt install qemu-system-arm qemu-system-x86` |
| **UTM** | Linux VM host（macOS 上跑 Buildroot 构建） | 必装 | `brew install --cask utm` |
| **Lima** | headless Linux VM 替代 UTM（构建用） | 采用 | `brew install lima` |
| **Apple Silicon native** | ARM64 Linux 直接跑 | 极简开发 | 不需装；Buildroot 仍是 Linux-only |

## 构建产物 · 平台覆盖

构建产物 = **可启动 img**(每个平台一份),所有平台共享一份内核(arm64)+ rootfs(ext4),bootloader 各自拼。

| 平台 | 启动链 | img 文件 | 用途 | 状态 |
|---|---|---|---|---|
| **QEMU virt** | QEMU 直启 SD 卡 img | `out/taihao-qemu.img` | 仿真验证 / CI | M1 ✅ |
| **RPi 4B** | VideoCore → kernel8 + DTB | `out/taihao-rpi4b.img` | 路演功能介绍 | M2 ✅ |
| **RPi 3B+** | VideoCore(`arm_64bit=1`)→ kernel8 + DTB | `out/taihao-rpi3bp.img` | 路演功能介绍 | M3 ✅ |
| **RK3588** | maskrom → idbloader → U-Boot → Image + DTB | `out/taihao-rk3588.img` | 批量生产机器人 | M4 后置 |

**共享**:`Image`(arm64 一份)+ `initramfs.cpio` + `rootfs.ext4`
**裁剪策略**:**内核一致,驱动按平台裁剪**(BASE Image 通用,各平台通过 Kconfig 片段启用平台特有驱动,如 RPi 加 `CONFIG_BRCMFMAC` / `CONFIG_VIDEO_BCM2835` 等)
**验证与分发**:按平台维护(QEMU / RPi / RK3588 各自的 img 各自迭代,不互锁)

**当前重心**:`qemu + RPi(3B+/4B)`,RK3588 真机后置

工作流：Buildroot / 内核构建在 Lima 下的 Linux VM 跑；QEMU 启动构建产物测试 / 体验 OS；真机部署走 dd / OTA。技术路径必须走 QEMU 系虚拟化，不用 Docker / 容器化。

## 调试 / 排查 / 验证 SOP

### 当前阶段（基座 OS 落地批次验收通过 + 多平台 img 启动镜像完成 M1-M3）

仓库状态：内核裁剪契约 v0.0.8 + `scripts/kernel-build/` 构建 / 仿真链路交付（CI 冒烟 7s / 快照 load 0s / 失效自动重建 / 增量 4.7s）；服务层（src/ 五模块 + systemd unit）实测验收通过（六门槛：五服务 active / rt-loop 46.1Hz / hal 白名单拒绝 / ext 越权拒绝 / 快照 <2s / CI 回归）；Buildroot 基座打包 + 分区 / OTA 骨架按 `docs/kernel-buildroot-任务.md` 验收通过（rootfs.ext2 镜像 5 服务 Started + Multi-User System；partition / ota-build / ota-apply 三脚本实测可执行）；多平台 SD 卡 img（M1 QEMU SD 卡 + M2 RPi 4B + M3 RPi 3B+）已交付，每个 img 1.3GB（FAT32 boot 256M + ext4 root 1G），共享 arm64 内核 + buildroot rootfs，按平台拼 bootloader 与 DTB 出可启动 img；RK3588 启动镜像（M4）后置。SOP：

1. **仓库状态**：开工前 `git -C <项目> status && git -C <项目> log --oneline -5`
2. **Kconfig 产物核对**：`config/kernel/` 两份片段 ↔ `docs/linux-内核裁剪方案.md` v0.0.8 决策表 #1~#87 逐条对照；自我检查表见 `docs/kconfig-简短说明.md`
3. **构建 / 仿真脚本**：`scripts/kernel-build/` 按任务文档 `docs/kernel-build-任务.md` 交付并自检（savedefconfig 合并通过 + 构建产出 Image）
4. **文档一致性**：任何修改完成后 grep 核验
   - 不出现 `详见 / 参见 / 见 docs / 见设计文档 / 见 .poc` 等外指
   - Mermaid 图无 `style` / `classDef` / Emoji
   - 命名符合「命名 + 术语定义」
   - 文档中明说目标硬件（RK3588 同级别）、基座（Buildroot）、烧录（dd + OTA）、仿真（QEMU）
5. **镜像构建（QEMU 系构建路径，不用 Docker / 容器化）**：
   - 本机（macOS）：`brew install ccache shellcheck lima`
   - Lima 启动 ARM64 Linux VM：`limactl start --name <vm> --set='.arch="aarch64"'`（或直接 `limactl start --name <vm> <templ>`），进入 `limactl shell <vm>`
   - VM 内装构建依赖：`apt install aarch64-linux-gnu-gcc bc bison flex libelf-dev openssl'` + git / ccache / cpio
   - 在 VM 内执行构建：`./scripts/kernel-build/build-kernel.sh ci|e2e`（源码自动 clone + 固定 commit v6.6；ccache 复用缓存缩短增量）
   - 产物同步回宿主：虚拟机内构建出的 `out/` 与用户已安装 QEMU 的宿主共享 / 同步后，用 `scripts/kernel-build/qemu-start-{ci,e2e}.sh` 启动验证
   - 冒烟标准：镜像内 systemd `basic.target` 达成，退出码 0
   - 快照标准：`save-snap` / `load-snap` 循环成功，重启 < 2 s；镜像 / initramfs 更换 → 快照失效自动重建
6. **变更日志**：每次对正式工程的修改必须在本文件「变更日志」新增一行

## 一致性核验

任何对正式工程的修改完成后，核对：

1. 与 README.md 对应章节一致（品牌 / 定位 / 核心能力 / 设计思路 / 集成方式）
2. 与本文件「铁律」「命名 + 术语定义」「修改 / 提交 / PR 规范」一致
3. 不引用任何外部文件
4. Mermaid 图无 `style` / `classDef` / Emoji
5. 文档以中文为主
6. 变更日志已更新

## 变更日志

| 日期 | 变更 | 影响范围 |
| --- | --- | --- |
| 仓库初始化（终态） | 太昊定位为「智能终端 / 机器人设备端 OS」；README + AGENTS 一次性大重写到终态（移除「起点 / 终点」废话与「太昊 = 构建系统」误读）；重写 git 历史为单 commit | 仓库根 |
| 2026-08-01 | Pi Agent M2 落地（Rust）：Agent Loop / SKILL Registry / Provider（OpenAI 兼容远端端点）；BR2_EXTERNAL 接入自研 cargo 包 + systemd unit 开机自启 + SKILL overlay；QEMU 验证通过；SOP 当前阶段更新为「M2」 | src/pi-agent、packaging/buildroot |
| 2026-08-03 | M3 全模块落地：① hal-gateway（白名单+审计+mock 驱动）② rt-loop（10~100Hz 实时回路）③ SKILL 集补齐 6 个（紧急避险/故障自愈/多机避障/分区协同/路径规划/感知融合）④ comm-center（SSE/WS/MQTT + Mesh UDP 双链路 + 故障切换 + 路由）⑤ extension-bridge（JSON-RPC + 白名单 + 权限边界）⑥ pi-agent 补 SKILL 签名校验/Hooks/Extension 加载 ⑦ Buildroot 5 包 + systemd 四层隔离栈沙箱 + 固件三分区/A-B OTA 脚本骨架 + verify.sh smoke；SOP 更新为「M3」 | src/*、packaging/buildroot、examples/skills、scripts |
| 2026-08-07 | Buildroot 自研包重构启动：原 5 包（pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge）及其 apparmor 模板移除，Config.in / taihao_defconfig 引用同步清理；SOP 当前阶段更新为「M3（构建集成重构中）」 | packaging/buildroot |
| 2026-08-07 | 源码层整体重置：src/ 五模块、examples/、packaging/（Buildroot + 分区脚本）、scripts/verify.sh 全部移除；AGENTS.md 目录树 / SOP 更新为「构建集成批次」（仅按 linux-内核裁剪方案 v0.0.7 完成镜像内核裁剪与封装）；新建 docs/kernel-build-任务.md 下发构建 / 仿真脚本任务 + scripts/kernel-build/ 目录骨架 | 仓库根 |
| 2026-08-07 | 新增「终端架构认知（两层模型）」：认知决策层（RK3588 + Pi-Agent + 4-7B）/ 执行层回路（rt-loop 10-100Hz）/ 稳控反射层（MCU）；同步撰写记录到 内核工作原理.html §十一 | AGENTS.md、内核工作原理.html |
| 2026-08-07 | 内核工作原理.html 修订：§二 端到端服务清单对齐契约（5 服务：pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge）、§十一 两层模型排版重构（三层卡片 + 职责边界 + 推理抽象层）；新建 docs/kernel-build-e2e-任务.md（端到端镜像构建与运行验证任务，e2e 验证批次） | 内核工作原理.html、docs/kernel-build-e2e-任务.md |
| 2026-08-07 | 技术路径定案：QEMU 系构建路径（Lima ARM64 Linux VM 内构建 + 宿主 QEMU 启动验证），明确不用 Docker / 容器化；工具栈 Lima 升为「采用」；SOP「镜像构建」步骤细化（VM 依赖安装 + build-kernel.sh + 产物同步 + 冒烟/快照标准） | AGENTS.md |
| 2026-08-07 | 达标实测 + 口径修订：scripts/kernel-build 全链路实测通过（CI 冒烟 7s / 快照 load 0s / 失效自动重建 / 增量 4.7s / rootfs busybox+systemd pid1）；启用未提交二次返修（ccache 4.x 兼容、snapshot hash 去 state、probe ANSI、qemu -display none 等）为交付态；契约 §2.3 冷构建口径放宽为「4~8 核区间 3~5 min + 硬上限 10 min」（Lima 4 核实测 263 s） | docs/linux-内核裁剪方案.md、docs/kernel-build-任务.md、docs/kernel-build-e2e-任务.md |
| 2026-08-07 | 服务层落地批次启动：新建 docs/kernel-services-任务.md（按设计规格实现 src/ 五模块 + systemd unit + 端到端五位 active 验收）替代原回填口径任务；SOP 当前阶段更新为「服务层落地批次」 | docs/kernel-services-任务.md、AGENTS.md |
| 2026-08-07 | 术语定案同步：①「中台调度通信模块」改名「调度通信中心」（英文 comm-center 不变，README / AGENTS / 任务文档同步）② Pi Agent 明确 AGENT 内核采用 `badlogic/pi-mono`（仅技术内核选型参考，不构成品牌词；OpenClaw 同上）③ SKILL.md 明确兼容 Anthropic Agent Skills 规范 | AGENTS.md、README.md、docs/kernel-services-任务.md、docs/kernel-build-e2e-任务.md |
| 2026-08-07 | 服务层实测复核验收（按任务文档 §六/§九口径）：五服务 active / rt-loop 46.1Hz 实测 / hal 白名单拒绝 / ext 越权拒绝 / 快照 load 0s / CI 冒烟 7s(RC=0) 六条硬门槛全通过；taihao-check.sh 以 perl 探测 Unix socket + 8082 健康端点取代 curl;kernel-services-验收报告重写为实测终态（替代早期未实跑版本）；SOP 当前阶段保持不变（服务层落地批次验收通过） | docs/kernel-services-验收报告.md、src/systemd/taihao-check.sh、AGENTS.md |
| 2026-08-07 | 基座 OS 落地批次启动：新建 docs/kernel-buildroot-任务.md（Buildroot 基座打包 + 分区 / A-B OTA 骨架，对齐内核契约 §2.1/§2.3/§8 + AGENTS 技术选型）；AGENTS 目录树补 packaging/ / src/ / scripts/services/、修正 HTML 文件名、SOP 当前阶段更新为「基座 OS 落地批次」 | docs/kernel-buildroot-任务.md、AGENTS.md |
| 2026-08-08 | 基座 OS 落地批次验收通过（按任务文档 §四 五硬性项）：Buildroot 镜像 rootfs.ext2（1GB ext4）QEMU 启动 → 5 服务 Started + Multi-User System；5 binary/unit/wants/SKILL/taihao 用户/default.target 全量核验；partition-layout / ota-build / ota-apply 三脚本实测可执行（4 分区 GPT + ota.json sha256 + 写槽→校验→切换→回滚契约）；kernel-buildroot-验收报告定稿；SOP 当前阶段更新为「基座 OS 落地批次验收通过」 | docs/kernel-buildroot-验收报告.md、packaging/buildroot、scripts/partition、AGENTS.md |
| 2026-08-14 | 黑盒验收口径收尾：post-build.sh 收尾三件事 ① 清掉 mkusers 上轮加的同名 dbus/systemd-* 用户,留给 fakeroot mkusers 重新加(避免 mkusers 跨次 build 冲突)② dbus.service.d/10-root.conf drop-in 强制 dbus 跑 root + machine-id 兜底生成 ③ systemd-remount-fs.service mask 掉(initramfs 无 /dev/root 必然 FAILED);登录门面改 TAIHAO:/etc/issue + /etc/hostname + /etc/os-release PRETTY_NAME + 自定义 taihao-login 替换 agetty(serial-getty drop-in)→ 黑盒 boot 零 [FAILED] + `Welcome to TAIHAO` + `username:` 提示;新增 scripts/run/ 三个 QEMU 启动脚本(qemu冒烟启动.sh / qemu运行镜像.sh / qemu调试镜像.sh)统一封装 Image+initramfs.cpio+snap 路径;AGENTS.md 新增「构建产物 · 平台覆盖」章节登记多平台 img(QEMU + RPi 3B+/4B + RK3588)需求 | packaging/buildroot/scripts/post-build.sh、scripts/run/、AGENTS.md |
| 2026-08-14 | 多平台 img M1 + M2 + M3 交付:QEMU SD 卡 img(`taihao-qemu.img`,FAT32 boot + ext4 root,genimage 拼);RPi 4B img(`taihao-rpi4b.img`,VideoCore 固件 + kernel8.img + bcm2711-rpi-4-b.dtb + config.txt arm_64bit=1);RPi 3B+ img(`taihao-rpi3bp.img`,同上 DTB 换 bcm2710-rpi-3-b-plus.dtb);共享 arm64 内核 + buildroot rootfs,bootloader 平台各自拼;post-image.sh 改写为支持多平台(PLATFORM=qemu\|rpi4b\|rpi3bp via BR2_ROOTFS_POST_IMAGE_SCRIPT_ARGS);新增 fetch-rpi-firmware.sh 运行时从 github tarball 拉 VideoCore 固件(缓存到 ~/taihao-out/rpi-firmware/);新增 scripts/run/img烧到SD卡.sh(macOS dd 烧录 + 二次确认);AGENTS.md 更新 M1-M3 状态 ✅ | packaging/buildroot/scripts/{post-image.sh,fetch-rpi-firmware.sh,genimage-{qemu,rpi4b,rpi3bp}.cfg}、scripts/run/img烧到SD卡.sh、AGENTS.md |