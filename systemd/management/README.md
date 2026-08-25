# 太昊 OS · management

终端系统管理 web 项目(占位骨架,后续填业务)。

## 启动

```bash
npm install
npm run dev        # 开发(ts-node)
npm run build      # 编译到 dist/
npm start          # 生产(运行 dist/index.js)
```

## 端口

默认 `80`(特权端口,需 root 启动)。可通过 `PORT` 环境变量覆盖:

```bash
PORT=8080 npm start
```

## 当前 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/health` | 健康检查 |
| GET | `/api/system/info` | 系统信息(OS / Node 版本 / 内存 / uptime) |

## 后续补

- 服务列表 / 启停 / 重启
- 日志查看
- 设备状态
- OTA 触发
- 边界:鉴权 / 隔离(暂不在骨架内)
