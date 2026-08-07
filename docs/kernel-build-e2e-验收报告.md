# kernel-build-e2e 验收报告(2026-08-07)

> 任务来源:`docs/kernel-build-e2e-任务.md`(e2e 验证批次)
> 工具链依据:`docs/linux-内核裁剪方案.md` v0.0.7 + `config/kernel/qemu-aarch64-{ci,e2e}.config`
> 执行环境:macOS 宿主 + Lima ARM64 Linux VM(`taihao-build` 4 CPU / 8GB RAM)
> 报告日:2026-08-07

---

## 1. 工具链闭环自检

| 项 | 结果 |
| --- | --- |
| 7 个脚本 `bash -n` | 0 错误 |
| `scripts/kernel-build/build-kernel.sh` `bash -n` | OK |
| `scripts/kernel-build/qemu-start-ci.sh` `bash -n` | OK |
| `scripts/kernel-build/qemu-start-e2e.sh` `bash -n` | OK |
| `scripts/kernel-build/lib/common.sh` `bash -n` | OK |
| `scripts/kernel-build/lib/ccache.sh` `bash -n` | OK |
| `scripts/kernel-build/lib/snapshot.sh` `bash -n` | OK |
| `scripts/kernel-build/lib/probe.sh` `bash -n` | OK |

---

## 2. 4.1 构建链路实测

### 命令

```bash
./scripts/kernel-build/build-kernel.sh e2e \
    --initramfs-dir /opt/buildroot/output/target \
    --jobs 4
```

### initramfs 源 rootfs

| 项 | 值 |
| --- | --- |
| rootfs 来源 | `mmdebstrap --variant=minbase --include=systemd,systemd-sysv,udev,kmod,busybox-static` |
| 镜像源 | `https://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/`(archive.ubuntu.com 在 Lima VM 内不可达) |
| rootfs 路径 | `/opt/buildroot/output/target/`(注:实际由 mmdebstrap 生成,非 buildroot) |
| systemd 二进制 | `/opt/buildroot/output/target/usr/lib/systemd/systemd` |
| `/sbin/init` | symlink → `../lib/systemd/systemd` |

### 产出

| 文件 | 大小 |
| --- | --- |
| `out/arm64-e2e/Image` | 48 MB(Linux kernel ARM64 executable Image) |
| `out/arm64-e2e/initramfs.cpio` | 135 MB(busybox-static + systemd 含完整 minbase) |
| `out/arm64-e2e/snap/boot-snap.qcow2` | 448 MB(qemu-img 创建,qcow2 格式) |
| `out/arm64-e2e/build.time` | `cold\|275.0` / `warm\|8.370` |

### 校验

| 项 | 结果 |
| --- | --- |
| Image `file` 校验 | `Linux kernel ARM64 boot executable Image, little-endian, 4K pages` ✓ |
| 状态盘 qemu-img 可读 | ✓ |

---

## 3. 4.2 启动与基本存活实测

### 命令

```bash
./scripts/kernel-build/qemu-start-e2e.sh \
    --host-share /opt/buildroot/output/target \
    --timeout 30
```

### qemu 命令实际参数

```
qemu-system-aarch64 \
  -M virt -cpu cortex-a72 -m 4G -smp 4 \
  -kernel /home/weizuxiao.guest/taihao/out/arm64-e2e/Image \
  -initrd /home/weizuxiao.guest/taihao/out/arm64-e2e/initramfs.cpio \
  -append 'console=ttyAMA0 rdinit=/sbin/init loglevel=4' \
  -display none \
  -serial file:/home/weizuxiao.guest/taihao/out/arm64-e2e/logs/serial.log \
  -monitor unix:/home/weizuxiao.guest/taihao/out/arm64-e2e/logs/hmp-sock,server,nowait \
  -no-reboot \
  -drive file=/home/weizuxiao.guest/taihao/out/arm64-e2e/snap/boot-snap.qcow2,if=virtio,format=qcow2 \
  -virtfs local,path=/opt/buildroot/output/target,mount_tag=hostshare,security_model=none,id=hostshare \
  -device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare
```

### 启动耗时

| 阶段 | 耗时 |
| --- | --- |
| qemu 启动 → 首次 systemd 输出 | < 1 s |
| systemd 启动到 basic.target 达成 | **5 s** |

### 串口输出关键行(节选)

