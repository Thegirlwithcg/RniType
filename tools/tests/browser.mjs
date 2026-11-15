// ตัวช่วยเปิด Chrome headless ผ่าน CDP (ใช้ร่วมกับ test ที่ต้องวัดผลจริง)
// ไม่ใช้ dependency — ต้องมี Chrome/Chromium (ตั้ง CHROME_PATH ได้)

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const CHROME =
  process.env.CHROME_PATH ||
  'C:\\Users\\User\\AppData\\Local\\ms-playwright\\chromium-1208\\chrome-win64\\chrome.exe';

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export async function launch({
  url,
  width = 1440,
  height = 900,
  scaleFactor = 1,
  pageMatch = null,
}) {
  const profile = mkdtempSync(path.join(tmpdir(), 'rni-cdp-'));
  const args = [
    '--headless=new',
    '--disable-gpu',
    '--remote-debugging-port=0',
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-timer-throttling',
    '--disable-renderer-backgrounding',
    '--disable-backgrounding-occluded-windows',
    '--allow-file-access-from-files',
    `--window-size=${width},${height}`,
  ];
  if (scaleFactor !== 1) args.push(`--force-device-scale-factor=${scaleFactor}`);
  args.push(url);

  const chrome = spawn(CHROME, args, { stdio: ['ignore', 'pipe', 'pipe'] });

  let stderr = '';
  const wsUrl = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`ไม่พบ DevTools endpoint\n${stderr}`)), 20000);
    chrome.stderr.on('data', (data) => {
      stderr += data.toString();
      const match = stderr.match(/DevTools listening on (ws:\/\/\S+)/);
      if (match) {
        clearTimeout(timer);
        resolve(match[1]);
      }
    });
    chrome.on('exit', (code) => reject(new Error(`chrome ออกก่อนกำหนด (${code})\n${stderr}`)));
  });

  const port = new URL(wsUrl).port;
  let target = null;
  for (let i = 0; i < 80 && !target; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const pages = list.filter((t) => t.type === 'page' && /^(https?|file):/.test(t.url));
      target = pageMatch ? pages.find((t) => t.url.includes(pageMatch)) : pages[0];
    } catch {
      /* chrome ยังไม่พร้อม */
    }
    if (!target) await sleep(150);
  }
  if (!target) {
    chrome.kill();
    throw new Error(`ไม่พบหน้า ${pageMatch ?? url} ใน Chrome`);
  }

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = () => reject(new Error('ต่อ CDP ไม่ได้'));
  });

  let id = 0;
  const pending = new Map();
  const exceptions = [];

  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message.result);
    } else if (message.method === 'Runtime.exceptionThrown') {
      exceptions.push(message.params.exceptionDetails.text);
    }
  };

  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const messageId = ++id;
      pending.set(messageId, { resolve, reject });
      ws.send(JSON.stringify({ id: messageId, method, params }));
    });

  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (result.exceptionDetails) {
      throw new Error(
        result.exceptionDetails.text + ' ' + (result.exceptionDetails.exception?.description || ''),
      );
    }
    return result.result.value;
  };

  const screenshot = async (file, { fullPage = false } = {}) => {
    const params = { format: 'png' };
    if (fullPage) {
      const metrics = await send('Page.getLayoutMetrics');
      const size = metrics.cssContentSize ?? metrics.contentSize;
      params.captureBeyondViewport = true;
      params.clip = { x: 0, y: 0, width: size.width, height: size.height, scale: 1 };
    }
    const { data } = await send('Page.captureScreenshot', params);
    writeFileSync(file, Buffer.from(data, 'base64'));
    return file;
  };

  const close = () => {
    try {
      ws.close();
    } catch {
      /* ปิดไปแล้ว */
    }
    chrome.kill();
    try {
      rmSync(profile, { recursive: true, force: true });
    } catch {
      /* windows อาจยังล็อกอยู่ */
    }
  };

  await send('Runtime.enable');
  await send('Page.enable');

  return { send, evaluate, screenshot, close, exceptions };
}
