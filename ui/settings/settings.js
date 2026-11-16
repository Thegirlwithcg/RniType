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

  function renderExclusions() {
    const list = exclusions();
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
