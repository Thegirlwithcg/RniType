/* ============================================================
   RniType — HUD overlay renderer (P4 Track C, slice 1)
   ------------------------------------------------------------
   วาดจาก HudState เท่านั้น (อ่านอย่างเดียว · คลิกทะลุ)
   ไม่มีแป้น / gesture / keymap / simulation — ตัดสินใจทั้งหมดอยู่ที่ฝั่งแอป

   แหล่ง state:
     • มี window.__TAURI__  → ฟัง event 'hud:state' (Tauri v2) และไม่ fetch
     • ไม่มี                → poll /api/state ทุก ~100 ms (state player สำหรับ dev)
   ============================================================ */
(() => {
  'use strict';

  const NS = 'http://www.w3.org/2000/svg';
  const POLL_MS = 100;
  const MARGIN = 172; // รัศมีวง HUD/2 + ขอบจอ (ADR-11: clamp pivot ไม่ให้ล้นจอ)

  const dom = {
    hud: document.getElementById('hud'),
    stage: document.getElementById('stage'),
    inner: document.getElementById('innerRing'),
    outer: document.getElementById('outerRing'),
    cmd: document.getElementById('cmdRing'),
    lang: document.getElementById('langRing'),
    previewChar: document.getElementById('previewChar'),
    previewCount: document.getElementById('previewCount'),
    pillMode: document.getElementById('pillMode'),
    pillText: document.getElementById('pillText'),
    pillLang: document.getElementById('pillLang'),
    pillBadges: document.getElementById('pillBadges'),
    pointer: document.getElementById('pointer'),
    pivot: document.getElementById('pivot'),
  };

  // รัศมีวง (หน่วยเดียวกับ viewBox -170..170)
  // v0.12 (ADR-17): วงในกว้างขึ้น (32–88) และข้อความอยู่ที่ r=61 เพื่อให้ป้ายไทยยาว ๆ มีที่
  // วงนอก (ตัวอักษร) 94–158 · text อยู่ r=126
  const R = {
    groupIn: 33, groupOut: 90, groupText: 63,
    charIn: 96, charOut: 158, charText: 127,
    wheelIn: 36, wheelOut: 112, wheelText: 76,
  };

  const TYPE_MODES = ['type-en', 'type-th', 'type-other'];
  const RING_STATES = ['group', 'char', 'commit', 'repeat'];

  /* ---------------- typography (ADR-16) ----------------
     ตัวหนังสือ = style จากแอป (event `hud:style` / command `hud_style`)
     ไม่ใช่ state รายเฟรม จึงไม่ปนกับ HudState — ค่าเริ่มต้นตรงกับ config.schema */
  const FONT_STACKS = window.RniFonts.stacks;
  const STYLE_DEFAULT = { textScale: 1.4, font: window.RniFonts.default };
  /** ค่าเริ่มต้นขนาดวง (config ui.hudScale) — ใช้เมื่อ state ไม่ส่งมา */
  const SCALE_DEFAULT = 1.0;
  const typography = { ...STYLE_DEFAULT };

  const MODE_LABEL = {
    cmd: 'Command',
    lang: 'Language',
    mouse: 'Mouse',
    nav: 'Place UI',
    disabled: 'Paused',
  };

  /* ---------------- helpers ---------------- */

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  function num(value, fallback, lo, hi) {
    const n = typeof value === 'number' && Number.isFinite(value) ? value : fallback;
    return clamp(n, lo, hi);
  }

  function text(value) {
    return typeof value === 'string' ? value : '';
  }

  function svg(name, attrs) {
    const node = document.createElementNS(NS, name);
    for (const key in attrs) node.setAttribute(key, attrs[key]);
    return node;
  }

  function clear(node) {
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  /** payload ของ Tauri event (v2 ห่อด้วย {event, id, payload}) */
  function payloadOf(message) {
    return message && typeof message === 'object' && 'payload' in message
      ? message.payload
      : message;
  }

  /** style ของตัวหนังสือ — มาจากแอปเท่านั้น (HUD ไม่เดาเอง) */
  function applyStyle(raw) {
    const style = raw && typeof raw === 'object' ? raw : {};

    typography.textScale = num(style.textScale, STYLE_DEFAULT.textScale, 0.8, 2);
    const font = text(style.font);
    typography.font = Object.prototype.hasOwnProperty.call(FONT_STACKS, font)
      ? font
      : STYLE_DEFAULT.font;

    const root = document.documentElement;
    root.style.setProperty('--hud-text-scale', typography.textScale.toFixed(2));
    root.style.setProperty('--hud-font-family', FONT_STACKS[typography.font]);
  }

  function sectorPath(rIn, rOut, a0, a1) {
    const gap = 0.018;
    const s0 = a0 + gap;
    const s1 = a1 - gap;
    const pt = (r, a) => `${(Math.cos(a) * r).toFixed(2)} ${(Math.sin(a) * r).toFixed(2)}`;
    return (
      `M ${pt(rOut, s0)} A ${rOut} ${rOut} 0 0 1 ${pt(rOut, s1)} ` +
      `L ${pt(rIn, s1)} A ${rIn} ${rIn} 0 0 0 ${pt(rIn, s0)} Z`
    );
  }

  function labelLength(value) {
    return Array.from(value).length;
  }

  /* ---------------- Thai text helpers ---------------- */

  /** สระล่าง 3 ตัว (ุ ู ฺ) — วาดขีดกำกับเหนือตัวอักษร (ADR-12) */
  const BELOW_MARKS = new Set(['\u0E38', '\u0E39', '\u0E3A']);

  /** ตัดข้อความเป็น grapheme cluster (ไม่แยกสระ/วรรณยุกต์ออกจากพยัญชนะ) */
  const SEGMENTER =
    typeof Intl !== 'undefined' && typeof Intl.Segmenter === 'function'
      ? new Intl.Segmenter('th', { granularity: 'grapheme' })
      : null;

  function graphemes(value) {
    if (!value) return [];
    return SEGMENTER
      ? [...SEGMENTER.segment(value)].map((part) => part.segment)
      : Array.from(value);
  }

  /** ป้ายกลุ่มที่ยาวตั้งแต่ 7 ตัว → แบ่ง 2 บรรทัด (กันชนกันในช่อง 45°) */
  const LABEL_TWO_LINES_AT = 7;

  /** เขียนข้อความลง <text> — 1 หรือ 2 บรรทัด (คืนจำนวนบรรทัด) */
  function renderLines(node, cx, cy, value) {
    const parts = graphemes(value);
    if (parts.length < LABEL_TWO_LINES_AT) {
      node.textContent = value;
      return 1;
    }

    const half = Math.ceil(parts.length / 2);
    [parts.slice(0, half), parts.slice(half)].forEach((line, index) => {
      const tspan = svg('tspan', { x: cx, dy: index === 0 ? '-0.55em' : '1.1em' });
      tspan.textContent = line.join('');
      node.appendChild(tspan);
    });
    return 2;
  }

  /** ป้ายกลุ่มยาว → ย่อตัวอักษรให้พอดีกับแถบวง (นับตาม grapheme ไม่ใช่ code point) */
  function lengthClass(value) {
    const len = graphemes(value).length;
    if (len <= 4) return '';
    if (len <= 6) return 'len-m';
    return 'len-l';
  }

  /** clamp pivot ให้วง HUD ยังอยู่ในจอ (ADR-11) — ตอนจริงแอป clamp มาแล้ว */
  function clampPivot(point, scale) {
    const margin = MARGIN * scale;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const rawX = num(point && point.x, w * 0.62, -1e6, 1e6);
    const rawY = num(point && point.y, h * 0.55, -1e6, 1e6);
    const x = w - margin < margin ? w / 2 : clamp(rawX, margin, w - margin);
    const y = h - margin < margin ? h / 2 : clamp(rawY, margin, h - margin);
    return { x, y };
  }

  /* ---------------- rings ---------------- */

  function renderGroupRing(ring) {
    const labels = Array.isArray(ring.labels) ? ring.labels : [];
    if (!labels.length) return;

    const step = (Math.PI * 2) / labels.length;
    const active = num(ring.active, -1, -1, 7);

    labels.forEach((label, i) => {
      const center = -Math.PI / 2 + i * step;
      const armed = active === i;

      const sector = svg('path', {
        d: sectorPath(R.groupIn, R.groupOut, center - step / 2, center + step / 2),
        class: `sector sector--group${armed ? ' is-armed' : ''}`,
      });
      dom.inner.appendChild(sector);

      const labelText = text(label.text);
      const twoLines = graphemes(labelText).length >= LABEL_TWO_LINES_AT;
      const node = svg('text', {
        class:
          'sector-label sector-label--group' +
          (armed ? ' is-armed' : '') +
          (active >= 0 && !armed ? ' is-muted' : '') +
          (twoLines ? ' is-two-line' : lengthClass(labelText) ? ` ${lengthClass(labelText)}` : ''),
        x: (Math.cos(center) * R.groupText).toFixed(2),
        y: (Math.sin(center) * R.groupText).toFixed(2),
      });
      renderLines(
        node,
        (Math.cos(center) * R.groupText).toFixed(2),
        (Math.sin(center) * R.groupText).toFixed(2),
        labelText,
      );
      dom.inner.appendChild(node);
    });
  }

  function renderCharRing(ring) {
    const labels = Array.isArray(ring.labels) ? ring.labels : [];
    if (!labels.length) return;

    const step = (Math.PI * 2) / labels.length;
    const active = num(ring.active, -1, -1, 7);

    labels.forEach((label, i) => {
      const center = -Math.PI / 2 + i * step;
      const selected = active === i;
      const textScale = typography.textScale;

      const sector = svg('path', {
        d: sectorPath(R.charIn, R.charOut, center - step / 2, center + step / 2),
        class: `sector sector--char${selected ? ' is-selected' : ''}`,
      });
      dom.outer.appendChild(sector);

      const cx = Math.cos(center) * R.charText;
      const cy = Math.sin(center) * R.charText;
      const labelText = text(label.text);

      // ขีดกำกับสระล่าง (ุ ู ฺ) = เส้นประสั้นเหนือตัวอักษร — ADR-12
      if (BELOW_MARKS.has(labelText)) {
        const line = cy - 6.5 * textScale;
        dom.outer.appendChild(
          svg('line', {
            class: `char-guide is-on${selected ? ' is-selected' : ''}`,
            x1: (cx - 8 * textScale).toFixed(2),
            x2: (cx + 8 * textScale).toFixed(2),
            y1: line.toFixed(2),
            y2: line.toFixed(2),
          }),
        );
      }

      const node = svg('text', {
        class: `char-text${selected ? ' is-selected' : ' is-dim'}`,
        x: cx.toFixed(2),
        y: cy.toFixed(2),
      });
      node.textContent = labelText;
      dom.outer.appendChild(node);

      // badge ของช่อง (เช่น IME) — ค่าตรง ๆ จาก contract
      if (text(label.badge)) {
        const badge = svg('text', {
          class: 'slot-badge',
          x: cx.toFixed(2),
          y: (cy + 16 * textScale).toFixed(2),
        });
        badge.textContent = text(label.badge);
        dom.outer.appendChild(badge);
      }
    });
  }

  function renderWheel(group, ring, kind) {
    const labels = Array.isArray(ring.labels) ? ring.labels : [];
    if (!labels.length) return;

    const step = (Math.PI * 2) / labels.length;
    const active = num(ring.active, -1, -1, 7);
    const sectorClass = kind === 'command' ? 'cmd-sector' : 'lang-sector';
    const labelClass = kind === 'command' ? 'cmd-label' : 'lang-label';

    labels.forEach((label, i) => {
      const center = -Math.PI / 2 + i * step;
      const selected = active === i;

      const sector = svg('path', {
        d: sectorPath(R.wheelIn, R.wheelOut, center - step / 2, center + step / 2),
        class: `${sectorClass}${selected ? ' is-selected' : ''}`,
      });
      group.appendChild(sector);

      const cx = Math.cos(center) * R.wheelText;
      const cy = Math.sin(center) * R.wheelText;

      const node = svg('text', {
        class: `${labelClass}${selected ? ' is-selected' : ''}`,
        x: cx.toFixed(2),
        y: cy.toFixed(2),
      });
      node.textContent = text(label.text);
      group.appendChild(node);

      if (text(label.sub)) {
        const sub = svg('text', {
          class: 'slot-sub',
          x: cx.toFixed(2),
          y: (cy + 14 * typography.textScale).toFixed(2),
        });
        sub.textContent = text(label.sub);
        group.appendChild(sub);
      }

      if (text(label.badge)) {
        const badge = svg('text', {
          class: 'slot-badge',
          x: cx.toFixed(2),
          y: (cy + 21 * typography.textScale).toFixed(2),
        });
        badge.textContent = text(label.badge);
        group.appendChild(badge);
      }
    });
  }

  /* ---------------- pill ---------------- */

  function modeLabel(s) {
    const mode = s.mode;
    if (mode === 'type-en') return 'Type EN';
    if (mode === 'type-th') return 'Type TH';
    if (mode === 'type-other') {
      const code = text(s.language && s.language.code).toUpperCase();
      return code ? `Type ${code}` : 'Type';
    }
    return MODE_LABEL[mode] || mode;
  }

  function renderPill(s) {
    const mode = modeLabel(s);
    const code = text(s.language && s.language.code).toUpperCase();
    const status = text(s.status && s.status.text).trim();

    dom.pillMode.textContent = mode;

    // status.text ที่ซ้ำกับชื่อโหมดหรือรหัสภาษาที่แสดงอยู่แล้ว → ไม่พิมพ์ซ้ำ
    const dup =
      status !== '' &&
      (status.toLowerCase() === mode.toLowerCase() ||
        (code !== '' && status.toLowerCase() === code.toLowerCase()));
    dom.pillText.textContent = dup ? '' : status;

    // ภาษา: โหมดพิมพ์มีรหัสอยู่ในชื่อโหมดแล้ว (Type EN) → ไม่พิมพ์ซ้ำ
    dom.pillLang.textContent = TYPE_MODES.includes(s.mode) ? '' : code;

    const badges = [];
    const push = (value) => {
      const t = text(value).trim();
      if (!t) return;
      if (badges.some((b) => b.toLowerCase() === t.toLowerCase())) return;
      badges.push(t);
    };

    const statusBadges = s.status && Array.isArray(s.status.badges) ? s.status.badges : [];
    statusBadges.forEach(push);
    if (num(s.layer, 1, 1, 2) > 1) push(`layer ${s.layer}`);
    if (s.shiftLock === true) push('shift');

    dom.pillBadges.replaceChildren(
      ...badges.map((value) => {
        const node = document.createElement('span');
        node.className = 'pill-badge';
        node.textContent = value;
        return node;
      }),
    );
  }

  /* ---------------- apply state ---------------- */

  function ringsVisible(s) {
    const mode = s.mode;
    if (mode === 'cmd' || mode === 'lang') return true; // โหมดนี้ = เปิดวงอยู่
    if (!TYPE_MODES.includes(mode)) return false; // mouse / nav / disabled
    if (s.ringAlways === true) return true;
    return RING_STATES.includes(s.state);
  }

  function applyState(next) {
    if (!next || typeof next !== 'object') return;

    const mode = text(next.mode) || 'disabled';
    const visible = next.visible !== false && next.hudHidden !== true;
    const scale = num(next.scale, SCALE_DEFAULT, 0.5, 3);
    const opacity = num(next.opacity, 0.92, 0.1, 1);
    const ringsOn = visible && ringsVisible(next);

    dom.hud.dataset.mode = mode;
    dom.hud.dataset.state = text(next.state) || 'idle';
    dom.hud.dataset.visible = String(visible);
    dom.hud.dataset.rings = ringsOn ? 'on' : 'off';
    dom.hud.style.setProperty('--hud-opacity', opacity.toFixed(3));
    dom.hud.style.setProperty('--hud-scale', scale.toFixed(3));

    // ตำแหน่ง HUD = pivot (ไม่ตามเมาส์ — ADR-11)
    const pivot = clampPivot(next.pivot, scale);
    dom.stage.style.left = `${pivot.x.toFixed(1)}px`;
    dom.stage.style.top = `${pivot.y.toFixed(1)}px`;

    // วง — วงซ่อนอยู่ก็ไม่ต้องสร้าง DOM (idle = เหลือ pill อย่างเดียว)
    clear(dom.inner);
    clear(dom.outer);
    clear(dom.cmd);
    clear(dom.lang);

    const inner = next.inner && typeof next.inner === 'object' ? next.inner : { kind: 'none' };
    const outer = next.outer && typeof next.outer === 'object' ? next.outer : { kind: 'none' };

    if (ringsOn) {
      if (inner.kind === 'group') renderGroupRing(inner);
      else if (inner.kind === 'command') renderWheel(dom.cmd, inner, 'command');
      else if (inner.kind === 'language') renderWheel(dom.lang, inner, 'language');

      // วงนอก (ตัวอักษร) มีเฉพาะโหมดพิมพ์
      if (outer.kind === 'char' && TYPE_MODES.includes(mode)) renderCharRing(outer);
    }

    // ตัวอักษรกลางวง + จำนวนซ้ำ
    const preview = next.preview && typeof next.preview === 'object' ? next.preview : { char: '', count: 0 };
    const previewChar = text(preview.char);
    const previewCount = num(preview.count, 0, 0, 1e6);
    dom.previewChar.textContent = previewChar;
    dom.previewCount.textContent = previewCount > 0 ? `×${previewCount}` : '';
    dom.hud.dataset.preview = previewChar !== '' || previewCount > 0 ? 'on' : 'off';

    // idle 3 วิ → pill จาง (ADR-22) · heartbeat 2 วิ ต้องไม่รีเซ็ต timer
    const signature = `${mode}|${dom.hud.dataset.state}|${ringsOn}|${previewChar}|${previewCount}`;
    if (signature !== lastSignature) {
      lastSignature = signature;
      dom.hud.dataset.idle = 'false';
      if (idleTimer) {
        clearTimeout(idleTimer);
        idleTimer = null;
      }
      if (visible && !ringsOn) {
        idleTimer = setTimeout(() => {
          dom.hud.dataset.idle = 'true';
          idleTimer = null;
        }, 3000);
      }
    }

    renderPill(next);

    // pointer (กำหนดโดยแอปผ่าน pointer.visible)
    const pointer = next.pointer && typeof next.pointer === 'object' ? next.pointer : null;
    const pointerOn = !!(pointer && pointer.visible === true);
    dom.pointer.dataset.visible = String(pointerOn && visible);
    if (pointerOn) {
      dom.pointer.style.left = `${num(pointer.x, 0, -1e6, 1e6).toFixed(1)}px`;
      dom.pointer.style.top = `${num(pointer.y, 0, -1e6, 1e6).toFixed(1)}px`;
    }

    // pivot crosshair — โชว์ตอนโหมด nav (จัดตำแหน่ง UI)
    dom.pivot.dataset.visible = String(mode === 'nav' && visible);
    dom.pivot.style.left = `${pivot.x.toFixed(1)}px`;
    dom.pivot.style.top = `${pivot.y.toFixed(1)}px`;

    document.title = `RniType HUD — ${mode}/${dom.hud.dataset.state}`;
  }

  /* ---------------- state source ---------------- */

  function startTauriSource(tauri) {
    const event = tauri.event;
    if (!event || typeof event.listen !== 'function') {
      console.warn('[hud] window.__TAURI__.event.listen ไม่พร้อม — HUD จะไม่ได้รับ state');
      return;
    }

    event
      .listen('hud:state', (message) => {
        const payload =
          message && typeof message === 'object' && 'payload' in message ? message.payload : message;
        applyState(payload);
      })
      .catch((err) => console.warn('[hud] ฟัง event hud:state ไม่สำเร็จ', err));

    // style ของตัวหนังสือ (typography) — เปลี่ยนไม่บ่อย จึงแยกจาก hud:state (ipc.md §2)
    event
      .listen('hud:style', (message) => applyStyle(payloadOf(message)))
      .catch((err) => console.warn('[hud] ฟัง event hud:style ไม่สำเร็จ', err));

    // ipc.md §3 — HUD แจ้งว่าพร้อมรับ state (ไม่เปลี่ยน state ของแอป)
    const invoke =
      tauri.core && typeof tauri.core.invoke === 'function' ? tauri.core.invoke.bind(tauri.core) : null;
    if (!invoke) return;

    // ดึง style ปัจจุบันก่อน (กันหลุด event ที่ยิงก่อน HUD พร้อม) — applyStyle ซ้ำไม่มีผลข้างเคียง
    invoke('hud_style')
      .then((style) => applyStyle(style))
      .catch(() => {});
    invoke('hud_ready').catch(() => {});
  }

  let lastJson = '';
  let lastSignature = '';
  let idleTimer = null;

  async function poll() {
    try {
      const res = await fetch('/api/state', { cache: 'no-store' });
      if (res.ok) {
        const body = await res.text();
        if (body !== lastJson) {
          lastJson = body;
          applyState(JSON.parse(body));
        }
      }
    } catch {
      // state player ปิดอยู่ — เงียบไว้
    }
    setTimeout(poll, POLL_MS);
  }

  function start() {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    if (tauri) {
      startTauriSource(tauri); // โหมดแอปจริง: ไม่ fetch
      return;
    }
    poll(); // โหมดเบราว์เซอร์: ดูจาก state player
  }

  /* ---------------- boot ---------------- */

  if (new URLSearchParams(location.search).has('preview')) {
    document.documentElement.classList.add('preview-bg');
  }

  // โหมดเบราว์เซอร์: ปรับ typography จาก query เพื่อตรวจที่ text scale ต่าง ๆ
  // (?preview=1&textScale=1.35&font=noto-sans-thai) — ไม่มีผลในโหมดแอปจริง
  {
    const params = new URLSearchParams(location.search);
    const previewStyle = {};
    if (params.has('textScale')) previewStyle.textScale = Number(params.get('textScale'));
    if (params.has('font')) previewStyle.font = params.get('font');
    applyStyle(previewStyle);
  }

  start();
})();
