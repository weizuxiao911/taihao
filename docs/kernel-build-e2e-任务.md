# 任务:端到端镜像构建与运行验证(e2e 验证批次)

> 执行对象:AI / 人工。本任务在「构建集成批次」交付的 `scripts/kernel-build/` 之上,把**端到端内核**从「可构建」推进到「可验证」:产出可启动的 e2e 镜像,打通 QEMU 启动、9p 共享、快照 save / load 闭环,并按契约完成全套服务集成验收与迭代指标实测。

## 一、依据(唯一契约,不引用仓库外文件)

- `<仓库根>/docs/linux-内核裁剪方案.md`(v0.0.7 定稿)
- 重点章节:

| 章节 | 内容 | 本任务落点 |
| --- | --- | --- |
| §2.1 端到端内核 | 完整集成验证;跑 pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge 全套服务;通过标准 = 全部服务 systemd unit `active` | 服务集成验收(§4.4) |
| §2.3 迭代速度硬指标 | 冷构建 < 3 min / 增量 < 30 s / qemu 重启 < 2 s | 指标实测(§4.5) |
| §4.3 savevm 快照 | 内核 / initramfs 替换后必须重新生成快照 | 快照失效检测(§4.3) |
| §9 步骤 6 | 端到端集成验证:跑全套服务 | 服务集成验收(§4.4) |

- 已交付基线:`config/kernel/qemu-aarch64-e2e.config` + `scripts/kconfig/merge_config.sh` + `scripts/kernel-build/`(build-kernel.sh / qemu-start-ci.sh / qemu-start-e2e.sh / lib/*)

## 2. 前置条件

| 项 | 要求 | 说明 |
| --- | --- | --- |
| 内核源码 | Linux 6.6,由 build-kernel.sh 自动拉取并锁定 tag | 无需手工准备 |
| 交叉工具链 | `aarch64-linux-gnu-gcc` 等 | `build-kernel.sh --cross-compile` 校验 |
| rootfs 源目录 | buildroot 输出 `/opt/buildroot/output/target` 或同等 rootfs,busybox + systemd | initramfs 打包源 |
| QEMU | `qemu-system-aarch64` + `qemu-img` | 启动 / 快照依赖 |
| ccache | 已安装 | 冷 / 增量指标依赖缓存命中 |
| 服务层 | pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge 已随 rootfs 部署,systemd unit 自启 | 与 §2.1 契约一致 |

> 服务层说明:5 项服务为 §2.1 端到端通过标准的内容载体,由 `src/` 层(仓库开放接入位实现)构建进 rootfs。若某批次服务层未就绪,本任务先落地工具链闭环(启动 + 快照 + 指标),服务断言位保留,待服务层补齐后回填。

## 3. 交付规格(沿用已有脚本,不重造)

| 脚本 | 能力 | 验收 |
| --- | --- | --- |
| `build-kernel.sh e2e` | 合入 `qemu-aarch64-e2e.config`;产 Image / initramfs / 状态盘 `snap/boot-snap.qcow2`;计时写 `build.time` | Image 为 ELF aarch64;initramfs 含 busybox + systemd |
| `qemu-start-e2e.sh` | 9p 共享 → guest `/mnt/host`;HMP unix socket;save-snap / load-snap / reset-snap 子命令;basic.target 超时探测 | 启动 → basic.target 达成;退出码 0/2/3 语义正确 |
| `lib/snapshot.sh` | hash 失效判定(image + initramfs + 状态盘);savevm / loadvm 封装 | 更换镜像后旧快照判定失效 |
| `lib/probe.sh` | systemd basic.target 存活探测 | 对 QEMU 串口输出正确命中 |

## 4. 验收(硬性)

### 4.1 构建链路

- `build-kernel.sh e2e --initramfs-dir <rootfs>` 退出码 0,`out/arm64-e2e/Image` 存在且 `file` 显示 ARM aarch64
- 状态盘 `out/arm64-e2e/snap/boot-snap.qcow2` 已创建(qemu-img info 可读)

### 4.2 启动与基本存活

- 执行 `qemu-start-e2e.sh`:QEMU 后台常驻,串口日志 `logs/serial.log` 存在
- `probe_basic_target` 命中 systemd basic.target,退出码 0
- guest 内 `mount -t 9p hostshare /mnt/host -o trans=virtio` 后宿主共享目录可见(热替换可用)

### 4.3 快照循环

- save-snap → load-snap 三段式:
  - save → hash 写入 `snap/boot-snap.hashes`,`savevm` 落盘
  - load → `loadvm` 恢复,从 load 到 basic.target 就绪时间戳差 < 2 s(记录实测值)
- **失效检测**:修改 initramfs(触发 hash 变化)后,`qemu-start-e2e.sh` 判定失效并丢弃旧 hash、重新冷启
- 快照生命周期:`reset-snap` 清理 hash,状态盘独立管理

### 4.4 服务集成验收(契约 §2.1 / §9 步骤 6)

guest 内 `systemctl is-active` 五个服务全部返回 `active`:

| 服务 | 说明 |
| --- | --- |
| pi-agent | 智能决策层(Agent Loop / SKILL) |
| hal-gateway | 硬件访问网关(HAL 白名单 + 审计) |
| rt-loop | 执行层回路(10-100 Hz 指令流下发 / 状态回收) |
| comm-center | 中台调度通信(多链路 + 故障切换 + 路由) |
| extension-bridge | 扩展 / MCP 桥(JSON-RPC + 白名单) |

> e2e kernel 保留了接入位全部子系统;任一服务 unit 非 active 或启动失败 = 本任务失败。

### 4.5 指标实测(契约 §2.3)

| 指标 | 目标 | 记录位置 |
| --- | --- | --- |
| 冷构建 | < 3 min | `build.time`(cold) |
| 增量构建 | < 30 s | `build.time`(warm) |
| qemu 快照重启 | < 2 s | serial 时间戳 / 手动测 |
| 冒烟执行(CI 侧) | 10 s 内 | CI 已交付,不重复 |

## 5. 自检(交付前全部通过)

- `bash -n` 全部脚本 0 错误
- 快照 save / load 双向循环 ≥ 2 次可用
- 镜像 / initramfs 任一更换 → 旧快照失效并自动重建
- 五个服务 `systemctl is-active` 全部 `active`
- 本次 e2e 验证实测日志留存(构建 / 启动 / 快照)

## 6. 交付清单

- 构建成功日志 + `build.time`(cold / warm 两值)
- 启动串口日志(basic.target 达成)
- 快照 save / load 实测记录 + < 2 s 计时
- 服务 `is-active` 结果列表

## 7. 里程碑与顺序

1. `build-kernel.sh e2e` → Image + initramfs + 状态盘
2. `qemu-start-e2e.sh` 冷启 → basic.target 达成
3. 9p 热替换验证
4. save-snap → 重启 → load-snap(计时)
5. 快照失效检测
6. 服务契约 `is-active` 全绿
7. 指标实测收尾

## 8. 预期

- 理想路径(单次打通):0.5 ~ 1 h
- 常规路径(rootfs / 快照往返):1.5 ~ 3 h
- 完成标志:工具链五步全通 + 服务契约五位全绿 + 指标实测日志留存