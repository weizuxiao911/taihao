# src/ — 太昊 OS 源码

太昊 OS 运行时源码目录。

## 布局

```
src/
├── pi-agent/          # Pi Agent runtime（Agent Loop / Skill Registry / Hooks / Provider）✓ M2 已实现
├── hal-gateway/       # HAL 网关（白名单 + 审计 + capability 隔离）— 待实现
├── comm/              # 中台调度通信模块（MQTT / Mesh 适配）— 待实现
└── extension/         # Extension·MCP 桥 — 待实现
```

## 选型（已定稿）

| 项 | 选择 | 状态 |
| --- | --- | --- |
| Pi Agent runtime 语言 | **Rust** | M2 已落地 |
| HAL gateway 语言 | **Rust**（与 Pi Agent 一致） | 待实现 |
| LLM 接入 | 远端 OpenAI 兼容端点（Provider 接口 + REST 客户端） | M2 已落地 |
| MQTT 客户端库 | Mosquitto C client / Paho（选型时再定） | 待实现 |
| SKILL frontmatter schema | name / version / security_level / description / author（见 `examples/skills/`） | 已定稿 |

## Pi Agent（M2 已实现）

- 位置：`src/pi-agent/`
- 组成：Agent Loop（≤1Hz 决策）、SKILL Registry（frontmatter 校验 + 防注入）、Hooks 预留、Provider（OpenAI 兼容 REST）
- 配置：`pi-agent.toml.example` → 部署为 `/etc/taihao-os/pi-agent.toml`；API key 走环境变量（`OPENAI_API_KEY`），不写入配置
- 决策输出：`/var/lib/taihao-os/pi-agent/decision.json`（JSON，mode 白名单校验）
- 验证：`cargo test`（14 用例）；端到端 mock LLM 验证通过（SKILL 加载 → 决策 → 输出）
- 安全：LLM 输出仅 mode/target，不直接控硬件；SKILL frontmatter 白名单字段校验

## 代码进入遵循

- 单一职责
- 跨模块只通过约定接口
- 凭证 / 运行时数据不入库
- 中文优先（注释 / 文档）
