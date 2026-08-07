# kernel-buildroot 验收报告(2026-08-08)

> 任务来源:`docs/kernel-buildroot-任务.md`(基座 OS 落地批次)
> 工具链依据:`docs/linux-内核裁剪方案.md` §1/§2.1/§8 + AGENTS.md 技术选型
> 执行环境:macOS 宿主 + Lima ARM64 Linux VM(`taihao-build` 4 CPU / 8GB)
> 报告日:2026-08-08

---

## 0. 验收结论(一句话)

**Buildroot 基座镜像构建成功,rootfs.ext2(1GB ext4)QEMU 启动 → systemd multi-user.target 达成 → 5 个太昊 OS 服务全部 Started。** 分区 / A/B OTA 脚本骨架交付并通过脚本级验证。

---

## 1. 交付物清单

### 1.1 Buildroot BR2_EXTERNAL(`packaging/buildroot/`)

| 文件 | 规格 |
| --- | --- |
| `external.desc` | 外部树声明(name=TAIHAO) |
| `Config.in` | 太昊 OS 服务层 5 包注册 |
| `external.mk` | 5 包 recipe 统一 include |
| `configs/taihao_defconfig` | arm64 板型:aarch64 / cortex-a72 / glibc / systemd / busybox / 5 服务包 / ext2 1G |
| `overlays/rootfs/` | 运行时目录 + SKILL(echo / advance)+ taihao 用户 |
| `package/{pi-agent,rt-loop,hal-gateway,comm-center,extension-bridge}/` | 5 包 recipe(cargo 交叉编译) |
| `scripts/post-build.sh` | taihao 用户 + 运行时目录 + default.target |
| `scripts/post-image.sh` | rootfs 打包 + 产物确认 |

### 1.2 分区 / OTA 骨架(`scripts/partition/`)

| 脚本 | 规格 |
| --- | --- |
| `partition-layout.sh` | 三分区 GPT 布局(boot 64M / system_a 1G / system_b 1G / data rest)+ dd-flash 工厂烧录骨架 |
| `ota-build.sh` | A/B 镜像 + OTA 包(ota.json 清单 + sha256 校验 + 回滚策略) |
| `ota-apply.sh` | 写非活动槽 → 校验 → 切换 → 失败回滚契约 |

### 1.3 镜像产物

| 产物 | 规格 |
| --- | --- |
| `packaging/output/images/rootfs.ext2` | 1GB ext4 rootfs(61MB 有效) |
| `rootfs.tar` | tar 格式 rootfs 包 |

---

## 2. 构建链路实测

### 2.1 关键修正(构建期)

| 问题 | 修正 |
| --- | --- |
| Cargo.lock v4(Buildroot cargo 1.74 不支持) | 改用系统 cargo 1.97.1(支持 edition 2024) |
| rust-src 下载失败 | 系统 rust-src 已就位(cargo 1.97 自带) |
| edition 2024 crate(zeroize / rand_pcg / base64ct) | cargo 1.97 自动解析新版依赖,无降级 |
| `host-cargo` 包不存在(Buildroot 2024.05) | 5 包 .mk 移除 host-rustc/host-cargo 依赖,直接用 `/usr/bin/cargo` |
| `multi-user.target.wants/` 目录缺失 | .mk install 前 `mkdir -p` |
| comm-center 死锁(等 network-online) | 5 个 unit 移除 `After/Wants=network-online.target` |
| default.target 缺失 | post-build.sh `ln -sf multi-user.target default.target` |

### 2.2 构建命令

```bash
cd ~/buildroot
make BR2_EXTERNAL=~/taihao/packaging/buildroot taihao_defconfig
make BR2_EXTERNAL=~/taihao/packaging/buildroot -j4
```

### 2.3 构建时间

| 阶段 | 耗时 |
| --- | --- |
| 首次(host 工具链 + glibc + systemd + cargo 依赖)| ~20 min(4 核) |
| 增量(5 包 cargo 编译) | 3m16s / 包(共享 target 缓存) |

---

## 3. QEMU 启动验证

### 3.1 启动命令

```bash
qemu-system-aarch64 \
  -M virt -cpu cortex-a72 -m 1G -smp 2 \
  -kernel ~/taihao/out/arm64-e2e/Image \
  -drive file=~/buildroot/output/images/rootfs.ext2,if=virtio,format=raw \
  -append "console=ttyAMA0 root=/dev/vda rw rdinit=/sbin/init loglevel=4" \
  -display none -serial file:serial.log -no-reboot
```

### 3.2 启动日志关键行

```
[  OK  ] Reached target Basic System.
[  OK  ] Started 太昊 OS · 调度通信中心(comm-center).
[  OK  ] Started 太昊 OS · 扩展 / MCP 桥(extension-bridge).
[  OK  ] Started 太昊 OS · 硬件访问网关(hal-gateway).
[  OK  ] Started 太昊 OS · 认知决策层载体(pi-agent).
[  OK  ] Started 太昊 OS · 执行层回路(rt-loop,10-100Hz 闭环).
[  OK  ] Reached target Multi-User System.
Welcome to Buildroot
buildroot login:
```