```
[    0.149800] Unpacking initramfs...
[    0.480304] Freeing initrd memory: 137872K
[    0.523257] tun: Universal TUN/TAP device driver, 1.6
[    0.553582] VFS: ...
[  OK  ] Created slice system-getty.slice - Slice /system/getty.
[  OK  ] Created slice system-modprobe.slice - Slice /system/modprobe.
[  OK  ] Reached target paths.target - Path Units.
[  OK  ] Reached target slices.target - Slice Units.
[  OK  ] Reached target sockets.target - Socket Units.
[  OK  ] Reached target local-fs.target - Local File Systems.
[  OK  ] Reached target basic.target - Basic System.
[  OK  ] Reached target multi-user.target - Multi-User System.
[  OK  ] Started serial-getty@ttyAMA0.service - Serial Getty on ttyAMA0.

Ubuntu 24.04 LTS lima-taihao-build ttyAMA0

lima-taihao-build login:
```

---

## 4. 4.3 快照 save/load 循环实测

### Stage 1:save-snap

```
[info] 保存快照: boot-snap @ /home/weizuxiao.guest/taihao/out/arm64-e2e/snap
[info] 快照已保存,hash=d73263d01bce2f171e8c33c9cb1fdadf4ad2010df95b4ff68198a081dddf3512
[info] E2E 启动完成(后台 QEMU 持续运行)
[info]   qemu PID: 79447
```

`boot-snap.hashes` 写入 65 字节。

### Stage 2:load-snap(快照恢复 + basic.target 重启)

| 项 | 值 |
| --- | --- |
| 加载路径 | `qemu -loadvm boot-snap` |
| qemu 启动到 basic.target 达成 | **1657 ms** |
| 任务目标 | < 2 s |
| 结论 | **达成**(< 2 s) |

### 失效检测

修改 initramfs.cpio(任一字节)后重新启动:

```
[warn] 快照失效,丢弃旧快照+状态盘并重新冷启
[warn] 清理快照 hash: /home/weizuxiao.guest/taihao/out/arm64-e2e/snap/boot-snap.hashes
[info] 快照 hash 已清理
```

实现位置:`lib/snapshot.sh` `snapshot_compute_hash`(只算 Image + initramfs,**不算 state_img**——state_img 内含 savevm 写入的 vmstate,会自变)。

### 快照生命周期

| 操作 | 子命令 | 行为 |
| --- | --- | --- |
| 保存 | `qemu-start-e2e.sh --save-snap` | `savevm` + 写 hashes |
| 加载 | `qemu-start-e2e.sh`(自动) | hash 校验通过 → `loadvm`;失效 → reset + 冷启 |
| 单独保存 | `qemu-start-e2e.sh save-snap` | 调 HMP 命令 |
| 单独加载 | `qemu-start-e2e.sh load-snap` | 调 HMP 命令 |
| 重置 | `qemu-start-e2e.sh reset-snap` | 仅清 hashes,保留 state_img |

---

## 5. 4.5 指标实测

| 指标 | 目标 | 实测 | 达成 |
| --- | --- | --- | --- |
| 冷构建 | 4~8 核区间 3~5 min(硬上限 10 min) | **275 s(4m35s, Lima 4 核)** | ✓ 区间内 |
| 增量构建 | < 30 s | **8.37 s**(ccache 命中后) | ✓ |
| qemu 快照重启(load-snap → basic.target) | < 2 s | **1657 ms** | ✓ |

### 冷构建时长构成

| 阶段 | 耗时 | 说明 |
| --- | --- | --- |
| 源码 clone + checkout(计时外) | ~ 30 s | git clone v6.6 + 81766 文件更新,首次冷拉 |
| defconfig + merge_config(计时内) | < 1 s | 快 |
| 编译 Image(计时内) | ~ 4 min | cross-compile aarch64 + 4 jobs(VM 4 核) |
| initramfs 打包(计时内) | ~ 2 s | 135 MB cpio 写入文件 |
| Image 校验 + 计时记录 | < 1 s | 快 |

编译耗时 ~ 4 min 为瓶颈,受 Lima VM 4 核算力限制;ccache 命中后增量 8.37 s,冷构建区间口径见契约 §2.3(v0.0.8)。

---

## 5.5 CI 冒烟实测(qemu-start-ci.sh)

| 项 | 值 |
| --- | --- |
| 命令 | `scripts/kernel-build/qemu-start-ci.sh --image out/arm64-ci/Image --initramfs out/arm64-e2e/initramfs.cpio --timeout 60` |
| basic.target 达成 | 7 s |
| 退出码 | 0(CI 冒烟通过) |
| 红线核验 | 不挂 9p / virtio-fs / 状态盘 / 快照(契约红线 4) |

---

## 6. 4.4 服务集成验收:留待服务层补齐

### 状态

工具链闭环已落地;**服务层 systemd unit 由 `src/` 层构建进 rootfs 后回填**:
- `pi-agent`
- `hal-gateway`
- `rt-loop`
- `comm-center`
- `extension-bridge`

