# 决策表 ↔ CONFIG 映射核对表

> 依据:docs/linux-内核裁剪方案.md v0.0.7
> 覆盖:决策表 87 项 (#1-#87)
> 映射:每条决策 → 涉及的 CONFIG_ 列表(CI / E2E 一致集合;判级差异见 CONFIG 映射矩阵)
> 已对齐 Linux 6.6 LTS 真实 Kconfig 符号(2026-08 核验)

## §5.1 网络协议栈

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 1 | TCP / UDP / IPv4 / IPv6 | 保留 | CONFIG_INET, CONFIG_IPV6 |
| 2 | ICMP / ping | 保留 | (无独立 CONFIG_;CONFIG_INET 子项) |
| 3 | unix domain socket / packet socket / netlink | 保留 | CONFIG_UNIX, CONFIG_PACKET, CONFIG_NET |
| 4 | loopback / bridge | 保留 | CONFIG_LOOPBACK, CONFIG_BRIDGE |
| 5 | SCTP / DCCP / RDS / TIPC / RXRPC | 全裁 | CONFIG_IP_SCTP=n, CONFIG_IP_DCCP=n, CONFIG_RDS=n, CONFIG_TIPC=n, CONFIG_RXRPC=n |
| 6 | Appletalk / IPX / DECnet / WAN | 全裁 | CONFIG_ATALK=n, CONFIG_X25=n, CONFIG_PHONET=n;(DECnet/IPX 已在 6.6 移除) |
| 7 | Bluetooth / WiFi 协议栈 | 全裁 | CONFIG_BT=n, CONFIG_CFG80211=n, CONFIG_MAC80211=n |
| 8 | NFC / IRDA / amateur radio | 全裁 | CONFIG_NFC=n, CONFIG_IRDA=n, CONFIG_AMATEUR_RADIO=n |
| 9 | CAN / SocketCAN | CI 裁 | CONFIG_CAN, CONFIG_CAN_RAW, CONFIG_CAN_BCM, CONFIG_CAN_GW, CONFIG_CAN_DEV, CONFIG_CAN_CALC_BITTIMING |

## §5.2 文件系统

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 10 | ext4 | 保留 | CONFIG_EXT4_FS |
| 11 | tmpfs / devtmpfs / devpts / sysfs / proc / configfs | 保留 | CONFIG_TMPFS, CONFIG_TMPFS_POSIX_ACL, CONFIG_DEVTMPFS, CONFIG_DEVTMPFS_MOUNT, CONFIG_UNIX98_PTYS, CONFIG_SYSFS, CONFIG_PROC_FS, CONFIG_CONFIGFS_FS |
| 12 | overlayfs | CI 裁 | CONFIG_OVERLAY_FS |
| 13 | virtio-9p / virtio-fs | CI 裁 | CONFIG_NET_9P_VIRTIO, CONFIG_9P_FS, CONFIG_VIRTIO_FS |
| 14 | squashfs | 可选 | CONFIG_SQUASHFS |
| 15 | FUSE | CI 裁 | CONFIG_FUSE_FS, CONFIG_FUSE_PASSTHROUGH |
| 16 | btrfs / xfs / f2fs / NTFS / exFAT | 全裁 | CONFIG_BTRFS_FS=n, CONFIG_XFS_FS=n, CONFIG_F2FS_FS=n, CONFIG_NTFS_FS=n, CONFIG_EXFAT_FS=n |
| 17 | jffs2 / ubifs / cramfs | 全裁 | CONFIG_JFFS2_FS=n, CONFIG_UBIFS_FS=n, CONFIG_CRAMFS=n |
| 18 | NFS client | 可选 | CONFIG_NFS_FS, CONFIG_NFSD |

## §5.3 安全 / 沙箱

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 19 | namespaces 全集 | 保留 | CONFIG_NAMESPACES, CONFIG_PID_NS, CONFIG_NET_NS, CONFIG_IPC_NS, CONFIG_UTS_NS, CONFIG_USER_NS, CONFIG_MNT_NS, CONFIG_CGROUP_NS |
| 20 | seccomp BPF | 保留 | CONFIG_SECCOMP, CONFIG_SECCOMP_FILTER |
| 21 | capabilities | 保留 | CONFIG_MULTIUSER(隐含) |
| 22 | AppArmor | 保留 | CONFIG_SECURITY, CONFIG_SECURITY_APPARMOR |
| 23 | SELinux / SMACK | 全裁 | CONFIG_SECURITY_SELINUX=n, CONFIG_SECURITY_SMACK=n |
| 24 | Yama | 保留 | CONFIG_SECURITY_YAMA |
| 25 | IMA / EVM | 可选 | CONFIG_INTEGRITY, CONFIG_INTEGRITY_MACHINE_KEYRING |
| 26 | LandLock | 可选 | CONFIG_SECURITY_LANDLOCK |
| 27 | TPM | 全裁 | CONFIG_TCG_TPM=n, CONFIG_TCG_TIS=n, CONFIG_TCG_TIS_I2C=n |

## §5.4 调试 / 可观测性

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 28 | printk / magic SysRq / IKCONFIG | 保留 | CONFIG_PRINTK, CONFIG_MAGIC_SYSRQ, CONFIG_IKCONFIG, CONFIG_IKCONFIG_PROC |
| 29 | ftrace / perf events | CI 裁 | CONFIG_FTRACE, CONFIG_FUNCTION_TRACER, CONFIG_PERF_EVENTS |
| 30 | kprobes / uprobes / bpf syscall / BPF JIT | CI 裁 | CONFIG_KPROBES, CONFIG_UPROBES, CONFIG_BPF, CONFIG_BPF_SYSCALL, CONFIG_BPF_JIT, CONFIG_HAVE_EBPF_JIT |
| 31 | kdump / crashkernel | 全裁 | CONFIG_CRASH_DUMP=n |
| 32 | lockdep / kmemleak / kcsan / KASAN / UBSAN | 可选 | CONFIG_LOCKDEP, CONFIG_DEBUG_LOCKDEP, CONFIG_DEBUG_KMEMLEAK, CONFIG_KCSAN, CONFIG_KASAN, CONFIG_UBSAN |
| 33 | kgdb / kdb | 全裁 | CONFIG_KGDB=n, CONFIG_KGDB_SERIAL_CONSOLE=n, CONFIG_KGDB_KDB=n, CONFIG_KDB=n |

## §5.5 IPC

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 34 | pipe / unix socket / eventfd / signalfd / timerfd / inotify / fanotify / epoll | 保留 | CONFIG_EPOLL, CONFIG_EVENTFD, CONFIG_SIGNALFD, CONFIG_TIMERFD, CONFIG_INOTIFY_USER, CONFIG_FANOTIFY, CONFIG_UNIX |
| 35 | System V IPC | CI 裁 | CONFIG_SYSVIPC, CONFIG_SYSVIPC_SYSCTL |
| 36 | POSIX mqueue | CI 裁 | CONFIG_POSIX_MQUEUE, CONFIG_POSIX_MQUEUE_SYSCTL |
| 37 | POSIX shmem | 保留 | (隐含在 CONFIG_TMPFS_POSIX_ACL=y) |
| 38 | 遗留 IPC | 全裁 | (无独立 CONFIG_) |

## §5.6 块设备 / IO

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 39 | virtio-blk / virtio-scsi / loop | 保留 | CONFIG_VIRTIO_BLK, CONFIG_SCSI_VIRTIO, CONFIG_BLK_DEV_LOOP, CONFIG_VIRTIO, CONFIG_VIRTIO_MMIO |
| 40 | io_uring | 保留 | CONFIG_IO_URING |
| 41 | DM (device-mapper) | CI 裁 | CONFIG_BLK_DEV_DM, CONFIG_DM_CRYPT, CONFIG_DM_SNAPSHOT, CONFIG_DM_THIN_PROVISIONING, CONFIG_DM_ZERO, CONFIG_DM_RAID, CONFIG_DM_VERITY |
| 42 | md (RAID) | 可选 | CONFIG_MD |
| 43 | MFM / RLL / ESDI / IDE / PATA | 全裁 | CONFIG_IDE=n, CONFIG_BLK_DEV_IDE=n |
| 44 | mtip32xx | 全裁 | CONFIG_BLK_DEV_MTIP32XX=n |

## §5.7 网络设备

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 45 | virtio-net / loopback | 保留 | CONFIG_VIRTIO_NET, CONFIG_LOOPBACK |
| 46 | tun / tap | CI 裁 | CONFIG_TUN, CONFIG_VETH |
| 47 | virtio-balloon | 保留 | CONFIG_VIRTIO_BALLOON |
| 48 | virtio-vsock | CI 裁 | CONFIG_VIRTIO_VSOCKETS, CONFIG_VSOCKETS |
| 49 | virtio-crypto | CI 裁 | CONFIG_CRYPTO_DEV_VIRTIO |
| 50 | 全部物理网卡驱动 | 全裁 | CONFIG_NET_VENDOR_*=n(40+ 厂商驱动) |
| 51 | WiFi / WWAN / Bluetooth 设备驱动 | 全裁 | CONFIG_WWAN=n, CONFIG_WWAN_HWSIM=n |

## §5.8 输入 / 显示 / 控制台

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 52 | virtio-console | 保留 | CONFIG_VIRTIO_CONSOLE |
| 53 | 8250 / pl011 / AMBA-pl011 串口 | 保留 | CONFIG_SERIAL_8250, CONFIG_SERIAL_8250_CONSOLE, CONFIG_SERIAL_AMBA_PL011, CONFIG_SERIAL_AMBA_PL011_CONSOLE |
| 54 | virtio-input | CI 裁 | CONFIG_VIRTIO_INPUT, CONFIG_INPUT_EVDEV |
| 55 | virtio-gpu + DRM / KMS | CI 裁 | CONFIG_DRM_VIRTIO_GPU, CONFIG_DRM, CONFIG_DRM_KMS_HELPER |
| 56 | framebuffer 简单驱动 | CI 裁 | CONFIG_FB, CONFIG_FB_SIMPLE |
| 57 | HID / 触摸 / 物理键盘驱动 | 全裁 | CONFIG_HID=n, CONFIG_INPUT_MOUSE=n, CONFIG_INPUT_TOUCHSCREEN=n, CONFIG_INPUT_KEYBOARD=n, CONFIG_INPUT_TABLET=n, CONFIG_INPUT_JOYSTICK=n |
| 58 | 声卡(OSS / ALSA / sound) | 全裁 | CONFIG_SOUND=n, CONFIG_SND=n, CONFIG_SND_PCI=n, CONFIG_SND_USB=n |

## §5.9 字符 / 平台设备

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 59 | SPI / I2C / GPIO(spidev / i2c-dev / gpiochip) | CI 裁 | CONFIG_SPI, CONFIG_SPI_SPIDEV, CONFIG_I2C, CONFIG_I2C_CHARDEV, CONFIG_GPIOLIB, CONFIG_GPIO_SYSFS |
| 60 | PWM / pwmchip | CI 裁 | CONFIG_PWM |
| 61 | V4L2 / media / videobuf2 | CI 裁 | CONFIG_VIDEO_DEV, CONFIG_VIDEO_V4L2_SUBDEV_API, CONFIG_MEDIA_USB_SUPPORT, CONFIG_V4L_PLATFORM_DRIVERS |
| 62 | WATCHDOG / hwmon | CI 裁 | CONFIG_WATCHDOG, CONFIG_WATCHDOG_CORE, CONFIG_HWMON |
| 63 | RTC(pl031 / rtc-generic) | 保留 | CONFIG_RTC_CLASS, CONFIG_RTC_DRV_PL031, CONFIG_RTC_DRV_GENERIC |
| 64 | CPU frequency / cpuidle | 保留 | CONFIG_CPU_FREQ, CONFIG_CPU_FREQ_STAT, CONFIG_CPU_IDLE |
| 65 | virtio-rng | 保留 | CONFIG_HW_RANDOM_VIRTIO |
| 66 | USB 控制器驱动(xhci / ehci / ohci) | CI 裁 | CONFIG_USB_SUPPORT, CONFIG_USB, CONFIG_USB_XHCI_HCD, CONFIG_USB_EHCI_HCD, CONFIG_USB_OHCI_HCD |

## §5.10 电源 / 内存

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 67 | THP / hugetlb / KSM | 保留 | CONFIG_TRANSPARENT_HUGEPAGE, CONFIG_HUGETLBFS, CONFIG_KSM |
| 68 | zswap / zram | 可选 | CONFIG_ZSWAP, CONFIG_ZRAM |
| 69 | swap | 可选 | CONFIG_SWAP |
| 70 | suspend / hibernate | 全裁 | CONFIG_SUSPEND=n, CONFIG_HIBERNATION=n |
| 71 | NUMA balancing | 全裁 | CONFIG_NUMA_BALANCING=n |

## §5.11 调度 / cgroup

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 72 | CFS + SCHED_NORMAL/BATCH/IDLE | 保留 | CONFIG_CFS_BANDWIDTH, CONFIG_FAIR_GROUP_SCHED |
| 73 | SCHED_FIFO / SCHED_RR | 保留 | CONFIG_RT_GROUP_SCHED |
| 74 | SCHED_DEADLINE | 全裁 | (无独立 CONFIG_;代码默认编入,通过 cgroup v2 不分配 SCHED_DEADLINE 带宽落实) |
| 75 | cgroup v1 | 全裁 | CONFIG_CGROUP_CPUACCT=n, CONFIG_CGROUP_SCHED=n(v1 专属 controller) |
| 76 | cgroup v2 controllers | 保留 | CONFIG_CGROUPS, CONFIG_MEMCG, CONFIG_MEMCG_SWAP, CONFIG_MEMCG_KMEM, CONFIG_BLK_CGROUP, CONFIG_CGROUP_WRITEBACK, CONFIG_CGROUP_PIDS, CONFIG_CGROUP_FREEZER |
| 77 | isolcpus | 保留 | (命令行参数;不需 Kconfig) |

## §5.12 模块化

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 78 | CONFIG_MODULES (端到端) | 保留 | CONFIG_MODULES, CONFIG_MODULE_UNLOAD |
| 79 | CONFIG_MODULES (CI 冒烟) | 全裁 | CONFIG_MODULES=n, CONFIG_MODULE_UNLOAD=n |

## §5.13 启动 / 平台

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 80 | EFI / U-Boot boot | 保留 | CONFIG_EFI, CONFIG_EFI_STUB |
| 81 | ACPI / PSCI | 保留 | CONFIG_ACPI(ARM64 自动 select CONFIG_ARM_PSCI_FW) |
| 82 | earlycon / earlyprintk | 保留 | CONFIG_SERIAL_EARLYCON(ARM64 无 CONFIG_EARLY_PRINTK) |
| 83 | arm64 generic timer / qemu virt DTS | 保留 | (ARM64 自动 select CONFIG_ARM_ARCH_TIMER 等) |
| 84 | kexec | 全裁 | CONFIG_KEXEC=n, CONFIG_KEXEC_FILE=n |

## §5.14 NPU / GPU / VPU

| 决策# | 子系统 | 判级 | CONFIG_ 列表 |
| --- | --- | --- | --- |
| 85 | NPU 厂商驱动(RKNN / 华为 NPU / …) | 全裁 | 无主线 CONFIG;厂商树分支 |
| 86 | GPU 厂商驱动(Mali / Adreno / …) | 全裁 | CONFIG_DRM_MALI=n, CONFIG_DRM_AMDGPU=n, CONFIG_DRM_RADEON=n, CONFIG_DRM_NOUVEAU=n, CONFIG_DRM_I915=n |
| 87 | VPU 视频编解码硬件加速 | 全裁 | CONFIG_VIDEO_CODEC_ENCODER=n, CONFIG_VIDEO_CODEC_DECODER=n |

## §6 关键 CONFIG_ 单独标记(决策表外)

| CONFIG_ | 取值 | 备注 |
| --- | --- | --- |
| CONFIG_PREEMPT_VOLUNTARY | y | 不上 RT patch;迭代速度优先 |
| CONFIG_HZ | 1000 | rt-loop 抖动窗口 |
| CONFIG_NO_HZ_FULL | y | 隔离核更平滑 |
| CONFIG_STATIC_USERMODEHELPER | y | 安全 |
| CONFIG_CHECKPOINT_RESTORE | y | 系统恢复 |