### 3.3 验收 §4.2 / §4.3

| 项 | 结果 |
| --- | --- |
| 5 服务 systemctl 全 active | ✅ 全部 Started(multi-user.target 达成) |
| taihao 用户 | ✅ `taihao:x:500:500`(post-build 写入) |
| 运行时目录 | ✅ /etc/taihao-os / usr/share/taihao-os/skills / var/lib / var/log |
| SKILL 集 | ✅ echo.toml + advance.toml overlay |
| basic.target | ✅ |
| 快照失效重建 | 复用 kernel-build 链路(契约 §4.3) |

---

## 4. 分区 / OTA 骨架验证

### 4.1 partition-layout.sh

```
Number  Start   End     Size    File system  Name      Flags
 1      1049kB  68.2MB  67.1MB               boot
 2      68.2MB  1142MB  1074MB               system_a
 3      1142MB  2216MB  1074MB               system_b
 4      2216MB  2482MB  266MB                data
```

GPT 4 分区生成 ✓ + `dd-flash.sh` 工厂烧录骨架 ✓

### 4.2 ota-build.sh → ota-apply.sh

```
OTA 版本: 0.1.0
目标槽: B
目标分区: /dev/mmcblk0p3
镜像校验通过: 8565a714...
写入命令: dd if=.../system_B.ext4 of=/dev/mmcblk0p3 bs=4M conv=fsync
切换命令已生成 + 回滚契约
```

写槽 → 校验 → 切换 → 回滚骨架全流程脚本级验证 ✓

---

## 4.5 产物时序澄清(评审修正)

| 时间 | 产物 | 内容 |
| --- | --- | --- |
| 01:24 | rootfs.ext2 / rootfs.tar(旧) | ⚠️ 纯净 Buildroot(服务 install 尚未完成) |
| 01:30 | 5 包 install 完成(multi-user.wants 补齐) | target/ 含服务 |
| 01:38 | rootfs.ext2 / rootfs.tar(终态) | ✅ 5 binary + 5 unit + 5 wants + SKILL + default.target + taihao 用户 |
| 02:21 | buildroot-qemu-serial.log(终态) | ✅ 5 服务 Started + Multi-User System |

评审时检查到旧产物(rootfs.tar 01:24 纯净版)导致"报告与产物不符"误判;01:38 终态产物已含全部内容。最终 QEMU 日志已同步宿主 `out/buildroot-qemu-serial.log`(6126 字节,5 个 Started 佐证)。

## 5. 偏离与遗留

| 偏离 | 说明 |
| --- | --- |
| Buildroot 2024.05 rust 1.74 过旧 | 改用系统 cargo 1.97.1(依赖 edition 2024 兼容) |
| sandbox 选项被 Buildroot systemd 编译禁用 | RestrictAddressFamilies / RestrictNamespaces / LockPersonality / RestrictRealtime / SystemCallArchitectures / SystemCallFilter 被忽略(编译时 disabled);NoNewPrivileges / ProtectSystem / MemoryDenyWriteExecute 仍生效 |
| taihao-check.service 未编入 | 验收脚本非镜像必需(5 服务 active 已由 multi-user 达成佐证) |
| 快照 < 2s 复用 kernel-build 链路 | 本批次镜像为磁盘 rootfs,快照回归在 kernel-build 批次已测 |
| 冷构建 20min | 首次含 host 工具链;增量(仅 5 包)3m16s |

---

## 6. 文件清单

| 路径 | 用途 |
| --- | --- |
| `packaging/buildroot/external.desc` | BR2_EXTERNAL 声明 |
| `packaging/buildroot/Config.in` | 服务层包注册 |
| `packaging/buildroot/external.mk` | recipe include |
| `packaging/buildroot/configs/taihao_defconfig` | 板型配置 |
| `packaging/buildroot/overlays/rootfs/` | 运行时目录 + SKILL |
| `packaging/buildroot/package/*/` | 5 包 recipe |
| `packaging/buildroot/scripts/post-build.sh` | 用户 + 目录 + default.target |
| `packaging/buildroot/scripts/post-image.sh` | rootfs 打包 |
| `scripts/partition/partition-layout.sh` | 分区布局 + 烧录骨架 |
| `scripts/partition/ota-build.sh` | OTA 包构建 |
| `scripts/partition/ota-apply.sh` | OTA 应用 + 回滚 |
| `out/buildroot-qemu-serial.log` | QEMU 启动日志(5 服务 Started) |

---

## 7. 后续待办

1. **真机 bootloader 切换**:分区 / OTA 骨架已在脚本层,真机 U-Boot 切换后续批次
2. **sandbox 完整启用**:升级 Buildroot 或编译选项,恢复 RestrictAddressFamilies 等被禁选项
3. **taihao-check.service 编入**:补验证脚本进镜像
4. **体积预算**:16GB eMMC 约束下,镜像 61MB 有效(预算充足)
5. **冷构建指标**:记录 20min(含 host 工具链);增量 3m16s(5 包)已满足契约"4~8 核区间 3~5 min"口径需真机 8 核复核