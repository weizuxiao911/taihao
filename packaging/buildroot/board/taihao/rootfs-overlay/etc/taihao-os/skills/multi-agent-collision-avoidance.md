---
name: multi-agent-collision-avoidance
version: 0.1.0
security_level: L2
description: |
  多机避障 SKILL：依托本地 Mesh 感知邻域设备位置与运动意图，
  路径规划中自动避让，防止同区域设备碰撞。
author: taihao-team
---

# 多机避障 (Multi-Agent Collision Avoidance)

## 触发条件

| 邻域信号 | 判定 | 避让动作 |
|---|---|---|
| 邻域设备距离 | < 安全间距 | 减速 + 侧向偏移 |
| 邻域运动意图 | 相向而行 | 偏航避让（优先右侧规则） |
| 邻域集群密度 | 高密度区域 | 降速穿越 + 加大间距 |

## 决策流程

```
Mesh 邻居表（位置 + 意图） → 冲突预测（时间窗）
        ↓ 无冲突: 维持当前模式
        ↓ 预测冲突: 计算避让向量 → 更新目标 (target=waypoint_offset)
        ↓ 立即危险: 减速 + 偏航 (mode=emergency_avoidance)
```

## 输出

- mode: `survey`（带避让目标）或 `emergency_avoidance`（立即危险）
- target: 避让航点 / 偏移量

## 依赖

- HAL：传感器集（距离 / 定位）
- Provider：本地 Mesh 链路（邻居表广播）
- SKILL：路径规划（避让航点接入）

## 安全准则

- 走 Pi Agent SKILL 调用（不绕过 HAL 网关）
- 避让动作受 HAL 网关白名单约束（推进器 / 转向指令）
