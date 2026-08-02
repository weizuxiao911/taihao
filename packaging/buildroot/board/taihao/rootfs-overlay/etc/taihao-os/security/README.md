# 太昊 OS 四层隔离栈模板
# 部署路径：/etc/taihao-os/security/
# 生效载体：systemd unit 沙箱参数（NoNewPrivileges / CapabilityBoundingSet /
#           PrivateDevices / RestrictAddressFamilies 等）+ 本目录配置模板

# ===== 层 1: iptables 网络隔离 =====
# 默认拒绝设备出网，仅放行白名单（中台调度通信端点 / Mesh 组播）
# 模板（按厂商网络规划填充）:
#   iptables -P OUTPUT DROP
#   iptables -A OUTPUT -o lo -j ACCEPT
#   iptables -A OUTPUT -p udp --dport 41800 -j ACCEPT        # 本地 Mesh
#   iptables -A OUTPUT -p tcp -d <comm_endpoint> -j ACCEPT   # 远程通信
#   iptables -A OUTPUT -p udp --dport 53 -j ACCEPT           # DNS（如需）

# ===== 层 2: namespaces 进程隔离 =====
# 由 systemd unit 承担（各服务见 /usr/lib/systemd/system/*.service）:
#   PrivateTmp / PrivateDevices / ProtectHome / ProtectSystem=strict
# 进程视图互相隔离，运行时数据只落在 /var/lib/taihao-os/<模块>

# ===== 层 3: seccomp + eBPF 系统调用过滤 =====
# systemd SystemCallFilter 覆盖大部分场景；需要更细粒度时按模块追加:
#   SystemCallFilter=@system-service
#   SystemCallErrorNumber=EPERM
# eBPF 探针（真机进阶）: 由厂商业务分区按需注入，不随系统底座变更

# ===== 层 4: AppArmor 强制访问控制 =====
# 各模块 profile 模板见 apparmor/ 子目录；加载方式:
#   apparmor_parser -r /etc/apparmor.d/taihao-*
