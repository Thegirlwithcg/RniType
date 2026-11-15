// HUD mockup smoke test — ขับ Chrome headless ผ่าน CDP แล้วตรวจพฤติกรรมจริง
// รัน: node tools/tests/hud-smoke.mjs
// ต้องมี Chrome/Chromium (ค่าเริ่มต้นชี้ไปที่ Playwright cache)
// v0.7: วงใน = กลุ่ม/วงคำสั่ง (LS), วงนอก = ตัวอักษร (RS), D-pad ← = วงคำสั่ง

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const CHROME =
  process.env.CHROME_PATH ||
  'C:\\Users\\User\\AppData\\Local\\ms-playwright\\chromium-1208\\chrome-win64\\chrome.exe';

const PAGE = pathToFileURL(
  path.resolve('project menager/Design/05-mockup/hud-prototype/index.html')
).href;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const profile = mkdtempSync(path.join(tmpdir(), 'rni-hud-'));
const chrome = spawn(
  CHROME,
  [
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
    '--window-size=1440,900',
    PAGE
  ],
  { stdio: ['ignore', 'pipe', 'pipe'] }
);

let stderr = '';
const wsBrowser = await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error('ไม่พบ DevTools endpoint\n' + stderr)), 20000);
  chrome.stderr.on('data', (d) => {
    stderr += d.toString();
    const m = stderr.match(/DevTools listening on (ws:\/\/\S+)/);
    if (m) {
      clearTimeout(timer);
      resolve(m[1]);
    }
  });
  chrome.on('exit', (code) => reject(new Error(`chrome ออกก่อนกำหนด (${code})\n${stderr}`)));
});

const port = new URL(wsBrowser).port;
let target = null;
for (let i = 0; i < 60 && !target; i++) {
  try {
    const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    target = list.find((t) => t.type === 'page' && t.url.includes('index.html'));
  } catch {
    /* chrome ยังไม่พร้อม */
  }
  if (!target) await sleep(150);
}
if (!target) {
  chrome.kill();
  throw new Error('ไม่พบหน้า mockup ใน Chrome');
}

const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error('ต่อ CDP ไม่ได้'));
});

let msgId = 0;
const pending = new Map();
const exceptions = [];
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { res, rej } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
  } else if (msg.method === 'Runtime.exceptionThrown') {
    exceptions.push(msg.params.exceptionDetails.text);
  }
};

const send = (method, params = {}) =>
  new Promise((res, rej) => {
    const id = ++msgId;
    pending.set(id, { res, rej });
    ws.send(JSON.stringify({ id, method, params }));
  });

const evaluate = async (expression) => {
  const r = await send('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true
  });
  if (r.exceptionDetails) {
    throw new Error(
      r.exceptionDetails.text + ' ' + (r.exceptionDetails.exception?.description || '')
    );
  }
  return r.result.value;
};

await send('Runtime.enable');

// รอ boot
for (let i = 0; i < 60; i++) {
  const ready = await evaluate('typeof window.__rni !== "undefined"');
  if (ready) break;
  await sleep(100);
}

const key = (type, k) =>
  evaluate(
    `window.dispatchEvent(new KeyboardEvent(${JSON.stringify(type)}, { key: ${JSON.stringify(k)}, bubbles: true })), true`
  );

// ดัน LS ไปทาง (set ของทิศ) แล้วรอปล่อย — ใช้กับวงคำสั่ง
const lsPush = async (dirs) => {
  await evaluate(
    `${dirs.map((d) => `__rni.kb.ls.add('${d}')`).join(';')};
     for (let i=0;i<12;i++) __rni.step(1/60); true`
  );
};
const lsRelease = async () => {
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<14;i++) __rni.step(1/60); true`);
};
const rsPush = async (dirs) => {
  await evaluate(
    `${dirs.map((d) => `__rni.kb.rs.add('${d}')`).join(';')};
     for (let i=0;i<12;i++) __rni.step(1/60); true`
  );
};
const rsRelease = async () => {
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<14;i++) __rni.step(1/60); true`);
};

