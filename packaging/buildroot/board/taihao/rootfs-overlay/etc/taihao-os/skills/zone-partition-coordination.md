---
name: zone-partition-coordination
version: 0.1.0
security_level: L2
description: |
  分区协同 SKILL：依据邻域拓扑与作业能力动态划分作业子区域，
  多台设备并行覆盖、互不重叠地完成任务。
author: taihao-team
---

# 分区协同 (Zone Partition Coordination)

## 触发条件

| 场景 | 判定 | 动作 |
|---|---|---|
| 集群作业 | 多台设备同任务区 | 参与分区协商 |
| 任务重分配 | 成员离线 / 能力变化 | 触发重新分区 |
| 边界冲突 | 邻域覆盖重叠 | 按协商结果让渡子区域 |

## 决策流程

```
任务区地图 + 邻居能力表 → 分区协商（Mesh 广播）
        ↓ 取得子区域: 更新作业边界 (target=zone_id + bounds)
        ↓ 边界冲突: 让渡 / 调整覆盖
        ↓ 分区完成: 并行执行（互不重叠）
```

## 输出

- mode: `survey`（作业中）
- target: `zone:{id}` / 边界坐标

## 依赖

- HAL：定位 / 里程计
- Provider：本地 Mesh 链路（分区协商消息）
- SKILL：路径规划（子区域覆盖路径）

## 安全准则

- 走 Pi Agent SKILL 调用（不绕过 HAL 网关）
- 分区决策只影响本机作业边界，不越权控制邻域设备
