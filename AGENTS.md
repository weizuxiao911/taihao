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

## 目录职责 + 文件/路径含义

仓库当前状态：

```
taihao/
├── README.md          # 项目门面（人）
├── AGENTS.md          # 本文件（AI）
├── LICENSE            # Apache-2.0
└── .gitignore
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

## 调试 / 排查 / 验证 SOP

### 当前阶段（早期骨架，零代码）

仓库当前无源码、无测试、无 CI、无构建产物。SOP 主要是状态核验：

1. **仓库状态**：开工前 `git -C <项目> status && git -C <项目> log --oneline -5`
2. **文档一致性**：任何修改完成后 grep 核验
   - 不出现 `详见 / 参见 / 见 docs / 见设计文档 / 见 .poc` 等外指
   - Mermaid 图无 `style` / `classDef` / Emoji
   - 命名符合「命名 + 术语定义」
   - 文档中明说目标硬件（RK3588 同级别）、基座（Buildroot）、烧录（dd + OTA）、仿真（QEMU）
3. **变更日志**：每次对正式工程的修改必须在本文件「变更日志」新增一行

### 未来阶段（实现期）

源码落地后，按模块拆 SOP：

- **Linux Core**：内核裁剪配置（6.6 LTS + PREEMPT_RT）、`/etc/taihao-os/` 编排、固件三分区镜像构建；验证 = systemd unit Ready + HAL 网关白名单测试
- **Pi Agent**：Skill Registry / Extension 加载顺序；验证 = SKILL 签名校验 + Hooks 触发日志
- **接入位四槽**：每槽独立验证
  - HAL：硬件 mock 驱动 + 网关审计日志
  - SKILL：SKILL.md frontmatter 校验 + 安全分级断言
  - Provider：OpenAI 兼容端点连通性 + 中台调度通信双链路切换
  - Extension·MCP：桥接调用往返 + 权限边界
- **核心安全准则**：强制回归——LLM 不下场实时控制 + 不绕过 HAL 网关驱动硬件
- **QEMU 仿真验证（强制）**：CI 中 QEMU 启动构建产物 → systemd Ready + Pi Agent 起来 + 接入位加载 → 全链路验证；amd64 + aarch64 均要跑通

具体命令与验证脚本在新增模块时落地到本节。

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