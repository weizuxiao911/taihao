# 任务:Kconfig 基线片段产出(aarch64 批次)

> 执行对象:AI / 人工。本任务产出太昊 OS 内核裁剪契约的 aarch64 基线片段,供 `scripts/kconfig/merge_config.sh` 直接加载,接入构建流水线。

## 依据(唯一契约,不引用仓库外文件)

- `<仓库根>/docs/linux-内核裁剪方案.md`(v0.0.7 定稿)
- 重点章节:§1 双架构等价替换、§2.1 双内核、§2.2 四条红线、§2.4 文件树、§5 决策表(87 项)、§6 关键 CONFIG

## 产出物(2 份,置于 `config/kernel/`)

| 文件 | 含义 | 约束 |
| --- | --- | --- |
| `qemu-aarch64-ci.config` | CI 冒烟内核基线 | `CONFIG_MODULES=n`;cgroup v1 关闭;所有"CI 裁"条目强制 `=n` |
| `qemu-aarch64-e2e.config` | 端到端内核基线 | `CONFIG_MODULES=y`;"CI 裁"条目保留 `=y` |

## 产出规则

1. **基线来源**:基于 `linux/arch/arm64/configs/defconfig` 合并、删减,产出配置片段(行式 `CONFIG_X=…`,merge_config 可加载),不重复编写完整 `.config`
2. **强制开启 / 强制关闭分列**:片段内部分区标注,如:

   ```kconfig
   # ===== 强制开启 =====
   # 40 virtio-blk/virtio-scsi/loop
   CONFIG_VIRTIO_BLK=y

   # ===== 强制关闭 =====
   # 75 cgroup v1
   CONFIG_CGROUP_*=n
   ```

3. **注释绑定条目编号**:每条 CONFIG 行头注释 `# <决策表#号> 子系统名`,决策表 ↔ 片段 1:1 可追溯;无注释行禁止出现
4. **架构**:本期仅 aarch64(`qemu-aarch64-*`)
5. **红线落实**:CI 不得含接入位硬件驱动(SPI / I2C / GPIO / V4L2 / CAN / SocketCAN / PWM / WATCHDOG / hwmon、virtio-9p / virtio-fs、FUSE / overlayfs / BPF / ftrace / perf 等);全裁项(kexec / kdump / SCHED_DEADLINE / System V IPC / POSIX mqueue / 旧协议等)均 `=n`
6. **§6 关键项覆盖**:`PREEMPT_VOLUNTARY`、`HZ=1000`、`IKCONFIG`、`MAGIC_SYSRQ`、`SECURITY` / `SECURITY_APPARMOR`、`SECCOMP`、`NAMESPACES` 全集、`CGROUP_*` v2、`STATIC_USERMODEHELPER`、`CHECKPOINT_RESTORE` 等;e2e 另含 `BPF` / `BPF_SYSCALL` / `BPF_JIT=y`、`MODULES=y`

## 自检(交付前全部通过)

| 项 | 期望 |
| --- | --- |
| 决策表覆盖 | 87 项逐条可追溯(注释连续,无缺号) |
| 硬约束 | 四条红线阻断,无违规驱动残留 |
| CI 裁处理 | ci 置 `=n` 且 e2e 置 `=y`,两份对照抽查一致 |
| 可合并 | 与 arm64 `defconfig` 经 merge_config.sh 合并后无冲突、能 savedefconfig |
| 双份差异 | 仅含契约允许差异,无多余偏离 |

## 交付清单

- 2 份 `.config` 片段
- 决策表 ↔ CONFIG 映射核对表(决策# / 子系统 / 所需 CONFIG 列表)
- 简短说明:合并命令、kconfig 依赖导致的偏离决策表之例外及理由(如有)

## 里程碑

本期仅 aarch64;amd64(x86_64)两份沿用同一决策表 + §1 平台节等价替换(pl011 → 8250、pl031 → RTC_CMOS、arch timer → TSC + HPET、PSCI → ACPI、virt DTS → q35),作为下一批次。

## 预期耗时

- 理想路径(依赖无坑、一次通过):1.5 ~ 2 h
- 常规路径(依赖连锁多轮 + savedefconfig 往返):3 ~ 5 h
- 交付标记:`savedefconfig` 合并通过 + 自检表全过
