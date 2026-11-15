#!/usr/bin/env node
// P1 state player — ป้อน HudState ให้ ui/hud โดยไม่ต้องมีแอปจริง
// รัน: node tools/state-player/serve.mjs  → http://127.0.0.1:4173/ui/hud/
// N = state ถัดไป · POST /api/state = ตั้ง state เอง

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

const ROOT = path.resolve(process.cwd());
const SAMPLES = path.join(
  ROOT,
  'project menager',
  'Design',
  '03-engineering',
  'contracts',
  'hud-state.samples.json',
);
const PORT = Number(process.env.PORT ?? 4173);

const states = JSON.parse(fs.readFileSync(SAMPLES, 'utf8'));
let index = 0;
let current = states[index];

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
};

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://${req.headers.host}`);

  if (url.pathname === '/api/state') {
    if (req.method === 'POST') {
      const body = await readBody(req);
      try {
        current = JSON.parse(body);
        json(res, current);
      } catch {
        json(res, { error: 'invalid json' }, 400);
      }
      return;
    }
    json(res, current);
    return;
  }

  if (url.pathname === '/api/next' && req.method === 'POST') {
    index = (index + 1) % states.length;
    current = states[index];
    json(res, current);
    return;
  }

  if (url.pathname === '/api/list') {
    json(res, states.map((s, i) => ({ i, mode: s.mode, state: s.state })));
    return;
  }

  if (url.pathname === '/') {
    res.writeHead(302, { location: '/ui/hud/?preview=1' });
    res.end();
    return;
  }

  serveStatic(url.pathname, res);
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`state player: http://127.0.0.1:${PORT}/ui/hud/`);
  console.log(`samples: ${states.length} · N = ถัดไป · Ctrl+C = ออก`);
});

function serveStatic(pathname, res) {
  let file = path.resolve(ROOT, `.${decodeURIComponent(pathname)}`);
  if (!file.startsWith(ROOT)) return notFound(res);
  if (pathname.endsWith('/')) file = path.join(file, 'index.html');

  fs.readFile(file, (err, data) => {
    if (err) return notFound(res);
    res.writeHead(200, {
      'content-type': MIME[path.extname(file)] ?? 'application/octet-stream',
      'cache-control': 'no-store',
    });
    res.end(data);
  });
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => {
      body += chunk;
      if (body.length > 1_000_000) req.destroy();
    });
    req.on('end', () => resolve(body));
    req.on('error', reject);
  });
}

function json(res, value, status = 200) {
  res.writeHead(status, { 'content-type': 'application/json; charset=utf-8' });
  res.end(JSON.stringify(value));
}

function notFound(res) {
  res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
  res.end('not found');
}
