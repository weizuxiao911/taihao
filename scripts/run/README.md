# 太昊 OS · 镜像运行脚本目录 (scripts/run)
#
# 三个核心入口,文件名直接说明用途:

| 脚本 | 用途 | 窗口 | 串口落点 | 退出码 |
|---|---|---|---|---|
| qemu冒烟启动.sh | CI 烟测,验证内核能起来 | 无 | log 文件 | 0=OK / 3=超时 |
| qemu运行镜像.sh | E2E 验收,人在机器前看 | cocoa | 当前终端 | qemu 同步退出 |
| qemu调试镜像.sh | debug,systemd log 全开 | cocoa | 当前终端 + log 文件 | qemu 同步退出 |

# ===== 前置依赖 =====
1. QEMU 已装:`brew install qemu`
2. 镜像已构建:`scripts/kernel-build/build-kernel.sh e2e --initramfs-dir <rootfs>`
   (产出落在仓库根 out/arm64-e2e/{Image, initramfs.cpio, snap/boot-snap.qcow2})

# ===== iTerm2 内启动(可选,验收场景)=====
如需在新 iTerm2 窗口启动(macOS),用 osascript 包一层:

```bash
osascript -e 'tell application "iTerm" to activate' \
  -e 'tell application "iTerm" to set w to (create window with default profile)' \
  -e 'tell current session of w to write text "cd /Users/weizuxiao/Documents/水下机器人/taihao && bash scripts/run/qemu运行镜像.sh"'
```

# ===== 产物路径 =====
所有脚本都基于仓库根,自动解析:
- `out/arm64-e2e/Image`            内核镜像
- `out/arm64-e2e/initramfs.cpio`   initramfs
- `out/arm64-e2e/snap/boot-snap.qcow2`  qemu 状态盘
- `out/arm64-e2e/logs/`            串口/HMP 日志
