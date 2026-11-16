// Settings window smoke test (P5 slice 2) — preview mode (ไม่ใช้ Tauri)
// รัน: node tools/tests/settings-smoke.mjs
// ตรวจ: tabs, General status/pause/exclusion, Languages rows + IME badge,
//       HUD slider/reset และถ่าย screenshot ลง Design/05-mockup/hud-shots

import { mkdirSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import { launch, sleep } from './browser.mjs';

const SHOTS = path.join(path.resolve('project menager', 'Design', '05-mockup', 'hud-shots'));
const PAGE = pathToFileURL(path.resolve('ui', 'settings', 'index.html')).href;

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` — ${detail}` : ''}`);
}

mkdirSync(SHOTS, { recursive: true });

const browser = await launch({ url: PAGE, width: 900, height: 700, pageMatch: 'settings/index.html' });

try {
  // ให้ settings.js boot (preview) จบ
  await sleep(400);

  /* ---------------- tabs ---------------- */

  const tabs = await browser.evaluate(`Array.from(document.querySelectorAll('[data-tab]')).map((b) => b.dataset.tab)`);
  check('sidebar has General/Languages/HUD tabs', JSON.stringify(tabs) === '["general","languages","hud"]', tabs.join(','));

  const keymapDisabled = await browser.evaluate(
    `document.querySelector('.nav__item.is-disabled')?.textContent.trim() === 'Keymap'`
  );
  check('Keymap stays disabled until universal mapping', keymapDisabled === true);

  const startTab = await browser.evaluate(
    `document.querySelector('[data-panel="general"]').hidden === false && document.querySelector('[data-panel="languages"]').hidden === true`
  );
  check('General is the initial pane', startTab === true);

  /* ---------------- General ---------------- */

  const device = await browser.evaluate(`document.getElementById('statusDevice').textContent`);
  check('status shows a connected controller in preview', device.includes('Xbox'), device);

  const usageLabels = await browser.evaluate(
    `Array.from(document.querySelectorAll('[data-panel="general"] .status-grid--4 .stat__label')).map((el) => el.textContent)`
  );
  check('usage cards show chars/WPM/latency/uptime',
    JSON.stringify(usageLabels) === '["Characters","WPM · 60 s","Action latency","Uptime"]',
    usageLabels.join(' · '));

  await browser.evaluate(`document.getElementById('pauseToggle').click()`);
  await sleep(100);
  const paused = await browser.evaluate(
    `document.getElementById('statusTyping').textContent === 'Paused' && document.getElementById('pauseToggle').checked`
  );
  check('pause toggle flips typing status', paused === true);
  await browser.evaluate(`document.getElementById('pauseToggle').click()`);

  await browser.evaluate(`
    document.getElementById('exclusionInput').value = 'game.exe';
    document.getElementById('exclusionForm').dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
  `);
  await sleep(80);
  const added = await browser.evaluate(`document.querySelectorAll('#exclusionList .list__row').length === 1`);
  check('adding an exclusion renders a row', added === true);

  const nameShown = await browser.evaluate(`document.querySelector('#exclusionList .list__name').textContent`);
  check('exclusion row shows the exe name', nameShown === 'game.exe', nameShown);

  await browser.screenshot(path.join(SHOTS, 'settings-general.png'));

  await browser.evaluate(`document.querySelector('#exclusionList .list__row button').click()`);
  await sleep(80);
  const removed = await browser.evaluate(
    `document.querySelectorAll('#exclusionList .list__row').length === 0 && !!document.querySelector('#exclusionList .list__empty')`
  );
  check('removing an exclusion restores the empty state', removed === true);

  /* ---------------- Languages ---------------- */

  await browser.evaluate(`document.querySelector('[data-tab="languages"]').click()`);
  await sleep(300);

  const langRows = await browser.evaluate(`document.querySelectorAll('#langList .lang').length`);
  check('Languages tab lists detected layouts', langRows === 3, `rows=${langRows}`);

  const summary = await browser.evaluate(`document.getElementById('langSummary').textContent`);
  check('summary names detected languages', summary.includes('English') && summary.includes('Thai'), summary);

  const enabled = await browser.evaluate(`
    Array.from(document.querySelectorAll('#langList .lang')).map((row) => ({
      code: row.querySelector('.lang__code').textContent,
      on: row.querySelector('input').checked,
      disabled: row.querySelector('input').disabled,
    }))
  `);
  check('EN and TH start enabled', enabled[0].code === 'EN' && enabled[0].on && enabled[1].code === 'TH' && enabled[1].on,
    enabled.map((row) => `${row.code}:${row.on ? 'on' : 'off'}`).join(' '));
  check('EN is the locked base language', enabled[0].disabled === true);
  check('IME layout is disabled until mapping exists', enabled[2].code === 'JA' && enabled[2].disabled === true);

  const imeBadge = await browser.evaluate(
    `Array.from(document.querySelectorAll('#langList .lang')).some((row) => row.textContent.includes('IME · keys') && row.textContent.includes('Needs mapping'))`
  );
  check('IME row carries the IME + needs-mapping badges', imeBadge === true);

  await browser.screenshot(path.join(SHOTS, 'settings-languages.png'));

  await browser.evaluate(`document.querySelectorAll('#langList .lang input')[1].click()`);
  await sleep(80);
  const thOff = await browser.evaluate(`document.querySelectorAll('#langList .lang input')[1].checked === false`);
  check('toggling Thai off updates the switch', thOff === true);

  /* ---------------- HUD ---------------- */

  await browser.evaluate(`document.querySelector('[data-tab="hud"]').click()`);
  await sleep(80);

  await browser.evaluate(`
    const slider = document.getElementById('textScale');
    slider.value = '1.8';
    slider.dispatchEvent(new Event('input', { bubbles: true }));
  `);
  await sleep(60);
  const output = await browser.evaluate(`document.getElementById('textScaleValue').textContent`);
  check('text size slider updates its output', output === '1.80×', output);

  const previewScale = await browser.evaluate(
    `getComputedStyle(document.documentElement).getPropertyValue('--preview-text-scale').trim()`
  );
  check('preview text scale follows the slider', previewScale === '1.80', previewScale);

  await browser.evaluate(`document.getElementById('reset').click()`);
  await sleep(100);
  const resetOutput = await browser.evaluate(`document.getElementById('textScaleValue').textContent`);
  check('Reset HUD restores 1.40×', resetOutput === '1.40×', resetOutput);

  const exceptions = browser.exceptions.length;
  check('no runtime exceptions', exceptions === 0, exceptions ? browser.exceptions.join(' | ') : '');
} finally {
  browser.close();
}

const passed = results.filter((result) => result.ok).length;
console.log(`\n${passed}/${results.length} ผ่าน · screenshots: ${SHOTS}`);
if (passed !== results.length) process.exit(1);
