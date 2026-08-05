#!/usr/bin/env node
// 太昊 OS 岸基控制侧演示台 — 与 QEMU 内 comm-center 双向通信
//
// 角色: 模拟岸基 / 云控制侧 (中台调度通信模块的对接对象)
// 协议:
//   GET  /events/:prefix   ← comm-center 轮询拉取任务 (topic\npayload)
//   POST /report/:topic    ← comm-center 设备上报
// 前端:
//   GET  /                 控制台页面
//   GET  /live             SSE 实时推送 (设备心跳 / 上报 / 任务)
//   GET  /api/state        当前状态快照
//   POST /api/task         下发任务 {topic, payload}
//
// 零依赖 (纯 Node 内置模块), 启动: node server.js [PORT=19090]

const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = Number(process.env.PORT || 19090);
const PUBLIC_DIR = path.join(__dirname, 'public');

// ===== 状态 =====
const state = {
  taskQueue: [],        // 待下发任务
  delivered: [],        // 已下发任务
  reports: [],          // 设备上报 (最多保留 200 条)
  heartbeats: 0,        // 设备心跳计数 (comm-center 轮询次数)
  lastHeartbeatAt: null,
  startedAt: Date.now(),
};
const sseClients = new Set();

function broadcast(payload) {
  const data = `data: ${JSON.stringify(payload)}\n\n`;
  for (const res of sseClients) res.write(data);
}

function snapshot() {
  const lastStatus = [...state.reports].reverse().find((r) => r.topic === 'status/device') || null;
  return {
    taskQueue: state.taskQueue,
    delivered: state.delivered.slice(-20),
    reports: state.reports.slice(-100),
    heartbeats: state.heartbeats,
    lastHeartbeatAt: state.lastHeartbeatAt,
    lastStatus,
    uptimeSecs: Math.floor((Date.now() - state.startedAt) / 1000),
    deviceOnline: (state.lastHeartbeatAt && Date.now() - state.lastHeartbeatAt < 30000) || (lastStatus && Date.now() - new Date(lastStatus.at).getTime() < 30000),
  };
}

// ===== HTTP 服务 =====
const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);

  // --- 前端页面 ---
  if (req.method === 'GET' && (url.pathname === '/' || url.pathname === '/index.html')) {
    return sendFile(res, path.join(PUBLIC_DIR, 'index.html'));
  }

  // --- 前端实时推送 (SSE) ---
  if (req.method === 'GET' && url.pathname === '/live') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    });
    res.write(`data: ${JSON.stringify(snapshot())}\n\n`);
    sseClients.add(res);
    req.on('close', () => sseClients.delete(res));
    return;
  }

  // --- comm-center 轮询拉取任务: GET /events/:prefix ---
  if (req.method === 'GET' && url.pathname.startsWith('/events/')) {
    state.heartbeats += 1;
    state.lastHeartbeatAt = Date.now();
    const task = state.taskQueue.shift();
    const body = task ? `${task.topic}\n${task.payload}` : '';
    if (task) {
      state.delivered.push({ at: new Date().toISOString(), ...task });
      broadcast({ type: 'delivered', task });
    }
    broadcast({ type: 'heartbeat', at: state.lastHeartbeatAt, total: state.heartbeats });
    res.writeHead(200, { 'Content-Type': 'text/plain', 'Content-Length': Buffer.byteLength(body) });
    return res.end(body);
  }

  // --- 设备上报: POST /report/:topic ---
  if (req.method === 'POST' && url.pathname.startsWith('/report/')) {
    const topic = decodeURIComponent(url.pathname.slice('/report/'.length));
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      const report = { at: new Date().toISOString(), topic, payload: body };
      state.reports.push(report);
      if (state.reports.length > 200) state.reports.shift();
      broadcast({ type: 'report', report });
      res.writeHead(200, { 'Content-Type': 'text/plain', 'Content-Length': 2 });
      res.end('ok');
    });
    return;
  }

  // --- 前端 API ---
  if (req.method === 'POST' && url.pathname === '/api/task') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      try {
        const task = JSON.parse(body);
        if (!task.topic || task.payload === undefined) throw new Error('topic/payload 必填');
        state.taskQueue.push(task);
        broadcast({ type: 'queued', task });
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true, queue: state.taskQueue.length }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: false, error: String(e.message || e) }));
      }
    });
    return;
  }

  if (req.method === 'GET' && url.pathname === '/api/state') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(snapshot()));
  }

  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('404');
});

function sendFile(res, file) {
  fs.readFile(file, (err, data) => {
    if (err) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      return res.end('404');
    }
    const ext = path.extname(file);
    const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css' };
    res.writeHead(200, { 'Content-Type': types[ext] || 'text/plain' });
    res.end(data);
  });
}

server.listen(PORT, '0.0.0.0', () => {
  console.log(`太昊 OS 岸基控制侧演示台: http://localhost:${PORT}`);
  console.log(`comm-center 配置 remote_endpoint = http://<宿主IP>:${PORT} (QEMU 内为 http://10.0.2.2:${PORT})`);
});
