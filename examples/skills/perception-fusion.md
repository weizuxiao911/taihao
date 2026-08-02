---
name: perception-fusion
version: 0.1.0
security_level: L3
description: |
  感知融合 SKILL：融合多源传感器（视觉 / 声呐 / IMU / 距离 / 电流 / 电量）
  为统一环境状态，供路径规划与自治决策使用。
author: taihao-team
---

# 感知融合 (Perception Fusion)

## 触发条件

| 传感器 | 数据 | 融合产物 |
|---|---|---|
| 视觉 (V4L2) | 图像帧 | 目标检测 / 障碍轮廓 |
| 声呐 | 距离点云 | 环境栅格 |
| IMU | 姿态 / 加速度 | 运动状态估计 |
| 电流 / 电量 | 功耗 | 续航预测 |

## 决策流程

```
多源采样（HAL 网关） → 时间对齐 → 融合（加权 / 置信度）
        → 统一状态输出: state.json（栅格 + 位姿 + 健康度）
```

## 输出

- 状态文件：`/var/lib/taihao-os/pi-agent/state.json`（供 Agent Loop 读取）
- mode: 不直接输出控制模式（辅助决策输入）

## 依赖

- HAL：传感器集（视觉 / 声呐 / IMU / 距离 / 电流 / 电量）
- SKILL：路径规划（栅格输入）

## 安全准则

- 传感器读取走 HAL 网关白名单（只读工具集）
- 融合结果标记置信度，低置信度不参与紧急决策