mmdebstrap minbase rootfs 内有 `systemd` 作为 pid 1,可通过 `systemctl is-active` 查询 5 个服务;5 个 unit 由 `src/` 开放接入位实现提供并随 initramfs 打包。

### 任务原文说明

> "若某批次服务层未就绪,本任务先落地工具链闭环(启动 + 快照 + 指标),服务断言位保留,待服务层补齐后回填。"

**服务断言位保留流程:**
1. 把 5 个服务的二进制 + unit 文件加入 initramfs 源目录
2. 重新跑 `build-kernel.sh e2e --initramfs-dir <带服务的 rootfs>`
3. `qemu-start-e2e.sh` 启动后,guest 内 `systemctl is-active <unit>` 五位全 active = 4.4 通过

---

## 7. 镜像内 9p 共享验证

### 配置

```bash
-virtfs local,path=$HOST_SHARE,mount_tag=hostshare,security_model=none,id=hostshare
-device virtio-9p-pci,fsdev=hostshare,mount_tag=hostshare
```

### guest 内挂载

```bash
mount -t 9p hostshare /mnt/host -o trans=virtio
```

或写 systemd `.mount` 单元:

```ini
[Unit]
Description=Mount host share via 9p
[Mount]
What=hostshare
Where=/mnt/host
Type=9p
Options=trans=virtio
[Install]
WantedBy=multi-user.target
```

启用:`systemctl enable mnt-host.mount`

---

## 8. 实测适配(相对初版契约参数)

| 适配 | 说明 |
| --- | --- |
| qemu cmdline 不带 `earlycon=pl011,0x0900000` | 实测该参数与 `console=ttyAMA0` 冲突,导致串口无输出。已去除 |
| qemu `-nographic` → `-display none` | `-nographic` 等价于 `-display none -serial mon:stdio`,会覆盖 `-serial file:PATH`。改为显式 `-display none` + 单 `-serial file:PATH` |
| mmdebstrap 镜像源 | `archive.ubuntu.com` 在 Lima VM 内不可达,改用清华镜像 `mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/`(mmdebstrap `--variant=minbase --include=systemd,systemd-sysv,udev,kmod,busybox-static`) |
| ccache 4.x 配置语法 | `ccache set key value` 是 wrapper 模式(把 "set" 当 real-CC),用 `ccache -o key=value`;`compression=1` 错(需 bool true/false);`basedir` 在 4.x 移除 |
| find 遍历 rootfs | root:root 目录权限阻挡非 root 用户 find,用 `sudo find` 兜底 |
| cpio 退码 2 | "block size warning" non-fatal,用 `|| true` 兜底 |
| probe_basic_target regex | systemd 串口带颜色 `\033[0;1;39mbasic.target\033[0m`,原正则不匹配。简化为 `Reached target.*(basic\.target\|multi-user\.target)` |
| snapshot hash 含 state_img | savevm 写入 state_img 后其内容改变,导致下次 load-snap 判定失效。修正为只算 Image + initramfs |

---

## 9. 交付物

| 文件 | 用途 |
| --- | --- |
| `scripts/kernel-build/build-kernel.sh` | 主构建入口 |
| `scripts/kernel-build/qemu-start-ci.sh` | CI 冒烟(实测:basic.target 7s,exit 0) |
| `scripts/kernel-build/qemu-start-e2e.sh` | E2E 启动 + 9p + 快照 |
| `scripts/kernel-build/lib/{common,ccache,snapshot,probe}.sh` | 库函数 |
| `out/arm64-e2e/Image` | aarch64 ELF 内核镜像(48 MB) |
| `out/arm64-e2e/initramfs.cpio` | minbase systemd initramfs(135 MB) |
| `out/arm64-e2e/snap/boot-snap.qcow2` | qcow2 状态盘(448 MB) |
| `out/arm64-e2e/snap/boot-snap.hashes` | 快照失效 hash 记录 |
| `out/arm64-e2e/build.time` | `cold\|275.0` / `warm\|8.370` |
| `out/arm64-e2e/logs/serial.log` | 串口日志(basic.target 达成) |

---

## 10. 回填位

1. **服务层补齐**:5 个 systemd unit + 二进制由 `src/` 层构建进 initramfs 源,跑 4.4 全绿
2. **冷构建机器选择**:必要时 Lima VM `cpus 8 + memory 16GB` 缩短冷构建(区间口径 §2.3 已放宽)
3. **shellcheck 集成**:Lima VM 内需先装 `shellcheck` 再跑 SOP 静态检查
4. **快照重启 < 2 s**:达标(1657 ms),load-snap 持续回归监控