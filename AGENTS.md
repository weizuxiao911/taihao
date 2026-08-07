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
| **Pi Agent** | 唯一智能决策层；Agent Loop / Skill Registry / Extension / Hooks |
| **开放接入位** | 垂直差异承载位；四槽见下 |

### 开放接入位四槽

| 槽 | 承载内容 |
| --- | --- |
| **HAL 集** | 硬件驱动（spidev / i2c-dev / V4L2 / SocketCAN / pwmchip / serial / 水声通信机 / …） |
| **SKILL 集** | 策略 / 任务 / 自治能力 SKILL.md 包 |
| **Provider 集** | 模型 / 工具 / 中台调度通信模块的 Provider；任意 OpenAI 兼容端点 |
| **Extension·MCP 桥** | 扩展与 MCP 桥接，接入自研边缘能力 |

### 中台调度通信模块

- 位置：Pi Agent 与外部系统之间，作为接入位实现
- 职责：调度（任务编排、消息路由、双链路选择、故障切换）+ 通信（协议适配）
- 对接对象：厂商配置决定，OS 出厂不预设

### 不使用 / 不绑定的词

- 不把"对接岸基控制系统"作为品牌特征写入文档——这只是中台调度通信模块的一个 Provider 实例
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
├── 工作原理.html      # 工作机制四页图（总体架构 / 运行机制 / 安全与数据 / 接入位速查）
├── config/kernel/     # 内核裁剪基线片段（qemu-aarch64-{ci,e2e}.config）
├── docs/              # 设计文档（内核裁剪方案 v0.0.7 + kconfig 派生产物 + 任务文档）
└── scripts/
    ├── kconfig/       # merge_config.sh（内核片段合并 / savedefconfig 验证）
    └── kernel-build/  # 构建 / 仿真脚本（build-kernel.sh + qemu-start-{ci,e2e}.sh + lib）
```

未来新增目录 / 文件时的职责边界：

- 太昊 OS 源码进 `src/`（Pi Agent runtime / HAL gateway / 接入位接口 / 中台调度通信模块）
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

## 模拟器 / 仿真工具栈

| 工具 | 角色 | 必要性 | 安装 |
| --- | --- | --- | --- |
| **QEMU** | 测试 / 体验 OS 镜像（aarch64 + x86_64，可编程集成、可调试） | 必装 | `brew install qemu` / `apt install qemu-system-arm qemu-system-x86` |
| **UTM** | Linux VM host（macOS 上跑 Buildroot 构建） | 必装 | `brew install --cask utm` |
| **Lima** | headless Linux VM 替代 UTM（构建用） | 采用 | `brew install lima` |
| **Apple Silicon native** | ARM64 Linux 直接跑 | 极简开发 | 不需装；Buildroot 仍是 Linux-only |

工作流：Buildroot / 内核构建在 Lima 下的 Linux VM 跑；QEMU 启动构建产物测试 / 体验 OS；真机部署走 dd / OTA。技术路径必须走 QEMU 系虚拟化，不用 Docker / 容器化。

## 调试 / 排查 / 验证 SOP

### 当前阶段（服务层回填批次）

仓库状态：内核裁剪契约 v0.0.8（冷构建放宽为 4~8 核区间 3~5 min + 硬上限 10 min）+ `scripts/kernel-build/` 构建 / 仿真脚本全链路实测通过（CI 冒烟 7s / 快照 load 0s / 失效自动重建 / 增量 4.7s）+ 验收报告交付；服务层（src/ 五模块 + systemd unit）回填中。SOP：

1. **仓库状态**：开工前 `git -C <项目> status && git -C <项目> log --oneline -5`
2. **Kconfig 产物核对**：`config/kernel/` 两份片段 ↔ `docs/linux-内核裁剪方案.md` v0.0.7 决策表 #1~#87 逐条对照；自我检查表见 `docs/kconfig-简短说明.md`
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

### 内核对齐（规划）

- **镜像内核裁剪与封装**：按 `docs/linux-内核裁剪方案.md` v0.0.7 完成内核裁剪 + 构建 / 仿真封装（`scripts/kernel-build/`），产出可启动镜像
- **仿真验证**：QEMU 启动 → 镜像内 systemd Ready；amd64 + aarch64 均要跑通

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
| 2026-08-07 | 服务层回填批次启动：新建 docs/kernel-services-回填-任务.md（重建 src/ 五模块 + systemd unit + rootfs 回填 + 五位 is-active 全绿验收）；SOP 当前阶段更新为「服务层回填批次」 | docs/kernel-services-回填-任务.md、AGENTS.md |