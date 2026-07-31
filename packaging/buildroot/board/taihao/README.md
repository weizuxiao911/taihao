# 太昊 OS — Board 配置

## 目标硬件

- **SoC**：RK3588 同级别（Cortex-A76/A55 八核 + NPU INT8 ≥ 6TOPS）
- **RAM**：4GB LPDDR4X
- **存储**：16GB eMMC
- **总线**：UART / I2C / SPI / PWM / CAN
- **网络**：Wi-Fi 5/6 (802.11s Mesh) + 外接水声通信机串口 + 可选 4G/5G

## 三分区布局

```
+------------------+ 0x0           <- ptable
|  MBR / GPT       |
+------------------+
|  系统分区 (ro)   | ~2GB         <- squashfs
|  Linux 6.6 LTS   |
|  Pi Agent        |
|  HAL gateway     |
|  systemd         |
+------------------+
|  配置分区 (rw)   | ~256MB       <- ext4
|  /etc/taihao-os/ |
+------------------+
|  业务分区 (rw)   | ~12GB        <- ext4
|  /var/lib/taihao-os/ |
|  厂商 SKILL/HAL  |
|  业务数据        |
+------------------+
```

## 引导

- **Bootloader**：U-Boot（aarch64）
- **内核 cmdline**：`console=ttyS2,115200 root=/dev/mmcblk0p3 ro rootfstype=squashfs apparmor=1 security=apparmor`
- **A/B 升级**：系统分区双备份（p1 当前 / p2 备用），OTA 写到备用 + 切 slot

## 仿真

- **QEMU aarch64**：`qemu-system-aarch64 -M virt -cpu cortex-a76 -m 4G -kernel ... -append "root=/dev/vda ..."`
- **QEMU amd64**（开发用）：bios + rootfs 镜像
