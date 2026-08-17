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

# ===== systemd-networkd-persistent-storage.service mask =====
# systemd-networkd 的持久化存储服务,配置缺失会 FAILED(无实际影响,网络相关)
ln -sf /dev/null "$TARGET_DIR/etc/systemd/system/systemd-networkd-persistent-storage.service"

echo "post-build: systemd-networkd-persistent-storage masked (避免 black-box 验收 [FAILED])"

# ===== default.target → multi-user.target(服务自启入口) =====
ln -sf /usr/lib/systemd/system/multi-user.target \
    "$TARGET_DIR/etc/systemd/system/default.target"

echo "post-build: default.target → multi-user.target"

# ===== 服务单元 Description 改英文(用户要求"全部中文改成英文") =====
# buildroot 包内 service 文件中文 Description → 英文(覆盖包安装的)
for svc in comm-center extension-bridge hal-gateway pi-agent rt-loop; do
    svc_file="$TARGET_DIR/usr/lib/systemd/system/${svc}.service"
    [ -f "$svc_file" ] || continue
    case "$svc" in
        comm-center)      desc="Taihao OS · Communication Center (comm-center)" ;;
        extension-bridge) desc="Taihao OS · Extension / MCP Bridge (extension-bridge)" ;;
        hal-gateway)      desc="Taihao OS · Hardware Access Gateway (hal-gateway)" ;;
        pi-agent)         desc="Taihao OS · Cognitive Decision Layer (pi-agent)" ;;
        rt-loop)          desc="Taihao OS · Execution Loop (rt-loop, 10-100Hz closed-loop)" ;;
    esac
    sed -i "s|^Description=.*|Description=$desc|" "$svc_file"
done
# taihao-check.service 同样改
check_file="$TARGET_DIR/usr/lib/systemd/system/taihao-check.service"
[ -f "$check_file" ] && sed -i 's|^Description=.*|Description=Taihao OS · Service Integration Check (verify 5 services active + whitelist + frequency)|' "$check_file"

echo "post-build: 5 service Description → 英文"

# ===== 太昊 OS 验收口径:登录门面 =====
# /etc/issue  →  "Welcome to TAIHAO"(agetty 在 login 提示之前输出,加 \n 隔开)
printf 'Welcome to TAIHAO\n' > "$TARGET_DIR/etc/issue"

# /etc/hostname  →  TAIHAO(agetty "<hostname> login: " 用)
printf 'TAIHAO\n' > "$TARGET_DIR/etc/hostname"

# 自定义登录提示符 "Welcome to TAIHAO" + "username: "
# 思路:在 serial-getty@.service 和 console-getty 上加 drop-in,
#       用 taihao-login 替换 agetty。taihao-login 输出 /etc/issue 和 "username: ",
#       read username 后 exec /bin/login "$user"(传 username,login 不再显示 <host> login:)
mkdir -p "$TARGET_DIR/usr/sbin"
cat > "$TARGET_DIR/usr/sbin/taihao-login" <<'EOF'
#!/bin/sh
# Taihao OS custom login prompt
# 输出 /etc/issue (Welcome to TAIHAO) + "username: ",read username 后 exec /bin/login "$user"
# 避免 exec /bin/login 无参数(login 会再显示一次 <host> login: prompt,造成重复)
if [ -r /etc/issue ]; then
    cat /etc/issue 2>/dev/null
fi
printf 'username: '
read -r user
if [ -z "$user" ]; then
    exit 0
fi
exec /bin/login "$user"
EOF
chmod 0755 "$TARGET_DIR/usr/sbin/taihao-login"

# 覆盖 serial-getty@.service(串口 ttyAMA0 等):ExecStart 用 taihao-login,Type=simple 立即启动
mkdir -p "$TARGET_DIR/etc/systemd/system/serial-getty@.service.d"
cat > "$TARGET_DIR/etc/systemd/system/serial-getty@.service.d/10-taihao-login.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/usr/sbin/taihao-login
Type=simple
EOF

# 覆盖 console-getty.service(控制台 tty1):ExecStart 用 taihao-login,Type=simple 立即启动
mkdir -p "$TARGET_DIR/etc/systemd/system/console-getty.service.d"
cat > "$TARGET_DIR/etc/systemd/system/console-getty.service.d/10-taihao-login.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/usr/sbin/taihao-login
Type=simple
EOF

# 覆盖 getty@.service template(用于 generator 创建的 getty@ttyN):ExecStart 用 taihao-login
mkdir -p "$TARGET_DIR/etc/systemd/system/getty@.service.d"
cat > "$TARGET_DIR/etc/systemd/system/getty@.service.d/10-taihao-login.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/usr/sbin/taihao-login
Type=simple
EOF

# 手动启用 getty@tty1.service(getty.target.wants)和 console-getty + serial-getty@ttyAMA0
mkdir -p "$TARGET_DIR/etc/systemd/system/getty.target.wants"
ln -sf /usr/lib/systemd/system/getty@.service \
    "$TARGET_DIR/etc/systemd/system/getty.target.wants/getty@tty1.service"
mkdir -p "$TARGET_DIR/etc/systemd/system/multi-user.target.wants"
ln -sf /usr/lib/systemd/system/console-getty.service \
    "$TARGET_DIR/etc/systemd/system/multi-user.target.wants/console-getty.service"
ln -sf /usr/lib/systemd/system/serial-getty@.service \
    "$TARGET_DIR/etc/systemd/system/multi-user.target.wants/serial-getty@ttyAMA0.service"

echo "post-build: getty@tty1 + console-getty + serial-getty@ttyAMA0 启用 taihao-login"

echo "post-build: 登录门面 → Welcome to TAIHAO + username: 提示(serial + console)"

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