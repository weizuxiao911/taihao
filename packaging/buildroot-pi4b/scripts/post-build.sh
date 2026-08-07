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

echo "post-build: systemd 配置就绪"

# ===== default.target → multi-user.target(服务自启入口) =====
ln -sf /usr/lib/systemd/system/multi-user.target \
    "$TARGET_DIR/etc/systemd/system/default.target"

echo "post-build: default.target → multi-user.target"