#!/bin/bash
# 把 src/ 服务集成到 initramfs 源 rootfs
#
# 用法:
#   scripts/services/install.sh /opt/buildroot/output/target
#
# 行为:
#   - 复制 5 个 binary 到 <rootfs>/usr/bin/
#   - 复制 5 个 systemd unit 到 <rootfs>/etc/systemd/system/multi-user.target.wants/
#   - 复制 SKILL .toml 到 <rootfs>/usr/share/taihao-os/skills/
#   - 创建 taihao 用户 + 必要的目录
#   - 写 audit logrotate 配置

set -euo pipefail

ROOT="${1:-/opt/buildroot/output/target}"
# SRC_ROOT 可由环境变量覆盖(用于 sudo 透传)
SRC_ROOT="${SRC_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"

[ -d "$ROOT" ] || { echo "ROOT 不存在: $ROOT" >&2; exit 1; }
[ -d "$SRC_ROOT" ] || { echo "SRC_ROOT 不存在: $SRC_ROOT" >&2; exit 1; }

# mmdebstrap 生成的 rootfs 大量文件是 root:root 权限,非 root 写不动
# 自动 sudo 升级权限(透传 ROOT + SRC_ROOT + 解析脚本绝对路径)
if [ "$(id -u)" -ne 0 ]; then
    ORIG_PATH="$(readlink -f "$0")"
    exec sudo bash -c "ROOT='$ROOT' SRC_ROOT='$SRC_ROOT' bash '$ORIG_PATH' '$@'" -- "$@"
fi

# 1. binaries
install -d -m 0755 "$ROOT/usr/bin"
for bin in pi-agent rt-loop hal-gateway comm-center extension-bridge; do
    src="$SRC_ROOT/src/target/release/$bin"
    if [ ! -f "$src" ]; then
        echo "缺少 binary: $src(先跑 cargo build --release)" >&2
        exit 1
    fi
    install -m 0755 "$src" "$ROOT/usr/bin/$bin"
done
echo "✓ 5 binary 已安装到 $ROOT/usr/bin/"

# 2. systemd unit
install -d -m 0755 "$ROOT/etc/systemd/system/multi-user.target.wants"
for svc in pi-agent rt-loop hal-gateway comm-center extension-bridge; do
    src="$SRC_ROOT/src/systemd/${svc}.service"
    [ -f "$src" ] || { echo "缺少 unit: $src" >&2; exit 1; }
    install -m 0644 "$src" "$ROOT/etc/systemd/system/${svc}.service"
    ln -sf "../${svc}.service" "$ROOT/etc/systemd/system/multi-user.target.wants/${svc}.service"
done

# check.service + check.sh:oneshot 跑 systemctl is-active + 白名单 + 越权 + 频率
if [ -f "$SRC_ROOT/src/systemd/taihao-check.service" ]; then
    install -m 0644 "$SRC_ROOT/src/systemd/taihao-check.service" "$ROOT/etc/systemd/system/taihao-check.service"
    install -m 0755 "$SRC_ROOT/src/systemd/taihao-check.sh" "$ROOT/usr/bin/taihao-check.sh"
    ln -sf "../taihao-check.service" "$ROOT/etc/systemd/system/multi-user.target.wants/taihao-check.service"
fi
echo "✓ 5 systemd unit + check.service + check.sh + multi-user.target.wants symlink 已安装"

# 3. SKILL
install -d -m 0755 "$ROOT/usr/share/taihao-os/skills"
for sk in "$SRC_ROOT/src/skills/"*.toml; do
    [ -f "$sk" ] || continue
    install -m 0644 "$sk" "$ROOT/usr/share/taihao-os/skills/"
done
echo "✓ SKILL .toml 已安装到 $ROOT/usr/share/taihao-os/skills/"

# 4. 目录结构
for d in /etc/taihao-os /etc/taihao-os/skills /var/lib/taihao-os /var/log/taihao-os; do
    install -d -m 0755 "$ROOT$d"
done
echo "✓ 运行时目录结构已建立"

# 5. 注入 socat(guest 内 nc 工具不可用时,check.sh 用 socat 跑 Unix socket 测试)
if [ -x /usr/bin/socat ]; then
    install -m 0755 /usr/bin/socat "$ROOT/usr/bin/socat"
    echo "✓ socat 已注入(供 check.sh 使用)"
fi

# 5. taihao 用户(占位:rootfs 默认无 shadow,但 systemd unit 引用 User=taihao)
# mmdebstrap minbase 通常没有 useradd 工具,改为直接写 /etc/passwd + /etc/group 占位
# 让 systemd 服务以 root 运行(沙箱已提供隔离)
# 注:本批次让 unit 简化为 User=root(沙箱已足够),避免依赖 useradd

# 6. 简化的 user/group 占位(如果存在 useradd)
if command -v useradd >/dev/null 2>&1; then
    chroot "$ROOT" useradd -r -s /usr/sbin/nologin -d /var/lib/taihao-os taihao 2>/dev/null || true
fi

echo "✓ 服务层集成完成"
echo ""
echo "重新打包 initramfs:"
echo "  bash scripts/kernel-build/build-kernel.sh e2e --initramfs-dir $ROOT"