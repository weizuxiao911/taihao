import express, { Request, Response } from 'express';
import path from 'path';

const app = express();
const PORT = parseInt(process.env.PORT || '80', 10);
const HOST = process.env.HOST || '0.0.0.0';

app.use(express.json());
app.use(express.static(path.join(__dirname, '../public')));

app.get('/api/health', (_req: Request, res: Response) => {
  res.json({
    status: 'ok',
    service: 'taihao-management',
    timestamp: new Date().toISOString(),
  });
});

app.get('/api/system/info', (_req: Request, res: Response) => {
  res.json({
    os: 'Taihao OS',
    version: '0.1.0',
    node: process.version,
    uptime_seconds: Math.round(process.uptime()),
    memory: process.memoryUsage(),
  });
});

app.get('/api/services', (_req: Request, res: Response) => {
  res.json({
    services: [
      { name: 'pi-agent', status: 'active', description: '智能体服务', uptime: 3600 },
      { name: 'hal-gateway', status: 'active', description: 'HAL 网关', uptime: 3600 },
      { name: 'rt-loop', status: 'active', description: '实时回路', uptime: 3600 },
      { name: 'comm-center', status: 'active', description: '调度通信中心', uptime: 3600 },
      { name: 'extension-bridge', status: 'active', description: '扩展桥', uptime: 3600 },
      { name: '联网服务', status: 'active', description: 'WiFi STA + 配网', uptime: 3600 },
      { name: '消息服务', status: 'active', description: '外部收发 + 内部分发', uptime: 3600 },
    ],
  });
});

app.get('/api/logs', (_req: Request, res: Response) => {
  res.json({
    logs: [
      { time: new Date().toISOString(), level: 'info', service: 'systemd', message: 'management web 服务已启动' },
      { time: new Date(Date.now() - 60000).toISOString(), level: 'info', service: 'pi-agent', message: 'agent loop 启动,频率 1Hz' },
      { time: new Date(Date.now() - 120000).toISOString(), level: 'info', service: 'hal-gateway', message: 'mock 驱动加载 done' },
      { time: new Date(Date.now() - 180000).toISOString(), level: 'warn', service: 'rt-loop', message: 'loop latency 偏高 23ms' },
      { time: new Date(Date.now() - 240000).toISOString(), level: 'info', service: 'comm-center', message: '消息通道 ready' },
      { time: new Date(Date.now() - 300000).toISOString(), level: 'info', service: 'extension-bridge', message: 'MCP 桥 initialize 成功' },
    ],
  });
});

app.use((_req: Request, res: Response) => {
  res.status(404).json({ error: 'not found' });
});

app.listen(PORT, HOST, () => {
  console.log(`[taihao-management] listening on http://${HOST}:${PORT}`);
});
