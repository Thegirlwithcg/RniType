// ตรวจ typography ของ HUD จริง (P4 slice 3) + เก็บ screenshot 100%/150% DPI
// รัน: node tools/tests/hud-typography.mjs
// ต้องมี Chrome/Chromium (ตั้ง CHROME_PATH ได้) · ใช้ state player เป็นตัวป้อน state
//
// ตรวจตาม HANDOFF §4.5 ข้อ 5:
//   1) ฟอนต์ไทยไม่มีหัวโหลดจริง (IBM Plex Sans Thai / Noto Sans Thai)
//   2) ขนาดขั้นต่ำ: group ≥ 13.5 · char ≥ 20 · sub/badge ≥ 10.5 · pill ≥ 13
//   3) ไม่มี label ชนกัน (คู่ที่ intersection > 2% ของอันที่เล็กกว่า)
//   4) contrast ของตัวอักษรจาง ≥ 4.5:1 (เทียบพื้น HUD เข้ม #12161e)
//   5) text scale เป็นตัวคูณแยกจากขนาดวง (1.2 → 1.6 = ตัวหนังสือใหญ่ขึ้นเท่านั้น)

import { spawn } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { launch, sleep } from './browser.mjs';

const PORT = Number(process.env.PORT ?? 4199);
const BASE = `http://127.0.0.1:${PORT}`;
const SAMPLES = path.join(
  path.resolve('project menager', 'Design', '03-engineering', 'contracts', 'hud-state.samples.json'),
);
const SHOTS = path.join(path.resolve('project menager', 'Design', '05-mockup', 'hud-shots'));

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` — ${detail}` : ''}`);
}

const samples = JSON.parse(readFileSync(SAMPLES, 'utf8'));
const indexOf = (mode, state) => samples.findIndex((s) => s.mode === mode && s.state === state);
const TYPE_TH = samples[indexOf('type-th', 'repeat')];
const TYPE_EN = samples[indexOf('type-en', 'char')];
const COMMAND = samples[indexOf('cmd', 'idle')];
const QUICKBAR = samples[indexOf('quickbar', 'idle')];
const CARET = samples[indexOf('caret', 'idle')];

async function setState(state) {
  const res = await fetch(`${BASE}/api/state`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(state),
  });
  if (!res.ok) throw new Error(`ตั้ง state ไม่ได้ (${res.status})`);
}

/** รอให้ HUD วาด state ที่ส่งไปแล้ว 1 รอบ (poll ทุก 100 ms) */
const settle = () => sleep(450);

mkdirSync(SHOTS, { recursive: true });

const server = spawn(process.execPath, [path.join('tools', 'state-player', 'serve.mjs')], {
  env: { ...process.env, PORT },
  stdio: ['ignore', 'pipe', 'pipe'],
});

let serverLog = '';
server.stdout.on('data', (d) => (serverLog += d.toString()));
server.stderr.on('data', (d) => (serverLog += d.toString()));

let browser = null;
let hidpi = null;

try {
  for (let i = 0; i < 40; i++) {
    try {
      await fetch(`${BASE}/api/list`);
      break;
    } catch {
      await sleep(150);
    }
  }

  /* ---------------- 100% DPI ---------------- */
  browser = await launch({
    url: `${BASE}/ui/hud/?preview=1`,
    width: 1600,
    height: 1000,
  });

  await browser.evaluate('document.fonts.ready.then(() => true)');

  // โหลดฟอนต์จริง (document.fonts.check อย่างเดียวไม่ trigger การโหลด)
  await browser.evaluate(`Promise.all([
    document.fonts.load('600 14px "IBM Plex Sans Thai"'),
    document.fonts.load('500 14px "Noto Sans Thai"'),
  ]).then(() => true)`);

  const fonts = await browser.evaluate(`({
    plex: document.fonts.check('600 14px "IBM Plex Sans Thai"'),
    noto: document.fonts.check('500 14px "Noto Sans Thai"'),
    stacks: [...document.fonts].map(f => f.family + ':' + f.weight).sort()
  })`);
  check('IBM Plex Sans Thai โหลดจาก ui/shared/fonts', fonts.plex === true, fonts.stacks.join(' '));
  check('Noto Sans Thai (fallback) โหลดด้วย', fonts.noto === true);

  /* ---- state 1: วงกลุ่ม + ตัวอักษร (ไทย) ---- */
  await setState(TYPE_TH);
  await settle();

  const rings = await browser.evaluate(`(() => {
    const px = (el) => parseFloat(getComputedStyle(el).fontSize);
    const first = (sel) => document.querySelector(sel);
    const fill = (sel) => getComputedStyle(first(sel)).fill;
    // ink box = กล่องหมึกจริง (getBBox ของ <text> ใช้ em box → ชนกันหลอกได้)
    const inkBox = (el) => {
      const size = parseFloat(getComputedStyle(el).fontSize);
      const x = parseFloat(el.getAttribute('x')) || 0;
      const y = parseFloat(el.getAttribute('y')) || 0;
      const len = el.getComputedTextLength();
      const lines = el.querySelectorAll('tspan').length || 1;
      const h = size * 0.78 + (lines > 1 ? size * 1.1 : 0);
      return { x: x - len / 2, y: y - h / 2, width: len, height: h };
    };
    return {
      mode: document.getElementById('hud').dataset.mode,
      rings: document.getElementById('hud').dataset.rings,
      group: px(first('.sector-label--group:not(.len-m):not(.len-l)')),
      groupLabels: [...document.querySelectorAll('.sector-label--group')].map(el => ({
        text: el.textContent, size: px(el), lines: el.querySelectorAll('tspan').length || 1, box: inkBox(el),
      })),
      char: [...document.querySelectorAll('.char-text')].map(el => ({
        text: el.textContent, size: px(el), cls: el.getAttribute('class'), box: inkBox(el),
      })),
      guides: document.querySelectorAll('.char-guide.is-on').length,
      sub: first('.slot-sub') ? px(first('.slot-sub')) : null,
      badge: first('.slot-badge') ? px(first('.slot-badge')) : null,
      pill: px(first('.chip--status')),
      dimFill: fill('.char-text.is-dim'),
      family: getComputedStyle(document.body).fontFamily,
      ringScale: parseFloat(getComputedStyle(document.getElementById('hud')).getPropertyValue('--hud-scale')),
    };
  })()`);

  check('HUD วาดวงไทย (mode/rings)', rings.mode === 'type-th' && rings.rings === 'on', `${rings.mode}/${rings.rings}`);
  check('ฟอนต์ของ HUD = IBM Plex Sans Thai', /IBM Plex Sans Thai/.test(rings.family), rings.family);

  check('group label ≥ 15px', rings.group >= 15, `${rings.group.toFixed(2)}px`);
  check('pill ≥ 13.5px', rings.pill >= 13.5, `${rings.pill.toFixed(2)}px`);
  if (rings.sub !== null) check('slot-sub ≥ 11.5px', rings.sub >= 11.5, `${rings.sub.toFixed(2)}px`);
  if (rings.badge !== null) check('slot-badge ≥ 11.5px', rings.badge >= 11.5, `${rings.badge.toFixed(2)}px`);

  const charMin = Math.min(...rings.char.map((c) => c.size));
  const charMax = Math.max(...rings.char.map((c) => c.size));
  check(
    'char 22–32px (ทุกช่อง)',
    charMin >= 22 && charMax <= 32,
    `${charMin.toFixed(2)}–${charMax.toFixed(2)}px`,
  );

  // ขนาดขั้นต่ำของ label กลุ่มทุกตัว (รวม .len-m/.len-l)
  const groupMin = Math.min(...rings.groupLabels.map((g) => g.size));
  check('group label ที่ยาวที่สุด ≥ 12px', groupMin >= 12, `${groupMin.toFixed(2)}px`);

  // ป้ายไทยยาว (วรรณยุกต์) ต้องแบ่ง 2 บรรทัด
  const twoLine = rings.groupLabels.filter((g) => g.lines === 2).map((g) => g.text);
  check('ป้าย "วรรณยุกต์" ใช้ 2 บรรทัด', twoLine.includes('วรรณยุกต์'), twoLine.join(', '));

  // ขีดกำกับสระล่าง (ุ ู) แทนคำว่า "ขีดล่าง"
  check('มีขีดกำกับสระล่าง 2 เส้น (ุ ู)', rings.guides === 2, `${rings.guides} เส้น`);

  /* ---- ไม่มี label ชนกัน ---- */
  const collisions = (items) => {
    const pairs = [];
    for (let i = 0; i < items.length; i++) {
      for (let j = i + 1; j < items.length; j++) {
        const a = items[i].box;
        const b = items[j].box;
        const w = Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x);
        const h = Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y);
        if (w <= 0 || h <= 0) continue;
        const overlap = w * h;
        const minArea = Math.min(a.width * a.height, b.width * b.height);
        if (overlap > minArea * 0.02) {
          pairs.push(`${items[i].text}/${items[j].text} ${(overlap / minArea * 100).toFixed(0)}%`);
        }
      }
    }
    return pairs;
  };

  const groupCollisions = collisions(rings.groupLabels);
  check('label กลุ่มไม่ชนกัน', groupCollisions.length === 0, groupCollisions.join(', '));

  const charCollisions = collisions(rings.char);
  check('ตัวอักษรในวงนอกไม่ชนกัน', charCollisions.length === 0, charCollisions.join(', '));

  /* ---- emphasis ของช่องที่ active (ADR-19: ไม่ใช้ transform — วงไม่เพี้ยน) ---- */
  const emphasis = await browser.evaluate(`(() => {
    const sec = document.querySelector('.sector--char.is-selected');
    const dim = document.querySelector('.char-text.is-dim');
    const sel = document.querySelector('.char-text.is-selected');
    const group = document.querySelector('.sector--group.is-armed');
    const normal = document.querySelector('.sector--char:not(.is-selected)');
    const px = (el) => (el ? parseFloat(getComputedStyle(el).fontSize) : 0);
    const num = (el, prop) => (el ? parseFloat(getComputedStyle(el)[prop]) || 0 : 0);
    return {
      selSize: px(sel),
      dimSize: px(dim),
      selStroke: num(sec, 'strokeWidth'),
      normalStroke: num(normal, 'strokeWidth'),
      groupStroke: num(group, 'strokeWidth'),
      transform: sec ? getComputedStyle(sec).transform : 'none',
    };
  })()`);
  check(
    'ตัวอักษรที่เลือกใหญ่กว่าช่องอื่น ≥ 1.15×',
    emphasis.dimSize > 0 && emphasis.selSize / emphasis.dimSize >= 1.15,
    `${emphasis.selSize.toFixed(1)} / ${emphasis.dimSize.toFixed(1)}px`,
  );
  check(
    'sector ที่เลือกเน้นด้วย stroke หนาขึ้น',
    emphasis.selStroke > emphasis.normalStroke,
    `${emphasis.selStroke} > ${emphasis.normalStroke}`,
  );
  check('กลุ่มที่เลือกมีสีเด่น (ไม่ scale)', emphasis.groupStroke > 0, `stroke ${emphasis.groupStroke}`);
  check(
    'ไม่มี transform บน sector (วงไม่เพี้ยน)',
    emphasis.transform === 'none',
    emphasis.transform,
  );

  /* ---- contrast ของตัวอักษรจาง ---- */
  const contrast = await browser.evaluate(`(() => {
    const toRgb = (value) => value.match(/\\d+(\\.\\d+)?/g).slice(0, 3).map(Number);
    const lum = ([r, g, b]) => {
      const ch = (v) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
      };
      return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
    };
    const ratio = (fg, bg) => {
      const a = lum(toRgb(fg));
      const b = lum(toRgb(bg));
      const [hi, lo] = a > b ? [a, b] : [b, a];
      return (hi + 0.05) / (lo + 0.05);
    };
    const fill = getComputedStyle(document.querySelector('.char-text.is-dim')).fill;
    return { fill, onGlass: ratio(fill, 'rgb(18,22,30)') };
  })()`);

  check(
    'contrast ตัวอักษรจาง ≥ 4.5:1',
    contrast.onGlass >= 4.5,
    `${contrast.fill} → ${contrast.onGlass.toFixed(2)}:1`,
  );

  await browser.screenshot(path.join(SHOTS, 'hud-typing-100dpi.png'));

  /* ---- state 2: วงคำสั่ง ---- */
  await setState(COMMAND);
  await settle();
  await browser.screenshot(path.join(SHOTS, 'hud-command-100dpi.png'));

  const cmd = await browser.evaluate(`(() => {
    const inkBox = (el) => {
      const size = parseFloat(getComputedStyle(el).fontSize);
      const x = parseFloat(el.getAttribute('x')) || 0;
      const y = parseFloat(el.getAttribute('y')) || 0;
      const len = el.getComputedTextLength();
      const lines = el.querySelectorAll('tspan').length || 1;
      const h = size * 0.78 + (lines > 1 ? size * 1.1 : 0);
      return { x: x - len / 2, y: y - h / 2, width: len, height: h };
    };
    const labels = [...document.querySelectorAll('#cmdRing .cmd-label')].map(el => ({
      text: el.textContent, size: parseFloat(getComputedStyle(el).fontSize), box: inkBox(el),
    }));
    const subs = [...document.querySelectorAll('#cmdRing .slot-sub')].map(el => ({
      text: el.textContent, size: parseFloat(getComputedStyle(el).fontSize), box: inkBox(el),
    }));
    return { count: labels.length, labels, subs, all: [...labels, ...subs] };
  })()`);
  check('วงคำสั่งมี 8 ช่อง', cmd.count === 8, `${cmd.count}`);
  const labelMin = Math.min(...cmd.labels.map((l) => l.size));
  const subMin = Math.min(...cmd.subs.map((l) => l.size));
  check('label วงคำสั่ง ≥ 13px', labelMin >= 13, `${labelMin.toFixed(2)}px`);
  check('sub วงคำสั่ง ≥ 11.5px', subMin >= 11.5, `${subMin.toFixed(2)}px`);
  const cmdCollisions = collisions(cmd.all);
  check('label + sub ของวงคำสั่งไม่ชนกัน', cmdCollisions.length === 0, cmdCollisions.join(', '));

  /* ---- state 3: quick bar (LB) ---- */
  await setState(QUICKBAR);
  await settle();

  const quick = await browser.evaluate(`(() => {
    const hud = document.getElementById('hud');
    const chips = [...document.querySelectorAll('.chip--status')].map(el => el.textContent.trim());
    return {
      mode: hud.dataset.mode,
      rings: hud.dataset.rings,
      slots: document.querySelectorAll('#quickbar .qb__slot').length,
      active: document.querySelectorAll('#quickbar .qb__slot.is-active').length,
      labels: [...document.querySelectorAll('#quickbar .qb__label')].map(el => el.textContent),
      pillMode: document.getElementById('pillMode').textContent.trim(),
      pillText: document.getElementById('pillText').textContent.trim(),
      chips,
    };
  })()`);
  check(
    'quick bar: แถบ 8 ช่อง + active 1 ช่อง',
    quick.mode === 'quickbar' && quick.rings === 'on' && quick.slots === 8 && quick.active === 1,
    `${quick.mode}/${quick.rings}/${quick.slots}/${quick.active}`,
  );
  check(
    'quick bar: ช่องตรง interaction model',
    quick.labels.join(',') === 'Copy,Paste,Cut,Undo,Redo,All,Save,Switch',
    quick.labels.join(','),
  );
  check('quick bar: pill = Quick bar', quick.pillMode === 'Quick bar', quick.pillMode);
  await browser.screenshot(path.join(SHOTS, 'hud-quickbar-100dpi.png'));

  /* ---- state 4: caret (R3) — ไม่มีวง ---- */
  await setState(CARET);
  await settle();

  const caret = await browser.evaluate(`(() => {
    const hud = document.getElementById('hud');
    return {
      mode: hud.dataset.mode,
      rings: hud.dataset.rings,
      inner: document.querySelectorAll('#innerRing > *').length,
      pillMode: document.getElementById('pillMode').textContent.trim(),
      pillText: document.getElementById('pillText').textContent.trim(),
      caretPad: getComputedStyle(document.getElementById('caretPad')).opacity,
    };
  })()`);
  check(
    'caret: ไม่มีวง (เหลือ pill)',
    caret.mode === 'caret' && caret.rings === 'off' && caret.inner === 0,
    `${caret.mode}/${caret.rings}/${caret.inner}`,
  );
  check(
    'caret: กากบาท 4 ทิศแสดง (สี mode-caret)',
    caret.caretPad === '1',
    `opacity=${caret.caretPad}`,
  );
  check(
    'caret: pill แสดงโหมดเลือกข้อความ',
    caret.pillMode === 'Caret' && caret.pillText === 'เลือกข้อความ',
    `${caret.pillMode} · ${caret.pillText}`,
  );
  await browser.screenshot(path.join(SHOTS, 'hud-caret-100dpi.png'));

  /* ---- 150% DPI + text scale 1.6 (แยกจากขนาดวง) ---- */
  const dpr = await browser.evaluate('window.devicePixelRatio');
  check('จอ 100% DPI → devicePixelRatio 1', dpr === 1, String(dpr));

  browser.close();
  browser = null;

  hidpi = await launch({
    url: `${BASE}/ui/hud/?preview=1&textScale=1.6&font=noto-sans-thai`,
    width: 1600,
    height: 1000,
    scaleFactor: 1.5,
  });
  await hidpi.evaluate('document.fonts.ready.then(() => true)');
  await setState(TYPE_EN);
  await settle();

  const hi = await hidpi.evaluate(`(() => {
    const px = (sel) => parseFloat(getComputedStyle(document.querySelector(sel)).fontSize);
    return {
      dpr: window.devicePixelRatio,
      group: px('.sector-label--group:not(.len-m):not(.len-l)'),
      char: px('.char-text.is-dim'),
      pill: px('.chip--status'),
      family: getComputedStyle(document.body).fontFamily,
      cssScale: getComputedStyle(document.documentElement).getPropertyValue('--hud-text-scale').trim(),
      ringScale: getComputedStyle(document.getElementById('hud')).getPropertyValue('--hud-scale').trim(),
    };
  })()`);

  check('จอ 150% DPI → devicePixelRatio 1.5', Math.abs(hi.dpr - 1.5) < 0.001, String(hi.dpr));
  check('textScale=1.6 → --hud-text-scale = 1.60', hi.cssScale === '1.60', hi.cssScale);
  check(
    'textScale 1.6 ขยายเฉพาะตัวหนังสือ (group ≈ 18.2px)',
    hi.group >= 17.5 && hi.group <= 19,
    `${hi.group.toFixed(2)}px`,
  );
  check(
    'ขนาดวงมาจาก state ไม่ใช่ text scale',
    Math.abs(parseFloat(hi.ringScale) - 1.0) < 0.001,
    `--hud-scale=${hi.ringScale}`,
  );
  check('font=noto-sans-thai ใช้ Noto Sans Thai', /^"?Noto Sans Thai/.test(hi.family), hi.family);

  await hidpi.screenshot(path.join(SHOTS, 'hud-typing-150dpi.png'));

  hidpi.close();
  hidpi = null;

  /* ---- Settings window (ดีไซน์ + ฟอนต์ preview) ---- */
  const settings = await launch({
    url: `${BASE}/ui/settings/index.html`,
    width: 660,
    height: 820,
    pageMatch: 'settings/index.html',
  });
  await settings.evaluate('document.fonts.ready.then(() => true)');
  await sleep(300);

  const ui = await settings.evaluate(`Promise.all([
    document.fonts.load('600 14px "IBM Plex Sans Thai"'),
  ]).then(() => {
    const group = document.querySelector('.preview__group');
    return {
      fonts: document.fonts.check('600 14px "IBM Plex Sans Thai"'),
      family: getComputedStyle(group).fontFamily,
      size: parseFloat(getComputedStyle(group).fontSize),
      slider: document.getElementById('textScale').value,
      options: document.querySelectorAll('input[name="font"]').length,
    };
  })`);

  check('Settings โหลดฟอนต์ไทยได้', ui.fonts === true);
  check('Settings preview ใช้ IBM Plex Sans Thai', /IBM Plex Sans Thai/.test(ui.family), ui.family);
  check('Settings มี slider + 3 ฟอนต์', ui.options === 3, `slider=${ui.slider} options=${ui.options}`);

  await settings.screenshot(path.join(SHOTS, 'settings-hud-100dpi.png'), { fullPage: true });
  settings.close();

  /* ---- สรุป ---- */
  const failed = results.filter((r) => !r.ok).length;
  console.log(`\n${results.length - failed}/${results.length} ผ่าน · screenshots: ${SHOTS}`);
  if (failed > 0) process.exitCode = 1;
} finally {
  if (browser) browser.close();
  if (hidpi) hidpi.close();
  server.kill();
  if (serverLog && process.exitCode) console.error(serverLog);
}
