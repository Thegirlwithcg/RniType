/* ============================================================
   RniType — Settings window controller (P5 slice 2)
   ------------------------------------------------------------
   General  : status (get_status/app:status) · pause · exclusion list
   Languages: rescan_languages · set_language_enabled
   HUD      : get_config / save_config + hud:style event
   Reset    : reset_config { section: general | languages | hud | all }
   ============================================================ */
(() => {
  'use strict';

  const DEFAULT_UI = {
    hudTextScale: 1.4,
    hudScale: 1.0,
    hudFontFamily: window.RniFonts.default,
  };
  const SAVE_DELAY_MS = 200;
  const MAX_ENABLED_LANGUAGES = 8;

  // ใช้เมื่อเปิดไฟล์ตรง ๆ โดยไม่มี Tauri (preview)
  const PREVIEW_LANGS = [
    { code: 'en', klid: '00000409', label: 'English (United States)', kind: 'layout', detected: true, builtin: false, supported: true, enabled: true, locked: true },
    { code: 'th', klid: '0000041E', label: 'Thai (Thailand)', kind: 'layout', detected: true, builtin: false, supported: true, enabled: true },
    { code: 'ja', klid: '00000411', label: 'Japanese (Japan)', kind: 'ime', detected: true, builtin: false, supported: false, enabled: false },
  ];

  // ตารางตัวอย่างสำหรับ preview (ไม่มี Tauri) — ตรงกับตาราง EN ใน core
  const PREVIEW_ROWS = {
    base: [
      ['a', 'e', 'i', 'o', 'u', 't', 'n', 's'],
      ['h', 'r', 'd', 'l', 'c', 'm', 'f', 'w'],
      ['g', 'p', 'b', 'v', 'k', 'x', 'y', 'j'],
      ['q', 'z', "'", '"', '-', '_', '.', ','],
      ['1', '2', '3', '4', '5', '6', '7', '8'],
      ['9', '0', '/', '\\', '(', ')', '[', ']'],
      ['!', '?', '@', '#', '$', '%', '&', '*'],
      [':', ';', '=', '+', '<', '>', '`', '~'],
    ],
    layer2: [
      ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'],
      ['I', 'J', 'K', 'L', 'M', 'N', 'O', 'P'],
      ['Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X'],
      ['{', '}', '|', '^', '¬', '°', '±', '§'],
      ['F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8'],
      ['F9', 'F10', 'F11', 'F12', 'PrintScreen', 'Insert', '', ''],
      ['Home', 'End', 'PageUp', 'PageDown', 'Left', 'Up', 'Down', 'Right'],
      ['Delete', 'Backspace', 'Escape', 'Enter', 'Tab', 'Space', '', ''],
    ],
  };

  const PREVIEW_GROUPS = {
    base: ['AEIOU', 'B', 'C', 'D', '1', '2', '3', '4'],
    layer2: ['A–H', 'I–P', 'Q–X', 'Sym', 'F1–F8', 'F9–F12', 'Nav', 'Keys'],
  };

  function previewKeymap() {
    const build = (rows) => {
      const cells = rows.flat();
      return { cells, defaults: cells.slice() };
    };
    return {
      code: mapLang,
      label: mapLang === 'th' ? 'Thai' : 'English',
      groups: PREVIEW_GROUPS.base,
      groups2: PREVIEW_GROUPS.layer2,
      layers: [
        { name: 'base', ...build(PREVIEW_ROWS.base) },
        { name: 'layer 2', ...build(PREVIEW_ROWS.layer2) },
      ],
      overrides: {},
    };
  }

  const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
  const invoke =
    tauri && tauri.core && typeof tauri.core.invoke === 'function'
      ? tauri.core.invoke.bind(tauri.core)
      : null;
  const listen =
    tauri && tauri.event && typeof tauri.event.listen === 'function'
      ? tauri.event.listen.bind(tauri.event)
      : null;

  const $ = (id) => document.getElementById(id);

  const dom = {
    nav: Array.from(document.querySelectorAll('[data-tab]')),
    panes: Array.from(document.querySelectorAll('[data-panel]')),
    // General
    statusDevice: $('statusDevice'),
    statusTyping: $('statusTyping'),
    statusVersion: $('statusVersion'),
    pauseToggle: $('pauseToggle'),
    statChars: $('statChars'),
    statWpm: $('statWpm'),
    statLatency: $('statLatency'),
    statUptime: $('statUptime'),
    statCpu: $('statCpu'),
    statMemory: $('statMemory'),
    statCorrections: $('statCorrections'),
    exclusionList: $('exclusionList'),
    exclusionForm: $('exclusionForm'),
    exclusionInput: $('exclusionInput'),
    resetGeneral: $('resetGeneral'),
    resetAll: $('resetAll'),
    // Languages
    langSummary: $('langSummary'),
    langList: $('langList'),
    rescanLangs: $('rescanLangs'),
    resetLangs: $('resetLangs'),
    // Keymap
    mapLang: $('mapLang'),
    mapLayer: $('mapLayer'),
    mapGroups: $('mapGroups'),
    mapGrid: $('mapGrid'),
    mapHint: $('mapHint'),
    mapResetGroup: $('mapResetGroup'),
    mapResetLang: $('mapResetLang'),
    // Practice
    benchLang: $('benchLang'),
    benchTarget: $('benchTarget'),
    benchInput: $('benchInput'),
    benchElapsed: $('benchElapsed'),
    benchWpm: $('benchWpm'),
    benchErrors: $('benchErrors'),
    benchRate: $('benchRate'),
    benchStart: $('benchStart'),
    benchClear: $('benchClear'),
    // HUD
    slider: $('textScale'),
    sliderValue: $('textScaleValue'),
    ringSlider: $('ringScale'),
    ringValue: $('ringScaleValue'),
    radios: Array.from(document.querySelectorAll('input[name="font"]')),
    reset: $('reset'),
    // shared
    status: $('status'),
  };

  let config = null;
  let status = null;
  let langs = [];
  let keymap = null;
  let mapLang = 'en';
  let mapLayer = 1;
  let mapGroup = 0;
  let activeTab = 'general';
  let languagesLoaded = false;
  let saveTimer = 0;
  let statusTimer = 0;
  const previewOnly = !invoke;

  /* ---------------- helpers ---------------- */

  function ui() {
    if (!config || typeof config !== 'object' || Array.isArray(config)) config = { version: 1 };
    if (!config.ui || typeof config.ui !== 'object' || Array.isArray(config.ui)) config.ui = {};
    return config.ui;
  }

  function textScale() {
    const value = ui().hudTextScale;
    return Number.isFinite(value) && value >= 0.8 && value <= 2 ? value : DEFAULT_UI.hudTextScale;
  }

  function ringScale() {
    const value = ui().hudScale;
    return Number.isFinite(value) && value >= 0.5 && value <= 3 ? value : DEFAULT_UI.hudScale;
  }

  function fontId() {
    const value = ui().hudFontFamily;
    return window.RniFonts.ids.includes(value) ? value : DEFAULT_UI.hudFontFamily;
  }

  function exclusions() {
    const list = config && config.system && config.system.exclusion;
    return Array.isArray(list) ? list.filter((item) => typeof item === 'string') : [];
  }

  function setExclusions(next) {
    if (!config || typeof config !== 'object') config = { version: 1 };
    if (!config.system || typeof config.system !== 'object') config.system = {};
    config.system.exclusion = next;
  }

  function setStatus(message, kind = '') {
    dom.status.textContent = message;
    dom.status.className = `status status--global${kind ? ` is-${kind}` : ''}`;
    clearTimeout(statusTimer);
    if (message) statusTimer = setTimeout(() => setStatus(''), 2400);
  }

  function applyPreview(scale, font) {
    const root = document.documentElement;
    root.style.setProperty('--preview-text-scale', scale.toFixed(2));
    root.style.setProperty('--preview-font', window.RniFonts.stacks[font]);
  }

  function makeBadge(text, variant = '') {
    const badge = document.createElement('span');
    badge.className = `badge${variant ? ` badge--${variant}` : ''}`;
    badge.textContent = text;
    return badge;
  }

  /* ---------------- tabs ---------------- */

  function selectTab(name) {
    activeTab = name;
    dom.nav.forEach((button) => {
      const on = button.dataset.tab === name;
      button.classList.toggle('is-active', on);
      if (on) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    });
    dom.panes.forEach((pane) => {
      pane.hidden = pane.dataset.panel !== name;
    });
    if (name === 'languages' && !languagesLoaded) rescan();
    if (name === 'keymap') loadKeymap();
    if (name === 'practice' && dom.benchLang.options.length === 0) renderBenchLanguages();
    if (name === 'general') refreshStats();
  }

  dom.nav.forEach((button) => {
    button.addEventListener('click', () => selectTab(button.dataset.tab));
  });

  /* ---------------- General ---------------- */

  function renderStatus() {
    if (!status) return;
    const connected = Boolean(status.connected);
    dom.statusDevice.textContent = connected
      ? status.device || 'Controller'
      : 'No controller connected';
    dom.statusTyping.textContent = status.paused ? 'Paused' : 'Active';
    dom.statusVersion.textContent = status.version || '—';
    dom.pauseToggle.checked = Boolean(status.paused);
  }

  function formatMs(micros) {
    if (!Number.isFinite(micros)) return '—';
    const ms = micros / 1000;
    if (ms < 1) return `${ms.toFixed(3)} ms`;
    return ms < 10 ? `${ms.toFixed(2)} ms` : `${ms.toFixed(1)} ms`;
  }

  function formatUptime(secs) {
    if (!Number.isFinite(secs)) return '—';
    const total = Math.max(0, Math.floor(secs));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = total % 60;
    const pad = (n) => String(n).padStart(2, '0');
    if (hours > 0) return `${hours}h ${pad(minutes)}m ${pad(seconds)}s`;
    if (minutes > 0) return `${minutes}m ${pad(seconds)}s`;
    return `${seconds}s`;
  }

  function renderStats(data) {
    if (!data || typeof data !== 'object') return;
    dom.statChars.textContent = String(data.chars ?? 0);
    dom.statWpm.textContent = Number.isFinite(data.wpm) ? data.wpm.toFixed(1) : '0.0';
    if (Number.isFinite(data.stepAvgUs)) {
      dom.statLatency.textContent = `${formatMs(data.stepAvgUs)} / ${formatMs(data.stepMaxUs)}`;
    }
    dom.statUptime.textContent = formatUptime(data.uptimeSecs);
    if (Number.isFinite(data.cpuPercent)) {
      dom.statCpu.textContent = `${data.cpuPercent.toFixed(1)}%`;
    }
    if (Number.isFinite(data.rssBytes)) {
      dom.statMemory.textContent = `${Math.round(data.rssBytes / (1024 * 1024))} MB`;
    }
    if (Number.isFinite(data.corrections)) {
      dom.statCorrections.textContent = String(data.corrections);
    }
  }

  async function refreshStats() {
    if (previewOnly || activeTab !== 'general' || !invoke) return;
    try {
      const result = await invoke('get_stats');
      if (result && result.ok === true) renderStats(result.data);
    } catch {
      /* ยังไม่มี runtime loop — ปล่อยค่าเดิม */
    }
  }

  setInterval(refreshStats, 1000);

  function renderExclusions() {    const list = exclusions();
    dom.exclusionList.replaceChildren();
    if (list.length === 0) {
      const empty = document.createElement('li');
      empty.className = 'list__empty';
      empty.textContent = 'No apps excluded. Every app receives typed keys.';
      dom.exclusionList.append(empty);
      return;
    }
    list.forEach((name, index) => {
      const row = document.createElement('li');
      row.className = 'list__row';
      const label = document.createElement('span');
      label.className = 'list__name';
      label.textContent = name;
      const remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'btn btn--small';
      remove.textContent = 'Remove';
      remove.addEventListener('click', () => {
        const next = exclusions();
        next.splice(index, 1);
        setExclusions(next);
        renderExclusions();
        scheduleSave();
      });
      row.append(label, remove);
      dom.exclusionList.append(row);
    });
  }

  dom.exclusionForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const name = dom.exclusionInput.value.trim();
    if (!name) return;
    const list = exclusions();
    if (list.some((item) => item.toLowerCase() === name.toLowerCase())) {
      setStatus(`"${name}" is already excluded`, 'error');
      return;
    }
    list.push(name);
    setExclusions(list);
    dom.exclusionInput.value = '';
    renderExclusions();
    scheduleSave();
  });

  dom.pauseToggle.addEventListener('change', async () => {
    const paused = dom.pauseToggle.checked;
    if (!invoke) {
      status = { ...(status || {}), paused };
      renderStatus();
      return;
    }
    try {
      const result = await invoke('set_paused', { paused });
      if (result && result.ok === true) {
        status = { ...(status || {}), ...result.data };
        renderStatus();
      } else {
        dom.pauseToggle.checked = !paused;
        setStatus('Could not change pause state', 'error');
      }
    } catch (err) {
      dom.pauseToggle.checked = !paused;
      setStatus(`Pause failed: ${err && err.message ? err.message : err}`, 'error');
    }
  });

  /* ---------------- Languages ---------------- */

  function renderLanguages() {
    const detected = langs.filter((row) => row.detected);
    const names = detected.map((row) => row.label.split(' (')[0]);
    dom.langSummary.textContent = detected.length
      ? `Detected on this PC: ${names.join(', ')}.`
      : 'No keyboard layout detected — using English only.';

    dom.langList.replaceChildren();
    if (langs.length === 0) {
      const empty = document.createElement('li');
      empty.className = 'lang__empty';
      empty.textContent = 'Press “Rescan languages” to look for installed layouts.';
      dom.langList.append(empty);
      return;
    }

    langs.forEach((row) => {
      const item = document.createElement('li');
      item.className = `lang${row.supported ? '' : ' is-unsupported'}`;

      const code = document.createElement('span');
      code.className = 'lang__code';
      code.textContent = String(row.code || '?').toUpperCase().slice(0, 4);

      const body = document.createElement('span');
      body.className = 'lang__body';
      const name = document.createElement('span');
      name.className = 'lang__name';
      name.textContent = row.label || row.code;
      const meta = document.createElement('span');
      meta.className = 'lang__meta';
      if (row.builtin) {
        meta.append(makeBadge('Built-in', 'ok'));
      } else {
        meta.append(makeBadge(row.detected ? 'Detected' : 'Not found', row.detected ? 'ok' : 'warn'));
      }
      meta.append(makeBadge(row.kind === 'ime' ? 'IME · keys' : 'Layout', row.kind === 'ime' ? 'ime' : ''));
      if (!row.supported) meta.append(makeBadge('Needs mapping', 'warn'));
      body.append(name, meta);

      const toggle = document.createElement('label');
      toggle.className = 'switch';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = Boolean(row.enabled);
      input.disabled = !row.supported || Boolean(row.locked);
      input.setAttribute('aria-label', `Enable ${row.label || row.code}`);
      input.addEventListener('change', () => toggleLanguage(row, input));
      const track = document.createElement('span');
      track.className = 'switch__track';
      track.setAttribute('aria-hidden', 'true');
      toggle.append(input, track);

      item.append(code, body, toggle);
      dom.langList.append(item);
    });
  }

  async function toggleLanguage(row, input) {
    const enabling = input.checked;
    if (!invoke) {
      row.enabled = enabling;
      renderLanguages();
      return;
    }
    if (enabling) {
      const enabled = langs.filter((item) => item.enabled).length;
      if (enabled >= MAX_ENABLED_LANGUAGES) {
        input.checked = false;
        setStatus(`Up to ${MAX_ENABLED_LANGUAGES} languages can be enabled`, 'error');
        return;
      }
    }
    try {
      const result = await invoke('set_language_enabled', { code: row.code, enabled: enabling });
      if (result && result.ok === true) {
        config = result.data || config;
        row.enabled = enabling;
        renderLanguages();
        setStatus(enabling ? `${row.code.toUpperCase()} enabled` : `${row.code.toUpperCase()} disabled`);
      } else {
        input.checked = !enabling;
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Not saved: ${message}`, 'error');
      }
    } catch (err) {
      input.checked = !enabling;
      setStatus(`Not saved: ${err && err.message ? err.message : err}`, 'error');
    }
  }

  async function rescan() {
    dom.rescanLangs.disabled = true;
    try {
      if (!invoke) {
        langs = PREVIEW_LANGS.map((row) => ({ ...row }));
        languagesLoaded = true;
        renderLanguages();
        return;
      }
      const result = await invoke('rescan_languages');
      if (result && result.ok === true) {
        langs = Array.isArray(result.data) ? result.data : [];
        languagesLoaded = true;
        renderLanguages();
      } else {
        setStatus('Could not scan languages', 'error');
      }
    } catch (err) {
      setStatus(`Scan failed: ${err && err.message ? err.message : err}`, 'error');
    } finally {
      dom.rescanLangs.disabled = false;
    }
  }

  dom.rescanLangs.addEventListener('click', () => {
    languagesLoaded = false;
    rescan();
  });
  dom.resetLangs.addEventListener('click', () => resetSection('languages'));

  /* ---------------- Keymap ---------------- */

  const SECTOR_POS = ['1 / 2', '1 / 3', '2 / 3', '3 / 3', '3 / 2', '3 / 1', '2 / 1', '1 / 1'];

  function mapLanguageOptions() {
    const editable = langs.filter((row) => row.supported);
    if (editable.length > 0) return editable;
    return [
      { code: 'en', label: 'English' },
      { code: 'th', label: 'Thai' },
    ];
  }

  function renderMapLanguages() {
    const options = mapLanguageOptions();
    if (!options.some((row) => row.code === mapLang)) mapLang = options[0]?.code || 'en';
    dom.mapLang.replaceChildren();
    options.forEach((row) => {
      const option = document.createElement('option');
      option.value = row.code;
      option.textContent = `${String(row.code).toUpperCase()} · ${row.label || row.code}`;
      option.selected = row.code === mapLang;
      dom.mapLang.append(option);
    });
  }

  function mapLayerData() {
    if (!keymap || !Array.isArray(keymap.layers)) return null;
    return keymap.layers[mapLayer - 1] || null;
  }

  function renderKeymap() {
    if (!keymap) return;
    const groups = (mapLayer === 1 ? keymap.groups : keymap.groups2) || [];
    dom.mapGroups.replaceChildren();
    groups.forEach((name, index) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = `keymap__group${index === mapGroup ? ' is-on' : ''}`;
      button.textContent = `${index + 1}. ${name}`;
      button.setAttribute('role', 'option');
      button.setAttribute('aria-selected', index === mapGroup ? 'true' : 'false');
      button.addEventListener('click', () => {
        mapGroup = index;
        renderKeymap();
      });
      dom.mapGroups.append(button);
    });

    const layer = mapLayerData();
    dom.mapGrid.replaceChildren();
    for (let sector = 0; sector < 8; sector++) {
      const index = mapGroup * 8 + sector;
      const value = layer?.cells?.[index] ?? '';
      const isDefault = value === (layer?.defaults?.[index] ?? '');
      const cell = document.createElement('button');
      cell.type = 'button';
      cell.className = `cell${value === '' ? ' is-empty' : ''}${isDefault ? '' : ' is-modified'}`;
      cell.style.gridArea = SECTOR_POS[sector];
      cell.dataset.value = value;
      cell.textContent = value === '' ? '—' : value;
      cell.title = value === '' ? 'Empty cell' : `${value} — click to change`;
      cell.addEventListener('click', () => startCellEdit(cell, mapGroup, sector));
      dom.mapGrid.append(cell);
    }

    const center = document.createElement('div');
    center.className = 'cell cell--center';
    center.style.gridArea = '2 / 2';
    center.textContent = groups[mapGroup] || '';
    dom.mapGrid.append(center);

    const modified = layer
      ? layer.cells.reduce((count, value, index) => count + (value !== layer.defaults[index] ? 1 : 0), 0)
      : 0;
    dom.mapHint.textContent = modified > 0
      ? `${modified} changed cell${modified === 1 ? '' : 's'} in this layer. Click a cell to edit; Enter saves, Esc cancels.`
      : 'Click a cell to assign a key. Enter saves, Esc cancels.';
  }

  function startCellEdit(cell, group, sector) {
    if (cell.querySelector('input')) return;
    const previous = cell.dataset.value || '';
    const input = document.createElement('input');
    input.type = 'text';
    input.maxLength = 2;
    input.value = previous;
    cell.textContent = '';
    cell.append(input);
    input.focus();
    input.select();

    let done = false;
    const finish = (commit) => {
      if (done) return;
      done = true;
      const value = commit ? input.value : previous;
      saveCell(group, sector, value);
    };
    input.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        finish(true);
      } else if (event.key === 'Escape') {
        event.preventDefault();
        finish(false);
      }
    });
    input.addEventListener('blur', () => finish(true));
  }

  async function saveCell(group, sector, value) {
    if (mapLayer === 1 && value === '') {
      setStatus('Base layer needs a key in every cell', 'error');
      renderKeymap();
      return;
    }
    if (!invoke) {
      const layer = mapLayerData();
      if (layer) {
        const index = group * 8 + sector;
        layer.cells[index] = value;
        const key = `${mapLayer}-${group}-${sector}`;
        if (value === layer.defaults[index]) delete keymap.overrides[key];
        else keymap.overrides[key] = value;
      }
      renderKeymap();
      setStatus('Saved (preview)');
      return;
    }

    try {
      const result = await invoke('set_keymap_cell', {
        code: mapLang,
        layer: mapLayer,
        group,
        sector,
        text: value,
      });
      if (result && result.ok === true) {
        keymap = result.data;
        renderKeymap();
        setStatus('Keymap saved');
      } else {
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Not saved: ${message}`, 'error');
        renderKeymap();
      }
    } catch (err) {
      setStatus(`Not saved: ${err && err.message ? err.message : err}`, 'error');
      renderKeymap();
    }
  }

  async function loadKeymap() {
    renderMapLanguages();
    if (!invoke) {
      keymap = previewKeymap();
      renderKeymap();
      return;
    }
    try {
      const result = await invoke('get_keymap', { code: mapLang });
      if (result && result.ok === true) {
        keymap = result.data;
        renderKeymap();
      } else {
        keymap = null;
        setStatus('Keymap is not available for this language', 'error');
      }
    } catch (err) {
      setStatus(`Could not load keymap: ${err && err.message ? err.message : err}`, 'error');
    }
  }

  async function resetMap(group) {
    if (!invoke) {
      const layer = mapLayerData();
      if (layer) {
        for (let sector = 0; sector < 8; sector++) {
          const index = mapGroup * 8 + sector;
          layer.cells[index] = layer.defaults[index];
          delete keymap.overrides[`${mapLayer}-${mapGroup}-${sector}`];
        }
      }
      renderKeymap();
      setStatus('Reset (preview)');
      return;
    }
    try {
      const result = await invoke('reset_keymap', { code: mapLang, group });
      if (result && result.ok === true) {
        keymap = result.data;
        renderKeymap();
        setStatus(group === undefined || group === null ? 'Language reset' : 'Group reset');
      } else {
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Reset failed: ${message}`, 'error');
      }
    } catch (err) {
      setStatus(`Reset failed: ${err && err.message ? err.message : err}`, 'error');
    }
  }

  dom.mapLang.addEventListener('change', () => {
    mapLang = dom.mapLang.value;
    keymap = null;
    loadKeymap();
  });

  dom.mapLayer.addEventListener('click', (event) => {
    const button = event.target.closest('button[data-layer]');
    if (!button) return;
    mapLayer = Number(button.dataset.layer);
    dom.mapLayer.querySelectorAll('button').forEach((item) => {
      item.classList.toggle('is-on', Number(item.dataset.layer) === mapLayer);
    });
    renderKeymap();
  });

  dom.mapResetGroup.addEventListener('click', () => resetMap(mapGroup));
  dom.mapResetLang.addEventListener('click', () => resetMap(null));

  /* ---------------- Practice / error rate ---------------- */

  const PHRASES = {
    en: 'the quick brown fox jumps over the lazy dog',
    th: 'การพิมพ์ด้วยจอยต้องใช้ความคุ้นเคยและฝึกฝนอย่างสม่ำเสมอ',
    ru: 'быстрая коричневая лиса прыгает через ленивую собаку',
    el: 'η γρήγορη καφέ αλεπού πηδάει πάνω από τον τεμπέλη σκύλο',
  };

  let benchStartedAt = 0;
  let benchFinished = true;
  let benchTimer = 0;

  function editDistance(a, b) {
    if (a === b) return 0;
    const left = Array.from(a);
    const right = Array.from(b);
    let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
    for (let i = 1; i <= left.length; i++) {
      const current = [i];
      for (let j = 1; j <= right.length; j++) {
        const cost = left[i - 1] === right[j - 1] ? 0 : 1;
        current[j] = Math.min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + cost);
      }
      previous = current;
    }
    return previous[right.length];
  }

  function benchTargetText() {
    return PHRASES[dom.benchLang.value] || PHRASES.en;
  }

  function renderBench() {
    dom.benchTarget.textContent = benchTargetText();
    const typed = dom.benchInput.value;
    const target = benchTargetText();
    const errors = editDistance(typed, target);
    const rate = (errors / Math.max(Array.from(target).length, 1)) * 100;
    const elapsed = benchStartedAt ? (performance.now() - benchStartedAt) / 1000 : 0;
    const words = Array.from(typed).length / 5;
    const wpm = elapsed > 0.5 ? words / (elapsed / 60) : 0;

    dom.benchElapsed.textContent = `${elapsed.toFixed(1)} s`;
    dom.benchWpm.textContent = wpm.toFixed(1);
    dom.benchErrors.textContent = String(errors);
    dom.benchRate.textContent = `${Math.min(rate, 100).toFixed(1)}%`;

    if (!benchFinished && typed === target) finishBench(typed, target, errors, rate, wpm, elapsed);
  }

  async function finishBench(typed, target, errors, rate, wpm, elapsed) {
    benchFinished = true;
    clearInterval(benchTimer);
    benchTimer = 0;
    dom.benchElapsed.textContent = `${elapsed.toFixed(1)} s`;
    dom.benchWpm.textContent = wpm.toFixed(1);
    dom.benchRate.textContent = `${Math.min(rate, 100).toFixed(1)}%`;
    setStatus('Practice recorded');

    if (previewOnly || !invoke) return;
    try {
      const result = await invoke('record_bench', {
        lang: dom.benchLang.value,
        target,
        typed,
        errors,
        errorPercent: Math.min(rate, 100),
        wpm,
        seconds: elapsed,
      });
      if (!result || result.ok !== true) {
        setStatus('Could not record practice result', 'error');
      }
    } catch (err) {
      setStatus(`Could not record: ${err && err.message ? err.message : err}`, 'error');
    }
  }

  function startBench() {
    clearInterval(benchTimer);
    dom.benchInput.value = '';
    benchFinished = false;
    benchStartedAt = performance.now();
    renderBench();
    dom.benchInput.focus();
    benchTimer = setInterval(renderBench, 200);
  }

  dom.benchLang.addEventListener('change', () => {
    benchFinished = true;
    benchStartedAt = 0;
    clearInterval(benchTimer);
    benchTimer = 0;
    dom.benchInput.value = '';
    renderBench();
    dom.benchTarget.textContent = benchTargetText();
  });

  dom.benchInput.addEventListener('input', renderBench);
  dom.benchStart.addEventListener('click', startBench);
  dom.benchClear.addEventListener('click', () => {
    benchFinished = true;
    benchStartedAt = 0;
    clearInterval(benchTimer);
    benchTimer = 0;
    dom.benchInput.value = '';
    renderBench();
  });

  function renderBenchLanguages() {
    dom.benchLang.replaceChildren();
    Object.keys(PHRASES).forEach((code) => {
      const option = document.createElement('option');
      option.value = code;
      option.textContent = code.toUpperCase();
      dom.benchLang.append(option);
    });
    if (langs.some((row) => row.code === 'th')) dom.benchLang.value = 'th';
    dom.benchTarget.textContent = benchTargetText();
  }

  /* ---------------- HUD ---------------- */

  function renderHud() {
    const scale = textScale();
    const font = fontId();
    const ring = ringScale();

    dom.slider.value = String(scale);
    dom.sliderValue.textContent = `${scale.toFixed(2)}×`;
    dom.ringSlider.value = String(ring);
    dom.ringValue.textContent = `${ring.toFixed(2)}×`;
    dom.radios.forEach((radio) => {
      radio.checked = radio.value === font;
    });
    applyPreview(scale, font);
  }

  dom.slider.addEventListener('input', () => {
    const scale = Number(dom.slider.value);
    ui().hudTextScale = scale;
    dom.sliderValue.textContent = `${scale.toFixed(2)}×`;
    applyPreview(scale, fontId());
    scheduleSave();
  });

  dom.ringSlider.addEventListener('input', () => {
    const ring = Number(dom.ringSlider.value);
    ui().hudScale = ring;
    dom.ringValue.textContent = `${ring.toFixed(2)}×`;
    scheduleSave();
  });

  dom.radios.forEach((radio) => {
    radio.addEventListener('change', () => {
      if (!radio.checked) return;
      ui().hudFontFamily = radio.value;
      applyPreview(textScale(), radio.value);
      scheduleSave();
    });
  });

  dom.reset.addEventListener('click', () => resetSection('hud'));
  dom.resetGeneral.addEventListener('click', () => resetSection('general'));
  dom.resetAll.addEventListener('click', () => resetSection('all'));

  /* ---------------- save / reset ---------------- */

  async function save() {
    if (!invoke) {
      setStatus('Preview only — open in the app to save', 'error');
      return;
    }

    dom.status.textContent = 'Saving…';
    dom.status.className = 'status status--global';

    try {
      const result = await invoke('save_config', { config });
      if (!result || result.ok !== true) {
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Not saved: ${message}`, 'error');
        console.error('[settings] save_config:', result);
        return;
      }
      config = result.data || config;
      renderAll();
      setStatus('Saved');
    } catch (err) {
      setStatus(`Not saved: ${err && err.message ? err.message : err}`, 'error');
      console.error('[settings] save_config failed', err);
    }
  }

  function scheduleSave() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, SAVE_DELAY_MS);
  }

  async function resetSection(section) {
    if (!invoke) {
      if (section === 'languages') {
        langs = PREVIEW_LANGS.map((row) => ({
          ...row,
          enabled: row.code === 'en' || row.code === 'th',
        }));
        renderLanguages();
      } else {
        const defaults = {
          hud: { hudTextScale: DEFAULT_UI.hudTextScale, hudScale: DEFAULT_UI.hudScale, hudFontFamily: DEFAULT_UI.hudFontFamily },
        };
        if (section === 'hud' || section === 'all') Object.assign(ui(), defaults.hud);
        if (section === 'general' || section === 'all') {
          if (!config.system || typeof config.system !== 'object') config.system = {};
          config.system.exclusion = [];
        }
        renderAll();
      }
      setStatus('Reset to defaults (preview)');
      return;
    }

    try {
      const result = await invoke('reset_config', { section });
      if (result && result.ok === true) {
        config = result.data || config;
        if (section === 'languages' || section === 'all') languagesLoaded = false;
        renderAll();
        if (activeTab === 'languages') rescan();
        setStatus('Reset to defaults');
      } else {
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Reset failed: ${message}`, 'error');
      }
    } catch (err) {
      setStatus(`Reset failed: ${err && err.message ? err.message : err}`, 'error');
    }
  }

  /* ---------------- render ---------------- */

  function renderAll() {
    renderHud();
    renderExclusions();
    renderStatus();
    if (languagesLoaded) renderLanguages();
  }

  /* ---------------- boot ---------------- */

  async function boot() {
    if (!invoke) {
      status = { device: 'Xbox Wireless Controller', connected: true, paused: false, admin: false, version: '0.1.0 (preview)' };
      renderAll();
      setStatus('Preview only — open in the app to save');
      return;
    }

    try {
      const [configResult, statusResult] = await Promise.all([
        invoke('get_config'),
        invoke('get_status'),
      ]);
      if (configResult && configResult.ok === true && configResult.data) {
        config = configResult.data;
      } else {
        console.warn('[settings] get_config:', configResult);
        setStatus('Could not load config', 'error');
      }
      if (statusResult && statusResult.ok === true) {
        status = statusResult.data;
      }
    } catch (err) {
      console.error('[settings] boot failed', err);
      setStatus('Could not load settings', 'error');
    }

    renderAll();
    setStatus('');
    // เตรียมรายการภาษาไว้เลย — เปิดแท็บ Languages แล้วเห็นทันที (และ log ยืนยันว่า scan ได้)
    rescan();
    refreshStats();

    // ค่าเปลี่ยนจากที่อื่น (tray / profile / อีกหน้าต่าง) → ตามให้ทัน
    if (listen) {
      listen('settings:config', (message) => {
        const payload = message && typeof message === 'object' && 'payload' in message
          ? message.payload
          : message;
        if (!payload || typeof payload !== 'object') return;
        config = payload;
        renderAll();
      }).catch(() => {});

      listen('app:status', (message) => {
        const payload = message && typeof message === 'object' && 'payload' in message
          ? message.payload
          : message;
        if (!payload || typeof payload !== 'object') return;
        status = payload;
        renderStatus();
      }).catch(() => {});
    }

    selectTab(activeTab);
  }

  if (!previewOnly) dom.status.textContent = 'Loading…';
  boot();
})();
