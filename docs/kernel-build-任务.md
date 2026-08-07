# 任务:内核构建脚本体系 + QEMU 启动封装(构建集成批次)

> 执行对象:AI / 人工。本任务产出太昊 OS 仿真的构建 / 仿真工程化脚本,落地到 `scripts/kernel-build/`,按标准化生产流程与工程目录规格交付。

## 依据(唯一契约,不引用仓库外文件)

- `<仓库根>/docs/linux-内核裁剪方案.md`(v0.0.7 定稿)
- 重点章节:§2.3 迭代速度指标(冷构建 4~8 核区间 3~5 min / 增量 < 30 s / qemu 重启 < 2 s)、§2.4 配置分离、§4.3 savevm 快照、§6 关键 CONFIG
- 已交付基线:`config/kernel/qemu-aarch64-{ci,e2e}.config` + `scripts/kconfig/merge_config.sh`

## 二、工程化目录结构(交付规格)

```
scripts/kernel-build/
├── build-kernel.sh          # 主构建入口(合并片段 → 编译 → 产出)
├── qemu-start-ci.sh         # CI 冒烟启动(无 9p,自动冒烟校验)
├── qemu-start-e2e.sh        # 端到端启动(9p 共享,快照封装)
└── lib/
    ├── common.sh            # 日志 / 错误处理 / 公共函数
    ├── ccache.sh            # ccache 统一配置
    ├── snapshot.sh          # save-snap / load-snap 封装
    └── probe.sh             # 启动存活 / 指标探测
```

## 三、交付物规格

### 3.1 build-kernel.sh

| 能力 | 规格 |
| --- | --- |
| 内核源码管理 | 拉取 Linux 6.6 LTS 固定提交(锁定 commit hash,杜绝上游变动扰动) |
| 配置合并 | `merge_config.sh` 合并 `defconfig` + `qemu-aarch64-{ci,e2e}.config` |
| 模块策略 | CI 时 `CONFIG_MODULES=n`;E2E 时 `CONFIG_MODULES=y` |
| 产出 | `out/arm64-{ci,e2e}/Image` + 最小 `initramfs`(内置最小根) |
| 缓存 | ccache 统一缓存目录;保留冷 / 增量构建命中率 |
| 指标 | 构建秒级计时,冷构建区间 3~5 min(4~8 核,硬上限 10 min)/ 增量 < 30 s;超上限或失败即非零退出 |

### 3.2 qemu-start-ci.sh

- CI 冒烟:不挂 `virtio-9p` / `virtio-fs`(契约红线 4)
- 启动后自动执行冒烟校验脚本(等待 systemd `basic.target` → 断言)
- 无快照依赖,直接冷启动;退出码 = 校验结果

### 3.3 qemu-start-e2e.sh

- 开发调试:挂 `virtio-9p` 共享主机 rootfs 上层,支持热替换
- 内置 `save-snap` / `load-snap` 简易命令封装(qemu monitor savevm / loadvm)
- 内核镜像 / initramfs 变更时自动触发快照失效(校验 hash,失效即丢弃)

### 3.4 lib/*

- `common.sh`:日志分级、`set -euo pipefail`、公共函数
- `ccache.sh`:ccache 初始化与缓存目录统一
- `snapshot.sh`:save-snap / load-snap 实现 + 失效判定
- `probe.sh`:启动探测与存活检查

## 四、验收门槛(硬性)

1. **可执行**:`build-kernel.sh ci|e2e` 参数校验齐全,错误输出到 stderr,非零退出
2. **构建达标**:冷构建区间 3~5 min(4~8 核,硬上限 10 min)、增量 < 30 s(实测,ccache 命中);冷构建按核数区间计,不再以 < 3 min 为阻断项
3. **initramfs**:最小化,`busybox` + `systemd`(作为 pid 1),可进入 `basic.target`
4. **CI 冒烟**:`qemu-start-ci.sh` 启动 → systemd `basic.target` 达成,脚本退出码 = 校验结果
5. **E2E 调试**:9p 挂载 + 快照 save / load 循环可用;内核 / initramfs 更换后旧快照自动失效
6. **防御**:无 `rm -rf` 危险操作、路径含空格安全、临时目录避免污染产物树
7. **文档**:脚本头部用法注释,中文优先

## 五、自检(交付前全部通过)

- `bash -n` 全部脚本无语法错误
- `shellcheck`(关键脚本)无错误
- `build-kernel.sh ci` → Image 存在且为 ELF aarch64
- `build-kernel.sh e2e` → Image 存在,模块按 `=y` 并入
- 冷 / 增量构建时长记录输出,验证指标达标

## 六、交付清单

- 7 个脚本(3 主入口 + 4 库)
- 用法说明(合并命令、参数、环境变量)
- 构建 / 启动实测日志(含时长)

## 七、里程碑与顺序

1. 构建脚本 → 编译通过 → Image 产出
2. 最小 initramfs 内置根
3. CI 启动与冒烟校验
4. E2E 启动 + 9p 共享 + 快照 save / load
5. 快照失效检测
6. 指标验证收尾

## 八、预期耗时

- 理想路径(一次通过):1.5 ~ 2 h
- 常规路径(initramfs 调试往返):3 ~ 5 h
- 交付标记:`build-kernel.sh ci` + `qemu-start-ci.sh` 双绿灯 + 指标实测日志

> 注:本批次只做仿真 / 开发工具链;真机 RK3588 接入为后续批次,不在本任务范围。