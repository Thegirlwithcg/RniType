/* ============================================================
   RniType — Settings window controller (P5 slice 1 · HUD tab)
   ------------------------------------------------------------
   อ่าน/เขียน config ผ่าน ipc.md §3: get_config / save_config
   ทุกครั้งที่บันทึก แอปจะ push `hud:style` ให้ HUD → เห็นผลทันที
   ============================================================ */
(() => {
  'use strict';

  const DEFAULT_UI = {
    hudTextScale: 1.4,
    hudScale: 1.0,
    hudFontFamily: window.RniFonts.default,
  };
  const SAVE_DELAY_MS = 200;

  const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
  const invoke =
    tauri && tauri.core && typeof tauri.core.invoke === 'function'
      ? tauri.core.invoke.bind(tauri.core)
      : null;

  const dom = {
    slider: document.getElementById('textScale'),
    sliderValue: document.getElementById('textScaleValue'),
    ringSlider: document.getElementById('ringScale'),
    ringValue: document.getElementById('ringScaleValue'),
    radios: Array.from(document.querySelectorAll('input[name="font"]')),
    reset: document.getElementById('reset'),
    status: document.getElementById('status'),
  };

  let config = null;
  let saveTimer = 0;
  let statusTimer = 0;
  let previewOnly = !invoke;

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

  function setStatus(message, kind = '') {
    dom.status.textContent = message;
    dom.status.className = `status${kind ? ` is-${kind}` : ''}`;
    clearTimeout(statusTimer);
    if (message) statusTimer = setTimeout(() => setStatus(''), 2000);
  }

  function applyPreview(scale, font) {
    const root = document.documentElement;
    root.style.setProperty('--preview-text-scale', scale.toFixed(2));
    root.style.setProperty('--preview-font', window.RniFonts.stacks[font]);
  }

  function render() {
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

  /* ---------------- save ---------------- */

  async function save() {
    if (!invoke) {
      setStatus('Preview only — open in the app to save', 'error');
      return;
    }

    dom.status.textContent = 'Saving…';
    dom.status.className = 'status';

    try {
      const result = await invoke('save_config', { config });
      if (!result || result.ok !== true) {
        const message = (result && result.error && result.error.message) || 'unknown error';
        setStatus(`Not saved: ${message}`, 'error');
        console.error('[settings] save_config:', result);
        return;
      }
      config = result.data || config;
      render();
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

  /* ---------------- events ---------------- */

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

  dom.reset.addEventListener('click', () => {
    const uiSection = ui();
    uiSection.hudTextScale = DEFAULT_UI.hudTextScale;
    uiSection.hudScale = DEFAULT_UI.hudScale;
    uiSection.hudFontFamily = DEFAULT_UI.hudFontFamily;
    render();
    save();
  });

  /* ---------------- boot ---------------- */

  async function boot() {
    if (!invoke) {
      previewOnly = true;
      render();
      setStatus('Preview only — open in the app to save');
      return;
    }

    try {
      const result = await invoke('get_config');
      if (result && result.ok === true && result.data) {
        config = result.data;
      } else {
        console.warn('[settings] get_config:', result);
        setStatus('Could not load config', 'error');
      }
    } catch (err) {
      console.error('[settings] get_config failed', err);
      setStatus('Could not load config', 'error');
    }

    render();
    setStatus('');

    // ค่าเปลี่ยนจากที่อื่น (tray / HUD / profile) → ตามให้ทัน
    if (tauri.event && typeof tauri.event.listen === 'function') {
      tauri.event
        .listen('settings:config', (message) => {
          const payload = message && typeof message === 'object' && 'payload' in message
            ? message.payload
            : message;
          if (!payload || typeof payload !== 'object') return;
          config = payload;
          render();
        })
        .catch(() => {});
    }
  }

  if (!previewOnly) dom.status.textContent = 'Loading…';
  boot();
})();
