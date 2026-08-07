# 任务:Buildroot 基座打包 + 分区 / OTA 骨架(基座 OS 落地批次)

> 执行对象:AI(可自主执行,含工具链安装)。本任务把已验收的内核(契约)与服务层(src/ 五模块)打包为 Buildroot 基座 OS 镜像,落地固件分区与 A/B OTA 脚本骨架,并在已交付的 QEMU 链路上完成镜像启动与集成验收。

## 一、依据(唯一契约,不引用仓库外文件)

- `<仓库根>/docs/linux-内核裁剪方案.md`(基线契约):

| 章节 | 内容 | 本任务落点 |
| --- | --- | --- |
| §1 场景定位 | 基座 Buildroot;16GB eMMC 体积约束;RKNN / ONNX / llama.cpp / Mosquitto 可全编入;无 apt 依赖 | 镜像方案(§3) |
| §2.1 端到端内核 | 完整集成验证;全部服务 systemd unit `active` | 集成验收(§4.3) |
| §2.3 迭代速度硬指标 | 冷构建 4~8 核区间 3~5 min(硬上限 10 min)/ 增量 < 30 s / qemu 重启 < 2 s | 指标实测(§4.4) |
| §8 真机关系 | 真机 A/B 分区 bootloader 切换;仿真不承担真机加密 | 分区 / OTA 骨架(§3.4) |

- `<仓库根>/AGENTS.md`:
  - 技术选型:基座 OS = Buildroot;烧录 = dd(工厂)+ OTA(部署后);A/B 双系统分区 + 失败回滚
  - 目录职责:`packaging/buildroot/`(overlays / configs / package recipes)、`packaging/output/` 不入库
  - 运行时路径:`/etc/taihao-os/`、`/usr/share/taihao-os/skills/`、`/var/lib/taihao-os/`、`/var/log/taihao-os/`
  - 技术路径:QEMU 系虚拟化(Lima 构建 + 宿主 QEMU 启动),不用 Docker / 容器化

- 已交付基线:`config/kernel/` + `scripts/kernel-build/`(构建 / 仿真全链路)+ `src/`(五模块 + systemd unit + SKILL)+ `scripts/services/install.sh`

## 二、前置条件

| 项 | 要求 | 说明 |
| --- | --- | --- |
| Linux VM | Lima ARM64(`taihao-build`) | Buildroot 构建环境 |
| 工具链 | Buildroot 依赖(host gcc / make / etc.) | VM 内安装 |
| 内核产物 | `out/arm64-e2e/Image` | 契约交付态 |
| 服务层 | `src/` 五模块交叉编译产物(aarch64) | 服务层批次交付态 |
| QEMU | `qemu-system-aarch64` | 宿主启动验证 |

## 三、交付规格

### 3.1 Buildroot 骨架(`packaging/buildroot/`)

| 文件 / 目录 | 规格 |
| --- | --- |
| `Config.in` | 自研包注册(pi-agent / hal-gateway / rt-loop / comm-center / extension-bridge) |
| `taihao_defconfig` | arm64 板型;基座包(系统库 / systemd / busybox);体积预算对齐 16GB eMMC |
| `overlays/rootfs/` | 运行时目录结构、SKILL、配置模板(对齐 AGENTS 运行时路径) |
| `package/*/` | 5 个自研包 recipe(cargo 交叉编译 aarch64,版本 / hash 固定) |

### 3.2 镜像产出

- Buildroot 输出 `packaging/output/`(不入库,`.gitignore` 已排除)
- 镜像结构:内核 `Image` + rootfs(ext4)打包;QEMU 可启动

### 3.3 服务层集成

- 5 个服务 binary + systemd unit 编入镜像,`multi-user.target.wants` 自启
- SKILL 集(系统级 `/usr/share/taihao-os/skills/`)随 overlay 部署
- `taihao` 系统用户与 group 建立(unit `User=taihao` 依赖)

### 3.4 分区 / A/B OTA 骨架(`scripts/partition/`)

| 脚本 | 规格 |
| --- | --- |
| `partition-layout.sh` | 三分区布局(boot / A / B + 数据分区),工厂 dd 烧录脚本 |
| `ota-build.sh` | A/B 镜像构建 + 差分 / 全量 OTA 包生成(骨架) |
| `ota-apply.sh` | 写入非活动槽 → 校验 → 切换活动槽;失败回滚(A/B 双系统) |

> 仿真层先落脚本骨架与格式;真机 bootloader 切换不在本批次(契约 §8 认知)。

## 四、验收(硬性)

### 4.1 Buildroot 构建

- `packaging/buildroot/` 结构完整;`taihao_defconfig` 可构建,退出码 0
- 镜像产物存在(`packaging/output/`),rootfs 含 systemd + busybox

### 4.2 服务集成

- 镜像内 `systemctl is-active` 五个服务全部 `active`(对齐契约 §2.1)
- `taihao` 用户存在;运行时目录结构完整

### 4.3 QEMU 启动验证

- QEMU 启动镜像 → systemd `basic.target` 达成
- 镜像 / initramfs 更换 → 快照失效自动重建(复用 kernel-build 链路)

### 4.4 指标实测

| 指标 | 目标 |
| --- | --- |
| Buildroot 镜像构建 | 记录实测值(冷 / 增量) |
| qemu 重启 | < 2 s(快照 load) |

### 4.5 分区 / OTA 骨架

- `partition-layout.sh` 三分区布局脚本可执行(生成布局文件 / 分区表描述)
- `ota-apply.sh` 具备槽位写入 → 校验 → 切换 → 失败回滚的逻辑骨架(脚本级验证,不落真机)

## 五、自检(交付前全部通过)

- `bash -n` 全部脚本 0 错误
- Buildroot 构建一次通过,镜像可启动
- 镜像内五服务 `is-active` 全绿
- OTA 脚本逻辑自检(槽位状态机单元级验证)
- 实测日志留存(构建 / 启动 / 服务验收)

## 六、交付清单

- `packaging/buildroot/`(Config.in / taihao_defconfig / overlays / 5 包 recipe)
- `scripts/partition/`(3 脚本)
- 构建 / 启动实测日志 + 服务验收记录

## 七、里程碑与顺序

1. Buildroot 骨架 + taihao_defconfig → 可构建
2. 5 自研包 recipe → 编入 rootfs
3. overlay + systemd unit + SKILL → 服务自启
4. 镜像产出 → QEMU 启动 → 五服务 active
5. 分区布局 + A/B OTA 脚本骨架
6. 指标实测 + 验收收尾

## 八、预期

- 理想路径(单次打通):2 ~ 4 h
- 常规路径(Buildroot 依赖 / 体积往返):4 ~ 8 h
- 完成标志:镜像 QEMU 启动五服务全绿 + 分区 / OTA 骨架交付 + 实测日志留存