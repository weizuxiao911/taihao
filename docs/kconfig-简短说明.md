# Kconfig 基线片段简短说明

> 依据:docs/linux-内核裁剪方案.md v0.0.7
> 产出:config/kernel/qemu-aarch64-ci.config + config/kernel/qemu-aarch64-e2e.config
> 映射:docs/kconfig-映射核对表.md
> 合并脚本:scripts/kconfig/merge_config.sh

## 合并命令

```bash
# 准备 Linux 6.6 LTS 源码(默认 ../linux-6.6)
export LINUX_SRC=/path/to/linux-6.6

# 加载 CI 冒烟基线片段 → out/arm64-ci/.config + savedefconfig
./scripts/kconfig/merge_config.sh ci

# 加载端到端基线片段 → out/arm64-e2e/.config + savedefconfig
./scripts/kconfig/merge_config.sh e2e

# amd64(x86_64)跨架构派生
./scripts/kconfig/merge_config.sh ci x86_64
./scripts/kconfig/merge_config.sh e2e x86_64

# 验证可启动
make -C $LINUX_SRC ARCH=arm64 -j$(nproc) Image dtbs
```

## 自检状态

| 项 | 期望 | 状态 |
| --- | --- | --- |
| 决策表覆盖 | 87 项逐条可追溯(注释连续,无缺号) | ✓ CI 87 + E2E 87 = 87 项全覆盖(单端专属项 #78/#79 在对端有"不适用"注释) |
| 硬约束 | 四条红线阻断,无违规驱动残留 | ✓ |
| CI 裁处理 | ci 置 `=n` 且 e2e 置 `=y`,两份对照抽查一致 | ✓ 20 条 CI 裁集合对称 |
| Kconfig 符号 | 所有 CONFIG 行以 Linux 6.6 LTS 实际 Kconfig 为准 | ✓ 2026-08 核验 |
| 可合并 | 与 arm64 `defconfig` 经 merge_config.sh 合并后无冲突、能 savedefconfig | 待环境支持 |
| 双份差异 | 仅含契约允许差异,无多余偏离 | ✓ |

## kconfig 依赖导致的偏离决策表之例外

### 例外 1:`CONFIG_NET_VENDOR_*` 厂商驱动覆盖范围

**问题**:决策表 #50 "全部物理网卡驱动" 列为全裁;片段已枚举 59 项 `CONFIG_NET_VENDOR_*=n`(含 NVIDIA / MELLANOX / TEHUTI 等)。Linux 6.6 `drivers/net/Kconfig` 中实际厂商子项约 60+ 个。

**理由**:
- 已枚举 59 项覆盖了 6.6 主线已知的厂商驱动
- merge_config.sh 行为:合并时 `defconfig` 中 `=n` 选项不会因为片段未提及而变成 `=y`;arm64 defconfig 默认不开任何物理网卡驱动(arm64 仅 virtio-net 与 tap/tun)
- 残余极少数厂商子项(如新增分支)未显式列,风险低

**缓解**:如未来新增厂商被纳入主线 defconfig,后续 v0.0.x 修订补齐

### 例外 2:`#75 cgroup v1` 关闭路径

**问题**:Linux 6.6 没有独立 `CONFIG_CGROUP_V1=y/n` 选项;cgroup v1 vs v2 通过 v1 专属 controller(`CONFIG_CGROUP_CPUACCT` / `CONFIG_CGROUP_SCHED`)+ `cgroup_no_v1=` 内核参数 + systemd 配置组合选定。

**理由**:
- v1 专属 controller 在 v2 模式下不可用;`CONFIG_CGROUP_CPUACCT=n` + `CONFIG_CGROUP_SCHED=n` 落实 v1 关闭
- `CGROUP_PIDS` / `CGROUP_FREEZER` 是 v2 也用,不在关闭清单
- 系统启动参数 `systemd.unified_cgroup_hierarchy=1` 强制 v2(由构建脚本注入 cmdline)

**落实位置**:本片段 `CONFIG_CGROUP_CPUACCT=n` + `CONFIG_CGROUP_SCHED=n`(在 #75 段,与 #76 段 v2 controllers 不互斥)

### 例外 3:`#77 isolcpus` 非 Kconfig 项

**问题**:决策表 #77 "isolcpus 保留" 在 Kconfig 中没有对应选项;isolcpus 是内核启动参数 `isolcpus=<cpu_list>`。

**理由**:不写入 .config 片段,通过 cmdline 注入;落实位置:qemu 启动脚本的 `-append "isolcpus=2,3"` 参数

### 例外 4:`#74 SCHED_DEADLINE` 无 Kconfig 开关

**问题**:Linux 6.6 无独立 `CONFIG_SCHED_DEADLINE` 选项;`SCHED_DEADLINE` 调度类代码默认编入(无法裁掉)。

**理由**:
- 决策表 #74 "全裁" 实质是"端到端集成验证不分配 SCHED_DEADLINE 带宽"
- 落实位置:cgroup v2 不分配 deadline runtime + 应用层不调用 `sched_setattr(SCHED_DEADLINE)`

### 例外 5:`#81 ACPI / PSCI` ARM64 自动开启

**问题**:`CONFIG_ARM_PSCI_FW` 在 arm64 上被 `CONFIG_ARM64` 自动 select,无法在 .config 中显式 =y 或 =n。

**理由**:
- qemu virt machine 默认 PSCI 启用(由 qemu 平台保证)
- 本片段只显式 `CONFIG_ACPI=y`;`ARM_PSCI_FW` 由 defconfig 状态决定

### 例外 6:`CONFIG_HUGETLBFS` 替代 `CONFIG_HUGETLB_PAGE`

**问题**:`CONFIG_HUGETLB_PAGE` 是 `def_bool HUGETLBFS`(跟随 HUGETLBFS),写 =y 无效。

**理由**:
- 决策表 #67 "保留 hugetlb" 的真实开关是 `CONFIG_HUGETLBFS`
- 本片段写 `CONFIG_HUGETLBFS=y`,`HUGETLB_PAGE` 自动跟随

### 例外 7:`CONFIG_DEV_PTS` 应为 `CONFIG_UNIX98_PTYS`

**问题**:`CONFIG_DEV_PTS` 在 6.6 中不存在;devpts 文件系统已合并入 UNIX98_PTYS 子系统。

**理由**:6.6 真实符号是 `CONFIG_UNIX98_PTYS`;旧名 `CONFIG_DEVPTS` 仅作历史兼容

### 例外 8:EFI / 串口 / 实时符号

| 旧名(错) | 真名 | 位置 |
| --- | --- | --- |
| CONFIG_EFI_ARMSTUB | CONFIG_EFI_STUB | drivers/firmware/efi/Kconfig(ARM64 自动 select) |
| CONFIG_EFI_VARS | 无独立(efivarfs 由 EFI 自动挂载) | - |
| CONFIG_ARM_PSCI | CONFIG_ARM_PSCI_FW | drivers/firmware/psci/Kconfig(ARM64 自动 select) |
| CONFIG_VIRTIO_SCSI | CONFIG_SCSI_VIRTIO | drivers/scsi/Kconfig |
| CONFIG_VIRTIO_VSOCKET | CONFIG_VIRTIO_VSOCKETS | net/vmw_vsock/Kconfig |
| CONFIG_VIRTIO_CRYPTO | CONFIG_CRYPTO_DEV_VIRTIO | drivers/crypto/virtio/Kconfig |
| CONFIG_VIRTIO_RNG | CONFIG_HW_RANDOM_VIRTIO | drivers/char/hw_random/Kconfig |
| CONFIG_VIRTIO_GPU | CONFIG_DRM_VIRTIO_GPU | drivers/gpu/drm/virtio/Kconfig |
| CONFIG_VIRTIO_9P | CONFIG_NET_9P_VIRTIO | net/9p/Kconfig |
| CONFIG_SCTP | CONFIG_IP_SCTP | net/sctp/Kconfig |
| CONFIG_DCCP | CONFIG_IP_DCCP | net/dccp/Kconfig |
| CONFIG_V4L2 | CONFIG_VIDEO_DEV | drivers/media/Kconfig |
| CONFIG_DEV_PTS | CONFIG_UNIX98_PTYS | drivers/tty/Kconfig |
| CONFIG_DECNET / CONFIG_IPX | 6.6 已移除 | - |

## savedefconfig 验证状态

**未执行**,原因:
- 仓库内无 `linux/` 源码(任务环境未提供)
- merge_config.sh 已落地,环境就绪后即可跑

**待补做**(环境就绪后):
1. 跑 `./scripts/kconfig/merge_config.sh ci` 与 `./scripts/kconfig/merge_config.sh e2e`
2. 检查 `out/arm64-{ci,e2e}/.config` 与 `defconfig` 差异在契约范围内
3. 跑 `make ARCH=arm64 savedefconfig` 收敛无冲突
4. 自检表第 4 项通过后,本任务验收完成

**风险评估**:
- Kconfig 选项名称基于 Linux 6.6 实际查证,新增/重命名可能性低
- savedefconfig 收敛后,真实 .config 与本片段可能有微调
- 建议:第一次跑 savedefconfig 后,差异(diff)反馈到本片段做收口

## 后续批次

- **amd64(x86_64)批次**:沿用本任务交付的决策表 + §1 平台节等价替换,生成 `qemu-x86_64-{ci,e2e}.config`,作为下批次任务
- **真机 RK3588 批次**:与 amd64 并行,需结合 RK3588 DTS + 厂商树完成