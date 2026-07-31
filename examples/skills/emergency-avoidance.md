---
name: emergency-avoidance
version: 0.1.0
security_level: L1
description: |
  紧急避险 SKILL：感知碰撞 / 缠绕 / 低电 / 严重传感器异常时
  立即触发保护动作（停机 / 悬停 / 脱离作业区 / 上浮）。
author: taihao-team
---

# 紧急避险 (Emergency Avoidance)

## 触发条件

| 传感器 | 阈值 | 动作 |
|---|---|---|
| 距离传感器 | < 0.5m | 立即减速 + 转向 |
| 电流互感器 | > 额定 × 1.5 | 紧急停机 |
| 电量计 | < 15% | 上浮 + 返航 |
| IMU 倾角 | > 45° | 姿态修正 + 减速 |
| 漏水传感器 | 检测到水 | 紧急上浮 + 报警 |

## 决策流程

```
感知异常 → 规则引擎快速通道(<5ms) → 触发保护动作
        ↓
        Report(TaskReport{phase: ABORT, reason: "emergency"})
        ↓
        上报岸基 / 中台调度通信模块
```

## 依赖

- HAL：传感器集（距离 / 电流 / 电量 / IMU / 漏水）
- HAL：执行器（推进器 / 云台 / 浮力调节）
- Provider：MQTT（紧急上报）

## 安全准则

- 走 Pi Agent SKILL 调用（不绕过 HAL 网关）
- HAL 网关白名单内执行（白名单外的执行器调用一律拒绝）
- 所有动作 + 决策日志落审计 (`/var/log/taihao-os/audit.log`)
