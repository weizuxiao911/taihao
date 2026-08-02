---
name: fault-recovery
version: 0.1.0
security_level: L2
description: |
  故障自愈 SKILL：监测异常时自动尝试本地恢复或降级运行
  （重连链路 / 切换通信通道 / 临时关闭故障模块）；
  恢复失败则上报进入维护流程。
author: taihao-team
---

# 故障自愈 (Fault Recovery)

## 触发条件

| 异常源 | 判定 | 恢复动作 |
|---|---|---|
| 远程链路断连 | 心跳超时 > 阈值 | 切换备用链路（远程 ↔ Mesh） |
| 通信通道劣化 | 丢包率 > 20% | 切换通道（4G/5G ↔ Wi-Fi ↔ 水声） |
| 模块异常 | 自检失败 N 次 | 临时关闭故障模块 + 降级运行 |
| 进程失联 | 保活探测失败 | 重启进程（系统服务） |

## 决策流程

```
监测异常 → 分级（可自愈 / 需降级 / 需维护）
        ↓ 可自愈: 本地恢复动作 (重连 / 切换 / 重启)
        ↓ 降级: 关闭故障模块 + 调整模式 (target=degraded)
        ↓ 失败: 上报维护流程 (Report(Maintenance{reason}))
```

## 输出

- mode: `return` / `survey`（降级继续）或 `idle`（进入维护等待）
- target: `degraded` / `maintenance`

## 依赖

- HAL：传感器集（自检探针）
- Provider：中台调度通信模块（重连 / 切换通道）
- Extension：进程保活钩子

## 安全准则

- 走 Pi Agent SKILL 调用（不绕过 HAL 网关）
- 降级决策受安全分级约束（L2 不得触碰 L1 紧急保护动作）