try {
  // 1) boot: วงนอก 8 sector + วงคำสั่ง (วงใน) 8 sector + chip mode
  const boot = await evaluate(`({
    char: document.querySelectorAll('.sector--char').length,
    cmd: document.querySelectorAll('#cmdRing .cmd-sector').length,
    chip: document.querySelector('#chipMode').textContent,
    toast: !!document.querySelector('#toast')
  })`);
  check('boot: 8 sectors วงนอก + 8 cmd sectors วงใน', boot.char === 8 && boot.cmd === 8, JSON.stringify(boot));
  check('boot: chipMode = Type EN', boot.chip === 'Type EN', boot.chip);

  // 1a) ตรวจภาษาเริ่มต้น (จำลอง): English + Thai เท่านั้น
  const detected = await evaluate(`({
    en: __rni.langEnabled.en, th: __rni.langEnabled.th,
    ru: __rni.langEnabled.ru, ja: __rni.langEnabled.ja
  })`);
  check('ตรวจภาษาเริ่มต้น: EN + TH (จำลองเครื่องไทย)',
    detected.en === true && detected.th === true && detected.ru === false && detected.ja === false,
    JSON.stringify(detected));

  // 1b) ค่าเริ่มต้น = B: idle เหลือ pill, ไม่มี aim wedge, ไม่มี chip L1
  await evaluate(`__rni.step(1/60); true`);
  const varB = await evaluate(`({
    rest: document.querySelector('#hud').classList.contains('is-rest'),
    aim: document.querySelector('#hud').classList.contains('is-aim'),
    mod: document.querySelector('#chipMod').textContent,
    ring: __rni.cfg.ringAlways
  })`);
  check('ค่าเริ่มต้น (B): idle เหลือ pill, aim ปิด, ไม่มี chip L1',
    varB.rest === true && varB.aim === false && varB.mod === '' && varB.ring === false,
    JSON.stringify(varB));

  // 1c) โหมดเรียนรู้ = แสดงวงตลอด + กลุ่มอื่นจางตอนพิมพ์
  await evaluate(`document.querySelector('#inRing').click(); __rni.step(1/60); true`);
  const ringOn = await evaluate(`({
    rest: document.querySelector('#hud').classList.contains('is-rest'),
    cfg: __rni.cfg.ringAlways
  })`);
  await evaluate(`__rni.kb.ls.add('up'); for (let i=0;i<8;i++) __rni.step(1/60); true`);
  const varBTyping = await evaluate(`({
    rest: document.querySelector('#hud').classList.contains('is-rest'),
    muted: document.querySelectorAll('#groupRing .sector-label.is-muted').length,
    armed: document.querySelectorAll('#groupRing .sector--group.is-armed').length
  })`);
  check('โหมดเรียนรู้ (checkbox): idle แสดงวง, พิมพ์แล้วกลุ่มอื่นจาง',
    ringOn.rest === false && ringOn.cfg === true && varBTyping.rest === false &&
    varBTyping.muted === 7 && varBTyping.armed === 1,
    JSON.stringify({ ringOn, ...varBTyping }));

  // 1d) เลื่อน LS เปลี่ยนกลุ่มได้โดยไม่ต้องปล่อย (2 → 4)
  const groupBefore = await evaluate(`__rni.st.group`);
  const lenBeforeSwitch = (await evaluate(`__rni.doc()`)).len;
  await evaluate(`__rni.kb.ls.clear(); __rni.kb.ls.add('right');
    for (let i=0;i<12;i++) __rni.step(1/60); true`);
  const switchState = await evaluate(`({ group: __rni.st.group, state: __rni.st.name, len: __rni.doc().len })`);
  check('เลื่อน LS เปลี่ยนกลุ่มได้ขณะดันค้าง',
    groupBefore === 0 && switchState.group === 2 && switchState.len === lenBeforeSwitch,
    JSON.stringify({ groupBefore, ...switchState }));

  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60);
    document.querySelector('#inRing').click(); __rni.step(1/60); true`);

  // 2) keymap ใหม่: กลุ่ม ↑ = สระ AEIOU + T N S (LS = วงใน)
  await evaluate(`__rni.kb.ls.add('up'); for (let i=0;i<8;i++) __rni.step(1/60); true`);
  const vowelChars = await evaluate('__rni.chars().join("")');
  check('EN กลุ่ม ↑ = aeioutns (พิมพ์เล็ก, Shift = ใหญ่)', vowelChars === 'aeioutns', vowelChars);
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  // 2c) Shift = toggle ตัวพิมพ์ใหญ่ (กดครั้งเดียวล็อก, กดอีกครั้งกลับตัวเล็ก)
  await evaluate(`__rni.kb.ls.add('up'); for (let i=0;i<8;i++) __rni.step(1/60); true`);
  await key('keydown', 'Shift');
  await evaluate(`for (let i=0;i<4;i++) __rni.step(1/60); true`);
  const shiftOn = await evaluate(`({
    lock: __rni.st.shiftLock,
    chars: __rni.chars().join(''),
    mod: document.querySelector('#chipMod').textContent
  })`);
  check('Shift ครั้งเดียว = ล็อกตัวใหญ่ + UI แสดง',
    shiftOn.lock === true && shiftOn.chars === 'AEIOUTNS' && /UPPER/.test(shiftOn.mod),
    JSON.stringify(shiftOn));

  await evaluate(`__rni.kb.rs.add('up'); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  const shiftPreview = await evaluate(`document.querySelector('#preview').textContent`);
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<15;i++) __rni.step(1/60); true`);
  const shiftLast = await evaluate(`__rni.st.lastChar`);
  check('พิมพ์ตัวใหญ่ได้ A', shiftPreview === 'A' && shiftLast === 'A', `${shiftPreview}/${shiftLast}`);

  await key('keyup', 'Shift');
  await evaluate(`for (let i=0;i<3;i++) __rni.step(1/60); true`);
  await key('keydown', 'Shift');
  await evaluate(`for (let i=0;i<4;i++) __rni.step(1/60); true`);
  const shiftOff = await evaluate(`({
    lock: __rni.st.shiftLock,
    chars: __rni.chars().join(''),
    mod: document.querySelector('#chipMod').textContent
  })`);
  check('Shift อีกครั้ง = กลับตัวเล็ก',
    shiftOff.lock === false && shiftOff.chars === 'aeioutns' && !/UPPER/.test(shiftOff.mod),
    JSON.stringify(shiftOff));
  await key('keyup', 'Shift');
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  // 2c2) "Other language" slot: shows TH (not EN); direct switch; hidden if only one language
  await evaluate(`__rni.step(1/60); true`);
  const slotDefault = await evaluate(`({
    label: document.querySelectorAll('#cmdRing .cmd-label')[1].textContent,
    hidden: document.querySelectorAll('#cmdRing .cmd-sector')[1].classList.contains('is-hidden')
  })`);
  check('ช่องภาษาอื่นแสดง TH (ไม่ใช่ EN)', slotDefault.label === 'TH' && slotDefault.hidden === false,
    JSON.stringify(slotDefault));

  await key('keydown', 'm');
  await rsPush(['up', 'right']);
  const slotIdx = await evaluate(`__rni.cmd.idx`);
  await rsRelease();
  const directMode = await evaluate(`__rni.cfg.mode`);
  check('เลือกช่องภาษาอื่น = สลับเป็นไทยทันที', slotIdx === 1 && directMode === 'type-th',
    `${slotIdx}/${directMode}`);
  await evaluate(`__rni.setMode('type-en'); __rni.step(1/60); true`);

  await evaluate(`__rni.langEnabled.th = false; __rni.step(1/60); true`);
  const single = await evaluate(`({
    label: document.querySelectorAll('#cmdRing .cmd-label')[1].textContent,
    hidden: document.querySelectorAll('#cmdRing .cmd-sector')[1].classList.contains('is-hidden')
  })`);
  await key('keydown', 'm');
  await rsPush(['up', 'right']);
  const ignoredIdx = await evaluate(`__rni.cmd.idx`);
  await evaluate(`__rni.closeCmd(); __rni.kb.rs.clear(); __rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);
  check('ภาษาเดียว: ไม่มีช่องภาษาอื่นให้เลือก',
    single.label === '' && single.hidden === true && ignoredIdx === -1,
    JSON.stringify({ ...single, ignoredIdx }));
  await evaluate(`__rni.langEnabled.th = true; __rni.step(1/60); true`);

  // 2b) ภาษาไทย: วรรณยุกต์ ่ ้ ๊ ๋ อยู่กลุ่มเดียวกัน + สระ ุ ู มีขีดกำกับ
  await key('keydown', '2');
  await evaluate(`__rni.kb.ls.add('right'); for (let i=0;i<12;i++) __rni.step(1/60); true`);
  const namChars = await evaluate(`__rni.chars().join('')`);
  await evaluate(`__rni.kb.ls.clear(); __rni.kb.ls.add('right'); __rni.kb.ls.add('down');
    for (let i=0;i<12;i++) __rni.step(1/60); true`);
  const tonChars = await evaluate(`__rni.chars().join('')`);
  await evaluate(`__rni.kb.ls.clear(); __rni.kb.ls.add('up'); __rni.kb.ls.add('right');
    for (let i=0;i<12;i++) __rni.step(1/60); true`);
  const guideCount = await evaluate(`document.querySelectorAll('#charRing .char-guide.is-on').length`);
  check('TH: กลุ่มวรรณยุกต์รวม ่ ้ ๊ ๋', tonChars === '่้๊๋์ำั็', `วรรณ=${tonChars} นำ=${namChars}`);
  check('TH: กลุ่มนำ = เแโใไอ + ฤฮ', namChars === 'เแโใไอฤฮ', namChars);
  check('TH: สระล่าง (ุ ู) มีขีดกำกับ', guideCount >= 2, String(guideCount));
  await key('keydown', '1');
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  // 2d) หลายภาษา: ช่องภาษาอื่นเป็น "Lang" → เปิดวงภาษา (TH/RU/JA)
  await evaluate(`__rni.langEnabled.ru = true; __rni.langEnabled.ja = true;
    __rni.setMode('type-en'); __rni.step(1/60); true`);
  const slotMulti = await evaluate(`document.querySelectorAll('#cmdRing .cmd-label')[1].textContent`);
  await key('keydown', 'm');
  await rsPush(['up', 'right']);
  await rsRelease();
  const langOpenState = await evaluate(`({
    open: __rni.langWheel.open,
    codes: [...document.querySelectorAll('#langRing .lang-label')].map((e) => e.textContent).filter(Boolean)
  })`);
  check('หลายภาษา: ช่องเป็น Lang → เปิดวงภาษา (TH/RU/JA)',
    slotMulti === 'Lang' && langOpenState.open === true && langOpenState.codes.join(',') === 'TH,RU,JA',
    JSON.stringify({ slotMulti, ...langOpenState }));

  await rsPush(['down', 'right']);
  const ruIdx = await evaluate(`__rni.langWheel.idx`);
  await rsRelease();
  await evaluate(`__rni.kb.ls.add('up'); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  const ruState = await evaluate(`({ mode: __rni.cfg.mode, chars: __rni.chars().join('') })`);
  check('RU: mapping จากตำแหน่ง QWERTY (a→ф)',
    ruIdx === 1 && ruState.mode === 'type-ru' && ruState.chars[0] === 'ф',
    JSON.stringify({ ruIdx, ...ruState }));
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  await evaluate(`__rni.openLangWheel(); __rni.step(1/60); true`);
  await rsPush(['down', 'left']);
  const jaIdx = await evaluate(`__rni.langWheel.idx`);
  await rsRelease();
  await evaluate(`__rni.kb.ls.add('up'); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  const jaState = await evaluate(`({
    mode: __rni.cfg.mode,
    chars: __rni.chars().join(''),
    mod: document.querySelector('#chipMod').textContent
  })`);
  check('IME (JA): ส่งคีย์ QWERTY ตรง + badge IME',
    jaIdx === 2 && jaState.mode === 'type-ja' && jaState.chars === 'aeioutns' && /IME/.test(jaState.mod),
    JSON.stringify({ jaIdx, ...jaState }));
  await evaluate(`__rni.kb.ls.clear(); __rni.setMode('type-en'); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  // 2e) Settings window (separate, English UI) + keymap editor + reset
  await evaluate(`document.querySelector('#btnSettings').click(); true`);
  const ui = await evaluate(`({
    title: document.querySelector('.settings__title').textContent,
    tabs: [...document.querySelectorAll('#settingsTabs button')].map((b) => b.textContent.trim()),
    open: !document.querySelector('#settings').hidden
  })`);
  check('Settings window: แยกหน้าต่าง + UI อังกฤษ',
    ui.open === true && ui.title === 'RniType Settings' && ui.tabs.join(',') === 'General,Languages,Keymap,HUD',
    JSON.stringify(ui));

  await evaluate(`document.querySelector('#settingsTabs [data-tab="keymap"]').click(); true`);
  await evaluate(`document.querySelector('#mapLang').value = 'ru';
    document.querySelector('#mapLang').dispatchEvent(new Event('change', { bubbles: true })); true`);
  await evaluate(`document.querySelectorAll('#mapGrid .mapcell')[0].click(); true`);
  const listening = await evaluate(`document.querySelectorAll('#mapGrid .mapcell.is-listening').length`);
  await evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', bubbles: true })); true`);
  const mapped = await evaluate(`(__rni.langOverrides.ru || {})['0-0']`);
  await evaluate(`document.querySelector('#btnResetLang').click(); true`);
  const resetMap = await evaluate(`(__rni.langOverrides.ru || {})['0-0']`);
  check('Keymap editor: คลิกช่อง → กดคีย์ใหม่ → reset ได้',
    listening === 1 && mapped === 'z' && resetMap === undefined,
    JSON.stringify({ listening, mapped, resetMap }));

  await evaluate(`__rni.cfg.opacity = 0.5; __rni.langEnabled.ru = false;
    __rni.resetSettings('all'); __rni.step(1/60); true`);
  const resetAll = await evaluate(`({
    op: __rni.cfg.opacity,
    ru: __rni.langEnabled.ru,
    th: __rni.langEnabled.th,
    mode: __rni.cfg.mode
  })`);
  check('Reset all = กลับค่าเริ่มต้น (EN+TH)',
    resetAll.op === 0.92 && resetAll.ru === false && resetAll.th === true && resetAll.mode === 'type-en',
    JSON.stringify(resetAll));
  await evaluate(`document.querySelector('#btnSettingsClose').click(); true`);

  // 3) วงคำสั่ง: เลือกด้วย RS (อนาล็อกขวา), LS ไม่มีผล, ปล่อย RS = ยืนยัน
  await key('keydown', 'm');
  const cmdOpen = await evaluate(`__rni.cmd.open`);
  check('M = เปิดวงคำสั่ง', cmdOpen === true, String(cmdOpen));

  await evaluate(`__rni.kb.ls.add('right'); for (let i=0;i<12;i++) __rni.step(1/60); true`);
  const lsIdx = await evaluate(`__rni.cmd.idx`);
  check('LS ในวงคำสั่ง = ไม่เลือกอะไร (RS เท่านั้น)', lsIdx === -1, String(lsIdx));
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<6;i++) __rni.step(1/60); true`);

  await rsPush(['right']);
  const cmdIdx = await evaluate(`__rni.cmd.idx`);
  const cmdCenter = await evaluate(`document.querySelector('#cmdCenter').textContent`);
  check('RS → ไฮไลต์ Mouse (ช่องที่ 3)', cmdIdx === 2, String(cmdIdx));
  check('วงคำสั่ง: ชื่อที่เลือกอยู่กลางวง', cmdCenter === 'Mouse', cmdCenter);
  await rsRelease();
  const cmdDone = await evaluate(`({ mode: document.body.dataset.mode, open: __rni.cmd.open })`);
  check('ปล่อย RS = ยืนยัน Mouse', cmdDone.mode === 'mouse' && cmdDone.open === false, JSON.stringify(cmdDone));

  // 4) เปิด/ยกเลิกวงคำสั่งด้วย C + B
  await key('keydown', 'c');
  const cOpen = await evaluate(`__rni.cmd.open`);
  await key('keydown', 'backspace'); // B = ยกเลิก (ไม่ใช่ backspace ของเอกสาร)
  const cState = await evaluate(`({ open: __rni.cmd.open, mode: document.body.dataset.mode })`);
  check('C เปิดวงคำสั่ง · B ยกเลิก ไม่เปลี่ยนโหมด',
    cOpen === true && cState.open === false && cState.mode === 'mouse', JSON.stringify(cState));

  // 5) mode switch ด้วยคีย์บอร์ด (3 = Mouse)
  await key('keydown', '1');
  await key('keydown', '3');
  const mouseMode = await evaluate(`document.body.dataset.mode`);
  check('กด 3 = Mouse mode', mouseMode === 'mouse', mouseMode);

  // 6) mouse mode: RS เลื่อน pointer
  const before = await evaluate(`({ x: __rni.pointer.x, y: __rni.pointer.y })`);
  await evaluate(`__rni.kb.rs.add('up'); for (let i=0;i<25;i++) __rni.step(1/60); true`);
  const after = await evaluate(`({ x: __rni.pointer.x, y: __rni.pointer.y })`);
  check('mouse: ลาก RS ขึ้น = pointer ขึ้น', after.y < before.y - 20, `${before.y} → ${after.y}`);
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<5;i++) __rni.step(1/60); true`);

  // 7) mouse mode: RB = คลิกซ้าย (มี ripple)
  await evaluate(`__rni.kb.rb = true; __rni.step(1/60); __rni.step(1/60); true`);
  const ripple = await evaluate(`document.querySelectorAll('.ripple').length`);
  check('mouse: RB แตะ = มี click ripple', ripple > 0, String(ripple));
  await evaluate(`__rni.kb.rb = false; __rni.step(1/60); true`);

  // 8) mouse drag: RB ค้าง + RS = ลากเลือกข้อความ
  await evaluate(`__rni.setPointer(300, 170); __rni.kb.lt = true; __rni.step(1/60); true`);
  await evaluate(`__rni.kb.rb = true; __rni.step(1/60); __rni.step(1/60); true`);
  const clickCaret = await evaluate(`__rni.doc().caret`);
  await evaluate(`__rni.kb.rs.add('left'); for (let i=0;i<14;i++) __rni.step(1/60); true`);
  const dragSel = await evaluate(`({ ...__rni.doc(), px: Math.round(__rni.pointer.x), py: Math.round(__rni.pointer.y), rb: __rni.pad.rb, drag: __rni.cfg.mode })`);
  check('mouse: RB + RS = ลากเลือกข้อความ', clickCaret > 0 && dragSel.sel === true, JSON.stringify(dragSel));
  await evaluate(
    `__rni.kb.rs.clear(); __rni.kb.rb = false; __rni.kb.lt = false;
     for (let i=0;i<6;i++) __rni.step(1/60); true`
  );
  await key('keydown', 'backspace'); // ลบ selection ทิ้งก่อนไปต่อ

  // 9) กลับ Type EN + RB ค้าง = วงล้อ 2
  await key('keydown', '1');
  await evaluate(
    `__rni.kb.rb = true; __rni.kb.ls.add('up');
     for (let i=0;i<10;i++) __rni.step(1/60); true`
  );
  const layerHeld = await evaluate(
    `({ layer: document.body.dataset.layer, chars: __rni.chars().join(''), last: __rni.st.lastChar, mod: document.querySelector('#chipMod').textContent })`
  );
  check('RB ค้าง = layer 2 (A–H)', layerHeld.layer === '2' && layerHeld.chars === 'ABCDEFGH', JSON.stringify(layerHeld));
  await evaluate(`__rni.kb.ls.clear(); __rni.kb.rb = false; for (let i=0;i<5;i++) __rni.step(1/60); true`);
  const layerLocked = await evaluate(`document.body.dataset.layer`);
  check('RB แตะ = ล็อก layer 2', layerLocked === '2', layerLocked);
  await evaluate(`__rni.kb.rb = true; __rni.step(1/60); __rni.kb.rb = false; __rni.step(1/60); true`);
  const layerBack = await evaluate(`document.body.dataset.layer`);
  check('แตะ RB อีกครั้ง = กลับ layer 1', layerBack === '1', layerBack);

  // 10) ค่าเริ่มต้น: RS เดี่ยวต้องไม่ขยับ caret (กันบรรทัดเลื่อน)
  const caret0 = (await evaluate(`__rni.doc()`)).caret;
  await evaluate(
    `__rni.kb.rs.add('left'); for (let i=0;i<8;i++) __rni.step(1/60);
     __rni.kb.rs.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`
  );
  const caretIdle = (await evaluate(`__rni.doc()`)).caret;
  check('RS เดี่ยว (ยังไม่เปิด) = caret ไม่ขยับ', caretIdle === caret0, `${caret0} → ${caretIdle}`);

  // 11) กด R3 หนึ่งครั้ง = เปิดลูกศร แล้ว RS เลื่อน caret ได้
  await key('keydown', 'n');
  await sleep(380);
  const dirOn = await evaluate(`__rni.dirMode.enabled`);
  await evaluate(
    `__rni.kb.rs.add('left'); for (let i=0;i<8;i++) __rni.step(1/60);
     __rni.kb.rs.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`
  );
  const caret1 = (await evaluate(`__rni.doc()`)).caret;
  check('R3 ครั้งเดียว = เปิดลูกศร, RS เลื่อน caret', dirOn === true && caret1 === caretIdle - 1,
    `on=${dirOn} ${caretIdle} → ${caret1}`);

  // 12) R3 สองครั้งติดกัน + RS = ขยาย selection
  await evaluate(
    `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));
     window.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true })); true`
  );
  const selOn = await evaluate(`__rni.doc().selMode`);
  await evaluate(
    `__rni.kb.rs.add('right'); for (let i=0;i<8;i++) __rni.step(1/60);
     __rni.kb.rs.clear(); for (let i=0;i<6;i++) __rni.step(1/60); true`
  );
  const selNow = await evaluate(`__rni.doc()`);
  check(
    'R3×2 แล้วเลื่อน RS = เลือกข้อความ',
    selOn === true && selNow.sel === true && selNow.anchor >= 0,
    JSON.stringify(selNow)
  );
  await key('keydown', ' '); // A = ยืนยัน (selection ยังอยู่)
  await key('keydown', 'backspace'); // ลบก่อนไปต่อ

  // 13) Quick bar (6)
  await key('keydown', '6');
  const quickMode = await evaluate(`document.body.dataset.mode`);
  check('กด 6 = Quick bar', quickMode === 'quickbar', quickMode);
  await evaluate(`__rni.kb.rs.add('up'); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<20;i++) __rni.step(1/60); true`);
  const quickToast = await evaluate(`document.querySelector('#toast').textContent`);
  check('Quick bar: RS ↑ แล้วปล่อย = Copy', /Copy/.test(quickToast) && /Ctrl \+ C/.test(quickToast), quickToast);

  // 14) HUD toggle + disabled (7 = HUD, 8 = พัก)
  await key('keydown', '7');
  const hudHidden = await evaluate(`__rni.cfg.hudHidden`);
  check('กด 7 = ซ่อน HUD', hudHidden === true, String(hudHidden));
  await key('keydown', '8');
  await evaluate('__rni.step(1/60); true');
  const disabled = await evaluate(`({ mode: document.body.dataset.mode, off: document.querySelector('#hud').classList.contains('is-off') })`);
  check('กด 8 = โหมดพัก', disabled.mode === 'disabled' && disabled.off, JSON.stringify(disabled));

  // 14b) วงคำสั่งยังเปิดได้ตอนพัก แล้วกลับเข้าโหมด EN
  await key('keydown', 'c');
  await rsPush(['up']);
  await rsRelease();
  const backEn = await evaluate(`({ mode: document.body.dataset.mode, open: __rni.cmd.open })`);
  check('พักอยู่: วงคำสั่งพากลับ Type EN ได้', backEn.mode === 'type-en' && backEn.open === false, JSON.stringify(backEn));

  // 15) พิมพ์จริง: EN 'E' (LS ↑ + RS 1)
  const beforeChars = await evaluate(`document.querySelector('#statChars').textContent`);
  await evaluate(
    `__rni.kb.ls.add('up');
     for (let i=0;i<10;i++) __rni.step(1/60);
     __rni.kb.rs.add('up');
     for (let i=0;i<10;i++) __rni.step(1/60); true`
  );
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<15;i++) __rni.step(1/60); true`);
  const typed = await evaluate(`document.querySelector('#statChars').textContent`);
  const dbg = await evaluate(`({
    mode: __rni.cfg.mode,
    state: __rni.st.name,
    group: __rni.st.group,
    sector: __rni.st.sector,
    lock: __rni.st.lock,
    lm: Math.hypot(__rni.pad.ls.x, __rni.pad.ls.y).toFixed(2),
    rm: Math.hypot(__rni.pad.rs.x, __rni.pad.rs.y).toFixed(2),
    chars: __rni.chars().join(''), last: __rni.st.lastChar
  })`);
  check('พิมพ์ EN: ตัวอักษรถูกส่งเข้าเอกสาร', Number(typed) === Number(beforeChars) + 1, `${beforeChars} → ${typed} ${JSON.stringify(dbg)}`);
  const ghostCount = await evaluate(`document.querySelectorAll('.ghost').length`);
  check('ไม่มี ghost บิน (เหลือ preview ที่ caret)', ghostCount === 0, String(ghostCount));
  await evaluate(`__rni.kb.ls.clear(); for (let i=0;i<5;i++) __rni.step(1/60); true`);

  // 16) วงคำสั่งฝั่งแก้ข้อความ: Copy ↓ · Paste ↙ · Undo ← · Redo ↖
  const lenAfterType = (await evaluate(`__rni.doc()`)).len;

  // 16a) เปิดวงขณะ RS ค้าง = ยังไม่เลือกอะไร (กันคำสั่งพลาด)
  await evaluate(`__rni.kb.rs.add('down'); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  await key('keydown', 'c');
  await evaluate(`for (let i=0;i<10;i++) __rni.step(1/60); true`);
  const guardIdx = await evaluate(`__rni.cmd.idx`);
  check('เปิดวงขณะ RS ค้าง = ต้องปล่อย RS ก่อน (ยังไม่เลือก)', guardIdx === -1, String(guardIdx));
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<10;i++) __rni.step(1/60); true`);

  // 16b) Undo (←)
  await rsPush(['left']);
  const undoIdx = await evaluate(`__rni.cmd.idx`);
  await rsRelease();
  const lenAfterUndo = (await evaluate(`__rni.doc()`)).len;
  check('วงใน: Undo ย้อนการพิมพ์', undoIdx === 6 && lenAfterUndo === lenAfterType - 1,
    `${lenAfterType} → ${lenAfterUndo} idx=${undoIdx}`);

  // 16c) Redo (↖)
  await key('keydown', 'c');
  await rsPush(['up', 'left']);
  const redoIdx = await evaluate(`__rni.cmd.idx`);
  await rsRelease();
  const lenAfterRedo = (await evaluate(`__rni.doc()`)).len;
  check('วงใน: Redo กลับมาเท่าเดิม', redoIdx === 7 && lenAfterRedo === lenAfterType,
    `${lenAfterUndo} → ${lenAfterRedo} idx=${redoIdx}`);

  // 16d) Copy (↓) ต้องเก็บข้อความที่เลือกไว้ใช้ Paste ได้
  await key('keydown', 'n');
  await key('keydown', 'n');
  await evaluate(`__rni.kb.rs.add('right'); for (let i=0;i<6;i++) __rni.step(1/60); __rni.kb.rs.clear(); for (let i=0;i<5;i++) __rni.step(1/60); true`);
  await key('keydown', 'c');
  await rsPush(['down']);
  const copyIdx = await evaluate(`__rni.cmd.idx`);
  await rsRelease();
  const clipLen = await evaluate(`__rni.clipboard().length`);
  check('วงใน: Copy เก็บข้อความที่เลือก', copyIdx === 4 && clipLen > 0, `idx=${copyIdx} len=${clipLen}`);

  // 16e) Paste (↙) — ลบ selection ก่อนเพื่อให้เห็นการแทรกชัดเจน
  await key('keydown', 'backspace');
  const lenBeforePaste = (await evaluate(`__rni.doc()`)).len;
  await key('keydown', 'c');
  await rsPush(['down', 'left']);
  const pasteIdx = await evaluate(`__rni.cmd.idx`);
  await rsRelease();
  const lenAfterPaste = (await evaluate(`__rni.doc()`)).len;
  check('วงใน: Paste แทรกข้อความ', pasteIdx === 5 && lenAfterPaste === lenBeforePaste + clipLen,
    `${lenBeforePaste} → ${lenAfterPaste} (+${clipLen}) idx=${pasteIdx}`);

  // 17) Nav = Pivot: เลื่อน UI ได้, HUD ไม่ตามเมาส์, R = กลับตำแหน่ง
  await key('keydown', '4');
  const navP0 = await evaluate(`({ x: __rni.pivot.x, y: __rni.pivot.y })`);
  await evaluate(`__rni.kb.rs.add('left'); for (let i=0;i<20;i++) __rni.step(1/60); true`);
  const navP1 = await evaluate(`({
    x: __rni.pivot.x, y: __rni.pivot.y,
    hudLeft: parseFloat(document.querySelector('#hud').style.left)
  })`);
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<10;i++) __rni.step(1/60); true`);
  const navSettled = await evaluate(`({ x: __rni.pivot.x, y: __rni.pivot.y })`);
  await evaluate(`__rni.setPointer(60, 60); __rni.step(1/60); true`);
  const navP2 = await evaluate(`({ x: __rni.pivot.x, y: __rni.pivot.y, hudLeft: parseFloat(document.querySelector('#hud').style.left) })`);
  check('Nav: RS เลื่อน pivot (UI)', navP1.x < navP0.x - 20 && Math.abs(navP1.hudLeft - navP1.x) < 1,
    `${navP0.x} → ${navP1.x}`);
  check('UI ไม่ตามเมาส์แล้ว', navP2.x === navSettled.x && navP2.y === navSettled.y &&
    Math.abs(navP2.hudLeft - navSettled.x) < 1, JSON.stringify(navP2));
  await key('keydown', 'r');
  const navReset = await evaluate(`__rni.pivot.x`);
  check('Nav: R = UI กลับตำแหน่งเดิม', navReset > navP1.x + 20, `${navP1.x} → ${navReset}`);
  await key('keydown', '1');
  await evaluate(`__rni.kb.rs.clear(); for (let i=0;i<8;i++) __rni.step(1/60); true`);

  // 17) ไม่มี exception จากหน้า
  check('ไม่มี JS exception', exceptions.length === 0, exceptions.join(' | '));
} catch (err) {
  check('test runner', false, err.message);
} finally {
  try {
    ws.close();
  } catch {}
  chrome.kill();
  await sleep(300);
  try {
    rmSync(profile, { recursive: true, force: true });
  } catch {}
}

const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed}/${results.length} ผ่าน`);
process.exit(failed ? 1 : 0);
