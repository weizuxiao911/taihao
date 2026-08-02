---
name: path-planning
version: 0.1.0
security_level: L3
description: |
  路径规划 SKILL：依据任务参数与实时环境生成作业路径
  （覆盖路径 / 返航路径 / 避障修正），输出目标航点序列。
author: taihao-team
---

# 路径规划 (Path Planning)

## 触发条件

| 任务类型 | 输入 | 输出 |
|---|---|---|
| 覆盖作业 | 任务区边界 + 分辨率 | 扫描覆盖路径 |
| 返航 | 当前位置 + 归位点 | 安全返航路径 |
| 避障修正 | 障碍物集合 | 绕行路径 |

## 决策流程

```
任务参数 → 环境栅格（感知融合输入）→ 规划算法
        ↓ 覆盖: 弓字形扫描（分区协同子区域）
        ↓ 返航: 最短路 + 障碍规避
        ↓ 修正: 局部绕行
        → 输出航点序列 (target=waypoints[])
```

## 输出

- mode: `survey` / `return`
- target: 航点序列（JSON 数组）

## 依赖

- HAL：定位 / 里程计
- SKILL：感知融合（环境栅格）
- SKILL：多机避障（邻域避让约束）

## 安全准则

- 走 Pi Agent SKILL 调用（不绕过 HAL 网关）
- 规划结果仅作目标参考，10~100Hz 控制回路由 rt-loop 承担
