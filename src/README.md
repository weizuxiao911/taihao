# src/ — 太昊 OS 源码

本目录将容纳太昊 OS 的运行时源码。当前为 skeleton 占位。

## 计划布局

```
src/
├── pi-agent/          # Pi Agent runtime（Agent Loop / Skill Registry / Extension / Hooks）
├── hal-gateway/       # HAL 网关（白名单 + 审计 + capability 隔离）
├── comm/              # 中台调度通信模块（MQTT / Mesh 适配）
└── extension/         # Extension·MCP 桥
```

## 选型待定

后续会话需对齐：

- **Pi Agent runtime 语言**：Rust / C / C++ / Go
- **HAL gateway 语言**：同上
- **MQTT 客户端库**：Mosquitto C client / Paho
- **SKILL frontmatter schema**：当前示例（`examples/skills/emergency-avoidance.md`）占位，需定稿

## M1 阶段

本目录在 M1 阶段保持空目录；M2 起开始填代码。

代码进入时遵循：

- 单一职责
- 跨模块只通过约定接口
- 凭证 / 运行时数据不入库
- 中文优先（注释 / 文档）
