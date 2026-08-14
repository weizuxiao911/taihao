#!/bin/bash
# Buildroot post-build 脚本:太昊 OS 运行时目录 + taihao 系统用户
#
# 依据:AGENTS.md 运行时路径
#   /etc/taihao-os/           全局配置
#   /usr/share/taihao-os/skills/  系统级 SKILL
#   /var/lib/taihao-os/       运行时持久化
#   /var/log/taihao-os/       审计日志

set -euo pipefail

TARGET_DIR="${TARGET_DIR:-$1}"

# ===== 运行时目录(overlay 已建,这里补权限) =====
for d in \
    "$TARGET_DIR/etc/taihao-os" \
    "$TARGET_DIR/etc/taihao-os/skills" \
    "$TARGET_DIR/usr/share/taihao-os/skills" \
    "$TARGET_DIR/var/lib/taihao-os" \
    "$TARGET_DIR/var/log/taihao-os"; do
    mkdir -p "$d"
done
chmod 0755 "$TARGET_DIR/etc/taihao-os" "$TARGET_DIR/usr/share/taihao-os/skills"
chmod 0750 "$TARGET_DIR/var/lib/taihao-os" "$TARGET_DIR/var/log/taihao-os"

# ===== taihao 系统用户(直接写 passwd/group,不依赖 useradd) =====
# uid 500(避开系统保留区间)
if ! grep -q '^taihao:' "$TARGET_DIR/etc/passwd" 2>/dev/null; then
    echo 'taihao:x:500:500:太昊 OS 服务用户:/var/lib/taihao-os:/usr/sbin/nologin' >> "$TARGET_DIR/etc/passwd"
fi
if ! grep -q '^taihao:' "$TARGET_DIR/etc/group" 2>/dev/null; then
    echo 'taihao:x:500:' >> "$TARGET_DIR/etc/group"
fi
if ! grep -q '^taihao:' "$TARGET_DIR/etc/shadow" 2>/dev/null; then
    echo 'taihao:!:19701:0:99999:7:::' >> "$TARGET_DIR/etc/shadow"
fi

echo "post-build: taihao 用户 + 运行时目录就绪"

# ===== systemd 配置 =====
# 强制 cgroup v2(契约 cgroup v1 全裁)
if [ -d "$TARGET_DIR/etc/systemd/system.conf.d" ]; then
    mkdir -p "$TARGET_DIR/etc/systemd/system.conf.d"
fi
mkdir -p "$TARGET_DIR/etc/systemd/system.conf.d"
cat > "$TARGET_DIR/etc/systemd/system.conf.d/10-taihao.conf" <<'EOF'
[Manager]
DefaultCPUAccounting=yes
DefaultMemoryAccounting=yes
EOF

# ===== 太昊 OS 验收口径:默认用户 = root,所有 systemd 服务跑 root =====
# 解决 dbus-daemon 警告 "Unknown username systemd-network/systemd-resolve/systemd-timesync"
# 这些用户由 mkusers(target-finalize + fakeroot 各跑一次)添加。
# 关键:target-finalize 跑完后,/etc/passwd 已经有这些用户(auto-uid)。
#       但 fakeroot(mkfs.ext2 时)会再次跑 mkusers,看到已有同名用户会冲突。
# 解法:post-build.sh 把 mkusers 加的同名用户清掉,留给 fakeroot mkusers 重新加(空目录 → 无冲突)。

for name in dbus systemd-network systemd-resolve systemd-timesync messagebus; do
    sed -i "/^${name}:/d" "$TARGET_DIR/etc/passwd" "$TARGET_DIR/etc/group" 2>/dev/null || true
done

# 清掉之前 debug 留下的 dbus drop-in(--console / --nosyslog 是临时 debug 参数)
rm -f "$TARGET_DIR/etc/systemd/system/dbus.service.d/20-debug-console.conf"
rm -f "$TARGET_DIR/etc/systemd/system/dbus.service.d/20-nosyslog.conf"

# dbus service 跑 root(本机验收口径)
mkdir -p "$TARGET_DIR/etc/systemd/system/dbus.service.d"
cat > "$TARGET_DIR/etc/systemd/system/dbus.service.d/10-root.conf" <<'EOF'
[Service]
User=root
Group=root
EOF
chown -R 0:0 "$TARGET_DIR/run/dbus" 2>/dev/null || true
[ -s "$TARGET_DIR/etc/machine-id" ] || printf '%032x\n' "$(awk 'BEGIN{srand();printf "%d",rand()*4294967295}')" > "$TARGET_DIR/etc/machine-id"

echo "post-build: dbus/systemd-network/systemd-resolve/systemd-timesync 用户让 fakeroot mkusers 接管"

# ===== systemd-remount-fs.service mask =====
# initramfs 没 /dev/root 块设备,remount 必然 ENXIO → FAILED
ln -sf /dev/null "$TARGET_DIR/etc/systemd/system/systemd-remount-fs.service"

echo "post-build: systemd-remount-fs masked (initramfs 无 /dev/root)"

# ===== default.target → multi-user.target(服务自启入口) =====
ln -sf /usr/lib/systemd/system/multi-user.target \
    "$TARGET_DIR/etc/systemd/system/default.target"

echo "post-build: default.target → multi-user.target"

# ===== 太昊 OS 验收口径:登录门面 =====
# /etc/issue  →  "Welcome to TAIHAO"(agetty 在 login 提示之前输出)
printf 'Welcome to TAIHAO\n' > "$TARGET_DIR/etc/issue"

# /etc/hostname  →  TAIHAO(agetty "<hostname> login: " 用)
printf 'TAIHAO\n' > "$TARGET_DIR/etc/hostname"

# 自定义登录提示符 "username: "
# 思路:在 serial-getty@.service 上加 drop-in,把 agetty 替换成 /sbin/taihao-login
#       taihao-login 打印 "username: " 读一行,exec /bin/login "$REPLY"
mkdir -p "$TARGET_DIR/usr/sbin"
cat > "$TARGET_DIR/usr/sbin/taihao-login" <<'EOF'
#!/bin/sh
# 太昊 OS 自定义登录提示:用 "username: " 替代 agetty 默认的 "<host> login: "
printf 'username: '
read -r user
[ -n "$user" ] || exit 1
exec /bin/login "$user"
EOF
chmod 0755 "$TARGET_DIR/usr/sbin/taihao-login"

# 覆盖 serial-getty@.service:ExecStart 用 taihao-login
mkdir -p "$TARGET_DIR/etc/systemd/system/serial-getty@.service.d"
cat > "$TARGET_DIR/etc/systemd/system/serial-getty@.service.d/10-taihao-login.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/usr/sbin/taihao-login
Type=idle
EOF

echo "post-build: 登录门面 → Welcome to TAIHAO + username: 提示"

# /etc/os-release 也被 systemd 早期用来打 "Welcome to ${PRETTY_NAME}"
# 把 PRETTY_NAME / NAME / ID 改成 TAIHAO,黑盒看不到任何 "Buildroot"
cat > "$TARGET_DIR/usr/lib/os-release" <<EOF
NAME=TAIHAO
VERSION="0.1.0"
ID=taihao
VERSION_ID=0.1.0
PRETTY_NAME="TAIHAO 0.1.0"
HOME_URL="https://taihao.local"
EOF
ln -sf /usr/lib/os-release "$TARGET_DIR/etc/os-release"

echo "post-build: os-release 改 TAIHAO(去掉 Buildroot 字样)"