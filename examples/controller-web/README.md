# 太昊 OS 岸基控制侧演示台 (controller-web)

> 与 QEMU 内太昊 OS 双向通信的 Web 演示工具。模拟「岸基 / 云」控制侧,
> 对接设备端中台调度通信模块 (comm-center) 的 SSE 任务下发与 HTTP 上报。

## 运行

```bash
node server.js            # 默认端口 19090
# PORT=8080 node server.js
```

浏览器打开 `http://localhost:19090`。

## 与 QEMU 内设备通信

1. 启动演示台 (宿主): `node server.js`
2. 启动 QEMU (镜像已配置 comm-center 指向 `http://10.0.2.2:19090`):

```bash
qemu-system-aarch64 -M virt -cpu cortex-a76 -m 1G \
  -kernel Image -drive file=rootfs.ext2,format=raw,if=virtio \
  -append "root=/dev/vda rw console=ttyAMA0 ip=10.0.2.15::10.0.2.2:255.255.255.0::eth0:none" \
  -nographic
```

3. 等待演示台显示「设备在线」; 在页面上下发任务 → 设备 comm-center 拉取执行并上报 → 页面实时展示。

## 协议 (与 comm-center 对接)

| 端点 | 方向 | 说明 |
|---|---|---|
| `GET /events/:prefix` | 设备 → 控制侧 | comm-center 轮询拉取任务, 返回 `topic\npayload`; 空 = 无任务 |
| `POST /report/:topic` | 设备 → 控制侧 | 设备上报 (路由 builtin:report 回传) |
| `GET /api/state` | 前端 | 状态快照 (心跳/上报/队列) |
| `POST /api/task` | 前端 | 下发任务 `{topic, payload}` |
| `GET /live` | 前端 | SSE 实时推送 |

## 零依赖

纯 Node.js 内置模块 (`http`/`fs`), 无 npm 依赖, 无 `node_modules`。
