/* ============================================================
   RniType — HUD interactive mockup (Ticket 01 · v0.7)
   จำลอง interaction model ตาม project menager/Design/02-design/*
   - วงใน (LS) = กลุ่มอักษร / วงคำสั่ง (โหมด + Copy/Paste/Undo/Redo)
   - วงนอก (RS) = ตัวอักษร, ปล่อย RS = พิมพ์
   - D-pad ← = เปิดวงคำสั่ง (เลือกด้วย RS, ปล่อย = ยืนยัน)
   - RB/RT = วงล้อชั้น 2 (ใน Type mode) / คลิกซ้าย-ขวา (ใน Mouse mode)
   - ใช้จอยจริงได้ผ่าน Gamepad API (Xbox / DualSense)
   ============================================================ */
(() => {
  'use strict';

  /* ---------------- keymap ---------------- */

  const g = (label, name, chars) => ({ label, name, chars });

  // Layer 1 — Type-EN: group ↑ = vowels A E I O U + common letters
  // Stored lowercase: LT (Shift) toggles uppercase lock
  const EN_1 = [
    g('Vow', 'Vowels A E I O U + common', ['a', 'e', 'i', 'o', 'u', 't', 'n', 's']),
    g('B', 'Common letters', ['h', 'r', 'd', 'l', 'c', 'm', 'f', 'w']),
    g('C', 'Mid letters', ['g', 'p', 'b', 'v', 'k', 'x', 'y', 'j']),
    g('D', 'Rare + symbols', ['q', 'z', "'", '"', '-', '_', '.', ',']),
    g('1', 'Digits 1–8', ['1', '2', '3', '4', '5', '6', '7', '8']),
    g('2', '9 0 + brackets', ['9', '0', '/', '\\', '(', ')', '[', ']']),
    g('3', 'Symbols', ['!', '?', '@', '#', '$', '%', '&', '*']),
    g('4', 'Symbols 2', [':', ';', '=', '+', '<', '>', '`', '~'])
  ];

  // Layer 2 — Type-EN (RB/RT): uppercase + control keys
  const EN_2 = [
    g('A–H', 'Uppercase A–H', ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']),
    g('I–P', 'Uppercase I–P', ['I', 'J', 'K', 'L', 'M', 'N', 'O', 'P']),
    g('Q–X', 'Uppercase Q–X', ['Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X']),
    g('Y,Z', 'Y Z + F1–F6', ['Y', 'Z', 'F1', 'F2', 'F3', 'F4', 'F5', 'F6']),
    g('F7+', 'F7–F12 + PrtSc', ['F7', 'F8', 'F9', 'F10', 'F11', 'F12', 'PrtSc', 'Ins']),
    g('Nav', 'Home / End / arrows', ['Home', 'End', 'PgUp', 'PgDn', '←', '↑', '↓', '→']),
    g('Keys', 'Del / Enter / Tab', ['Del', 'Bksp', 'Esc', 'Enter', 'Tab', 'Space', 'Ctrl', 'Shift']),
    g('Sym', 'Extra symbols', ['{', '}', '|', '^', '¬', '°', '±', '§'])
  ];

  // Layer 1 — Type-TH (v0.10: all tone marks in one group)
  const TH_1 = [
    g('พยบ', 'Common consonants', ['ก', 'น', 'ร', 'อ', 'ง', 'ม', 'ย', 'ล']),
    g('สระ', 'Vowels', ['า', 'ะ', 'ิ', 'ี', 'ื', 'ึ', 'ุ', 'ู']),
    g('นำ', 'Leading vowels + special', ['เ', 'แ', 'โ', 'ใ', 'ไ', 'อ', 'ฤ', 'ฮ']),
    g('วรรณ', 'Tone marks / diacritics', ['่', '้', '๊', '๋', '์', 'ำ', 'ั', '็']),
    g('พย2', 'Mid consonants', ['ด', 'ท', 'ส', 'ว', 'ต', 'ป', 'ค', 'ข']),
    g('พย3', 'Mid-rare consonants', ['จ', 'ช', 'บ', 'พ', 'ฟ', 'ผ', 'ฝ', 'ห']),
    g('บาลี', 'Rare / Pali', ['ซ', 'ญ', 'ฎ', 'ฏ', 'ฐ', 'ฑ', 'ฒ', 'ณ']),
    g('อื่น', 'Remaining consonants', ['ฉ', 'ฌ', 'ฆ', 'ธ', 'ศ', 'ษ', 'ถ', 'ภ'])
  ];

  // Layer 2 — Type-TH (RB/RT)
  const TH_2 = [
    g('ฬ ฃ ฅ', 'Characters not in layer 1', ['ฬ', 'ฃ', 'ฅ', 'ํ', 'ๅ', '', '', '']),
    g('ฦ ฯ ๆ', 'Special characters', ['ฦ', 'ฯ', 'ๆ', '๚', '๛', '', '', '']),
    g('๑–๘', 'Thai digits ๑–๘', ['๑', '๒', '๓', '๔', '๕', '๖', '๗', '๘']),
    g('๙ ๐ ฿', 'Thai digits + currency', ['๙', '๐', '฿', '–', '—', '', '', '']),
    g('ว่าง', 'Empty', ['', '', '', '', '', '', '', '']),
    g('ว่าง', 'Empty', ['', '', '', '', '', '', '', '']),
    g('Nav', 'Home / End / arrows', ['Home', 'End', 'PgUp', 'PgDn', '←', '↑', '↓', '→']),
    g('Keys', 'Del / Enter / Tab', ['Del', 'Bksp', 'Esc', 'Enter', 'Tab', 'Space', 'Ctrl', 'Shift'])
  ];

  // ---------------- languages ----------------
  // kind: 'layout' = map characters by physical key · 'ime' = pass keys to Windows IME
  // (Thai/English have designed tables; other layouts start from QWERTY positions)

  const RU_MAP = {
    q: 'й', w: 'ц', e: 'у', r: 'к', t: 'е', y: 'н', u: 'г', i: 'ш', o: 'щ', p: 'з',
    '[': 'х', ']': 'ъ', a: 'ф', s: 'ы', d: 'в', f: 'а', g: 'п', h: 'р', j: 'о',
    k: 'л', l: 'д', ';': 'ж', "'": 'э', z: 'я', x: 'ч', c: 'с', v: 'м', b: 'и',
    n: 'т', m: 'ь', ',': 'б', '.': 'ю', '/': '.', '-': '-', _: '_'
  };

  const EL_MAP = {
    q: ';', w: 'ς', e: 'ε', r: 'ρ', t: 'τ', y: 'υ', u: 'θ', i: 'ι', o: 'ο', p: 'π',
    a: 'α', s: 'σ', d: 'δ', f: 'φ', g: 'γ', h: 'η', j: 'ξ', k: 'κ', l: 'λ',
    z: 'ζ', x: 'χ', c: 'ψ', v: 'ω', b: 'β', n: 'ν', m: 'μ', '-': '-', _: '_'
  };

  const LANGS = [
    { key: 'en', code: 'EN', name: 'English', kind: 'layout', base: true },
    { key: 'th', code: 'TH', name: 'Thai', kind: 'layout' },
    { key: 'ru', code: 'RU', name: 'Russian', kind: 'layout', map: RU_MAP, sample: true },
    { key: 'el', code: 'EL', name: 'Greek', kind: 'layout', map: EL_MAP, sample: true },
    { key: 'ja', code: 'JA', name: 'Japanese', kind: 'ime', sample: true },
    { key: 'ko', code: 'KO', name: 'Korean', kind: 'ime', sample: true }
  ];

  // Mockup focuses on Thai: default "installed" = English + Thai.
  // Production: GetKeyboardLayoutList + installed input languages at startup.
  const langEnabled = { en: true, th: true, ru: false, el: false, ja: false, ko: false };
  const langOverrides = {}; // { ru: { '0-3': 'x' } } — 8×8 cell edits

  const BASE_LAYERS = { en: [EN_1, EN_2], th: [TH_1, TH_2] };

  function mapChar(ch, map) {
    if (!map) return ch;
    const lower = ch.toLowerCase();
    const mapped = map[lower];
    if (!mapped) return ch;
    return ch === lower ? mapped : mapped.toUpperCase();
  }

  function translatedGroups(groups, lang) {
    const ov = langOverrides[lang.key] || {};
    return groups.map((gr, gi) => ({
      ...gr,
      chars: gr.chars.map((ch, ci) => {
        const o = ov[gi + '-' + ci];
        return o != null ? o : mapChar(ch, lang.map);
      })
    }));
  }

  function langByKey(key) {
    return LANGS.find((l) => l.key === key) || LANGS[0];
  }

  function langKeyOfMode(mode) {
    return mode && mode.startsWith('type-') ? mode.slice(5) : 'en';
  }

  function buildLayers() {
    const out = {};
    for (const lang of LANGS) {
      const base = BASE_LAYERS[lang.key] || [EN_1, EN_2];
      out['type-' + lang.key] = [
        translatedGroups(base[0], lang),
        translatedGroups(base[1], lang)
      ];
    }
    return out;
  }

  let LAYERS = buildLayers();

  // Quick bar — 8 ทางลัด (01-interaction-model §5)
  const QUICK = [
    { label: 'Copy', combo: 'Ctrl + C' },
    { label: 'Paste', combo: 'Ctrl + V' },
    { label: 'Cut', combo: 'Ctrl + X' },
    { label: 'Undo', combo: 'Ctrl + Z' },
    { label: 'Redo', combo: 'Ctrl + Y' },
    { label: 'All', combo: 'Ctrl + A' },
    { label: 'Save', combo: 'Ctrl + S' },
    { label: 'Alt+Tab', combo: 'Alt + Tab' }
  ];

  // Command wheel (D-pad ←) — 8 slots
  // order = direction: 0 ↑ · 1 ↗ · 2 → · 3 ↘ · 4 ↓ · 5 ↙ · 6 ← · 7 ↖
  const CMD_SLOTS = [
    { key: 'type-en', label: 'EN', name: 'English', sub: 'Latin / QWERTY', kind: 'mode' },
    { key: 'language', label: 'TH', name: 'Language', sub: 'Installed layouts', kind: 'mode' },
    { key: 'mouse', label: 'Mouse', name: 'Mouse', sub: 'Move pointer', kind: 'mode' },
    { key: 'nav', label: 'UI', name: 'Place UI', sub: 'Move pivot', kind: 'mode' },
    { key: 'copy', label: 'Copy', name: 'Copy', sub: 'Ctrl + C', kind: 'edit' },
    { key: 'paste', label: 'Paste', name: 'Paste', sub: 'Ctrl + V', kind: 'edit' },
    { key: 'undo', label: 'Undo', name: 'Undo', sub: 'Ctrl + Z', kind: 'edit' },
    { key: 'redo', label: 'Redo', name: 'Redo', sub: 'Ctrl + Y', kind: 'edit' }
  ];

  /* ---------------- geometry ---------------- */

  const R = {
    charOut: 158, charIn: 74,
    groupOut: 68, groupIn: 31,
    groupText: 49.5, charText: 116, indexText: 92
  };
  const NS = 'http://www.w3.org/2000/svg';

  const P = {
    lsAct: 0.35, lsRel: 0.25,
    rsAct: 0.40, rsRel: 0.25,
    hyst: 9,
    commit: 0.06,
    cancel: 0.60,
    modeHold: 0.26
  };

  /* ---------------- config / state ---------------- */

  const cfg = {
    mode: 'type-en',
    ringAlways: false,   // false = variant B (idle เหลือ pill) · true = โหมดเรียนรู้ (แสดงวงตลอด)
    opacity: 0.92,
    scale: 1,
    aim: false,
    index: false,
    autohide: true,
    hudHidden: false
  };

  const st = {
    name: 'idle',        // idle | group | char | commit
    group: 0,
    sector: -1,
    hold: 0,
    commit: 0,
    lock: false,
    shiftLock: false,    // LT/Shift = toggle ตัวพิมพ์ใหญ่ (latch)
    lastAct: performance.now()
  };

  const pad = { ls: { x: 0, y: 0 }, rs: { x: 0, y: 0 }, lt: false, lb: false, rb: false, rt: false };
  const target = { ls: { x: 0, y: 0 }, rs: { x: 0, y: 0 } };
  const kb = { ls: new Set(), rs: new Set(), lt: false, lb: false, rb: false, rt: false };
  const gp = {
    ls: { x: 0, y: 0 }, rs: { x: 0, y: 0 },
    btn: {}, last: {}, name: null, lastAct: 0,
    lt: false, lb: false, rb: false, rt: false
  };
  const demo = { active: false, token: 0, ls: { x: 0, y: 0 }, rs: { x: 0, y: 0 } };

  const layer = { down: false, pinned: false, at: 0 };
  const cmdWheel = { open: false, idx: -1, sector: -1, commit: 0, armed: true };
  const langWheel = { open: false, idx: -1, sector: -1, commit: 0, armed: true };
  let langList = [];
  const btnPrev = { rb: false, rt: false };
  let ltPrev = false;
  let r3Pending = false;
  let r3Timer = 0;

  const pointer = { x: window.innerWidth * 0.5, y: window.innerHeight * 0.6 };
  const pivot = { x: window.innerWidth * 0.62, y: window.innerHeight * 0.55 };
  let pointerUserMoved = false;
  let docScroll = 0;
  let lastTypeMode = 'type-en';

  /* ---------------- dom ---------------- */

  const $ = (sel) => document.querySelector(sel);
  const scene = $('#scene');
  const hud = $('#hud');
  const docEl = $('#doc');
  const pointerEl = $('#pointer');
  const pivotEl = $('#pivot');
  const toastEl = $('#toast');
  const chipMode = $('#chipMode');
  const chipSel = $('#chipSel');
  const chipMod = $('#chipMod');
  const commitRing = $('#commitRing');
  const aimLS = $('#aimLS');
  const aimRS = $('#aimRS');
  const hudHint = $('#hudHint');
  const cmdCenter = $('#cmdCenter');

  /* ---------------- document model ---------------- */

  const DEFAULT_DOC =
    '# spike-c-sendinput.md\n\n' +
    'Spike C — ส่งคีย์ภาษาไทยผ่าน SendInput\n\n' +
    'ต้องผ่าน: พิมพ์ "กขคง" ลง Notepad ได้ถูกต้อง\n' +
    'ตรวจ: Unicode path · ไม่มีคีย์ค้าง · latency < 16 ms\n\n' +
    'HUD ตรึงที่ pivot — ดัน LS เพื่อเปิดวงล้อ (ย้าย UI ได้ในโหมด Nav)\n';

  let docText = DEFAULT_DOC;
  let caretPos = docText.length;
  let caretEl = null;
  const typedTimes = [];

  /* ---------------- helpers ---------------- */

  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const mag = (v) => Math.min(1, Math.hypot(v.x, v.y));
  const raf = (fn) => requestAnimationFrame(fn);
  const sleep = (ms) => new Promise((res) => setTimeout(res, ms));
  const isTyping = () => cfg.mode.startsWith('type-');
  const isQuick = () => cfg.mode === 'quickbar';

  // สระไทยที่เขียนล่างฐาน (ุ ู ฺ) → HUD วาดขีดกำกับด้านบนให้เห็นชัด
  const TH_BELOW = new Set(['\u0E38', '\u0E39', '\u0E3A']);

  function isBelowMark(ch) {
    return !!ch && TH_BELOW.has(ch);
  }

  function esc(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function layerNo() {
    if (!isTyping()) return 1;
    return (layer.down || layer.pinned) ? 2 : 1;
  }

  function ringGroups() {
    if (isTyping()) return LAYERS[cfg.mode][layerNo() - 1];
    return null;
  }

  function angleFromUp(v) {
    return ((Math.atan2(v.x, -v.y) * 180) / Math.PI + 360) % 360;
  }

  function sectorOf(v, current = -1) {
    const a = angleFromUp(v);
    const cand = Math.round(a / 45) % 8;
    if (current >= 0 && cand !== current) {
      const boundary = (current * 45 + 22.5) % 360;
      const diff = Math.abs((((a - boundary + 540) % 360) - 180));
      if (diff < P.hyst) return current;
    }
    return cand;
  }

  const aimVec = (deg) => {
    const rad = ((deg - 90) * Math.PI) / 180;
    return { x: Math.cos(rad), y: Math.sin(rad) };
  };

  function toast(msg) {
    toastEl.textContent = msg;
    toastEl.classList.add('is-on');
    clearTimeout(toast._t);
    toast._t = setTimeout(() => toastEl.classList.remove('is-on'), 1200);
  }

  /* ---------------- build the rings ---------------- */

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

  const charPaths = [];
  const charTexts = [];
  const charGuides = [];
  const charBaseY = [];
  const groupPaths = [];
  const groupLabels = [];
  const cmdSectors = [];
  const cmdLabels = [];
  const langSectors = [];
  const langLabels = [];

  function buildRings() {
    const charRing = $('#charRing');
    const groupRing = $('#groupRing');

    for (let i = 0; i < 8; i++) {
      const center = -Math.PI / 2 + (i * Math.PI) / 4;
      const a0 = center - Math.PI / 8;
      const a1 = center + Math.PI / 8;

      const p = document.createElementNS(NS, 'path');
      p.setAttribute('d', sectorPath(R.charIn, R.charOut, a0, a1));
      p.setAttribute('class', 'sector sector--char');
      p.dataset.i = String(i);
      charRing.appendChild(p);
      charPaths.push(p);

      const ix = document.createElementNS(NS, 'text');
      ix.setAttribute('class', 'index-text');
      ix.setAttribute('x', (Math.cos(center) * R.indexText).toFixed(2));
      ix.setAttribute('y', (Math.sin(center) * R.indexText).toFixed(2));
      ix.textContent = String(i + 1);
      charRing.appendChild(ix);

      const gp2 = document.createElementNS(NS, 'path');
      gp2.setAttribute('d', sectorPath(R.groupIn, R.groupOut, a0, a1));
      gp2.setAttribute('class', 'sector sector--group');
      gp2.dataset.i = String(i);
      groupRing.appendChild(gp2);
      groupPaths.push(gp2);

      const label = document.createElementNS(NS, 'text');
      label.setAttribute('class', 'sector-label sector-label--group');
      label.setAttribute('x', (Math.cos(center) * R.groupText).toFixed(2));
      label.setAttribute('y', (Math.sin(center) * R.groupText).toFixed(2));
      groupRing.appendChild(label);
      groupLabels.push(label);

      const ct = document.createElementNS(NS, 'text');
      ct.setAttribute('class', 'char-text is-dim');
      const chx = Math.cos(center) * R.charText;
      const chy = Math.sin(center) * R.charText;
      ct.setAttribute('x', chx.toFixed(2));
      ct.setAttribute('y', chy.toFixed(2));
      ct.textContent = '';
      charRing.appendChild(ct);
      charTexts.push(ct);

      // ขีดกำกับสำหรับสระ/วรรณยุกต์ที่ลอย (ุ ู ่ ้ ...) ให้เห็นตำแหน่งบน–ล่าง
      const guide = document.createElementNS(NS, 'line');
      guide.setAttribute('class', 'char-guide');
      guide.setAttribute('x1', (chx - 9).toFixed(2));
      guide.setAttribute('x2', (chx + 9).toFixed(2));
      guide.setAttribute('y1', chy.toFixed(2));
      guide.setAttribute('y2', chy.toFixed(2));
      charRing.appendChild(guide);
      charGuides.push(guide);
      charBaseY.push(chy);
    }

    $('#aimLSWedge').setAttribute('d', sectorPath(30, 148, -Math.PI / 2 - 0.24, -Math.PI / 2 + 0.24));
    $('#aimRSWedge').setAttribute('d', sectorPath(52, 148, -Math.PI / 2 - 0.22, -Math.PI / 2 + 0.22));

    // วงคำสั่งในวงใน: 8 ช่อง (โหมด 4 + แก้ข้อความ 4) — เลือกด้วย RS
    const cmdRing = $('#cmdRing');
    CMD_SLOTS.forEach((slot, i) => {
      const center = -Math.PI / 2 + (i * Math.PI) / 4;
      const p = document.createElementNS(NS, 'path');
      p.setAttribute('d', sectorPath(34, 96, center - Math.PI / 8, center + Math.PI / 8));
      p.setAttribute('class', `cmd-sector cmd-sector--${slot.kind}`);
      p.dataset.i = String(i);
      cmdRing.appendChild(p);
      cmdSectors.push(p);

      const label = document.createElementNS(NS, 'text');
      label.setAttribute('class', 'cmd-label');
      label.setAttribute('x', (Math.cos(center) * 69).toFixed(2));
      label.setAttribute('y', (Math.sin(center) * 69).toFixed(2));
      label.textContent = slot.label;
      cmdRing.appendChild(label);
      cmdLabels.push(label);
    });

    // วงภาษา (ชั้นที่ 2 ของวงคำสั่ง) — 8 ช่องสำหรับ layout ที่ติดตั้ง
    const langRing = $('#langRing');
    for (let i = 0; i < 8; i++) {
      const center = -Math.PI / 2 + (i * Math.PI) / 4;
      const p = document.createElementNS(NS, 'path');
      p.setAttribute('d', sectorPath(34, 96, center - Math.PI / 8, center + Math.PI / 8));
      p.setAttribute('class', 'lang-sector');
      p.dataset.i = String(i);
      langRing.appendChild(p);
      langSectors.push(p);

      const label = document.createElementNS(NS, 'text');
      label.setAttribute('class', 'lang-label');
      label.setAttribute('x', (Math.cos(center) * 69).toFixed(2));
      label.setAttribute('y', (Math.sin(center) * 69).toFixed(2));
      label.textContent = '';
      langRing.appendChild(label);
      langLabels.push(label);
    }
  }

  /* ---------------- typing / document ---------------- */

  /* ---------------- selection / caret ---------------- */

  let selAnchor = -1;                 // -1 = ไม่มี selection
  const selMode = { active: false };   // โหมดเลือกด้วย RS (R3 สองครั้ง)
  const drag = { active: false, anchor: -1 };
  const CARET_HTML = '<span class="caret" id="caret"></span>';

  function selectionRange() {
    if (selAnchor < 0 || selAnchor === caretPos) return null;
    return [Math.min(selAnchor, caretPos), Math.max(selAnchor, caretPos)];
  }

  function deleteSelection() {
    const sel = selectionRange();
    if (!sel) return false;
    const [a, b] = sel;
    docText = docText.slice(0, a) + docText.slice(b);
    caretPos = a;
    selAnchor = -1;
    return true;
  }

  function renderDoc() {
    const sel = selectionRange();
    let html = '';

    if (sel) {
      const [a, b] = sel;
      const marked = `<span class="sel">${esc(docText.slice(a, b))}</span>`;
      html += esc(docText.slice(0, a));
      html += caretPos === a ? CARET_HTML + marked : marked + CARET_HTML;
      html += '<span class="preview" id="preview"></span>';
      html += esc(docText.slice(b));
    } else {
      html += esc(docText.slice(0, caretPos));
      html += CARET_HTML;
      html += '<span class="preview" id="preview"></span>';
      html += esc(docText.slice(caretPos));
    }

    docEl.innerHTML = html;
    caretEl = document.getElementById('caret');
    docEl.style.transform = `translateY(${-docScroll}px)`;
    updateStats();
  }

  function lineStartAt(pos) {
    return docText.lastIndexOf('\n', Math.max(0, pos - 1)) + 1;
  }

  /// เลื่อน caret: 0=ขึ้น 2=ขวา 4=ลง 6=ซ้าย
  function moveCaret(dir, extend) {
    let next = caretPos;

    if (dir === 2) {
      next = Math.min(docText.length, caretPos + 1);
    } else if (dir === 6) {
      next = Math.max(0, caretPos - 1);
    } else {
      const start = lineStartAt(caretPos);
      const col = caretPos - start;

      if (dir === 0) {
        if (start === 0) return;
        const prevStart = lineStartAt(start - 1);
        next = Math.min(prevStart + col, start - 1);
      } else {
        const nl = docText.indexOf('\n', caretPos);
        if (nl < 0) return;
        const nextStart = nl + 1;
        const end = docText.indexOf('\n', nextStart);
        next = Math.min(nextStart + col, end < 0 ? docText.length : end);
      }
    }

    if (next === caretPos) return;

    if (extend) {
      if (selAnchor < 0) selAnchor = caretPos;
    } else {
      selAnchor = -1;
    }

    caretPos = next;
    renderDoc();
    ensureCaretVisible();
  }

  function ensureCaretVisible() {
    if (!caretEl) return;
    const r = caretEl.getBoundingClientRect();
    const body = docEl.parentElement.getBoundingClientRect();
    if (r.top < body.top + 8) scrollDoc(-(body.top + 20 - r.top));
    else if (r.bottom > body.bottom - 8) scrollDoc(r.bottom - body.bottom + 20);
  }

  function insertText(txt) {
    if (!txt) return;
    snapshot();
    deleteSelection();
    docText = docText.slice(0, caretPos) + txt + docText.slice(caretPos);
    caretPos += txt.length;
    typedTimes.push(performance.now());
    if (typedTimes.length > 60) typedTimes.shift();
    renderDoc();
    updateStats();
    st.lastAct = performance.now();
  }

  function backspace() {
    if (deleteSelection()) {
      renderDoc();
      st.lastAct = performance.now();
      return;
    }
    if (caretPos === 0) return;
    snapshot();
    docText = docText.slice(0, caretPos - 1) + docText.slice(caretPos);
    caretPos -= 1;
    renderDoc();
    st.lastAct = performance.now();
  }

  /* ---------------- clipboard / undo-redo ---------------- */

  let clipboard = '';
  const history = { undo: [], redo: [] };

  function snapshot() {
    history.undo.push({ text: docText, caret: caretPos, anchor: selAnchor });
    if (history.undo.length > 80) history.undo.shift();
    history.redo.length = 0;
  }

  function restoreDoc(state) {
    docText = state.text;
    caretPos = clamp(state.caret, 0, docText.length);
    selAnchor = state.anchor >= 0 ? clamp(state.anchor, 0, docText.length) : -1;
    selMode.active = false;
    drag.active = false;
    renderDoc();
  }

  function undo() {
    if (!history.undo.length) { toast('Nothing to undo'); return; }
    history.redo.push({ text: docText, caret: caretPos, anchor: selAnchor });
    restoreDoc(history.undo.pop());
    toast('Undo');
  }

  function redo() {
    if (!history.redo.length) { toast('Nothing to redo'); return; }
    history.undo.push({ text: docText, caret: caretPos, anchor: selAnchor });
    restoreDoc(history.redo.pop());
    toast('Redo');
  }

  function copySelection() {
    const sel = selectionRange();
    if (sel) {
      clipboard = docText.slice(sel[0], sel[1]);
    } else {
      const start = lineStartAt(caretPos);
      const end = docText.indexOf('\n', caretPos);
      clipboard = docText.slice(start, end < 0 ? docText.length : end);
    }
    toast(clipboard ? `Copied ${clipboard.length} chars` : 'Nothing to copy');
  }

  function pasteClipboard() {
    if (!clipboard) { toast('Clipboard is empty'); return; }
    insertText(clipboard);
    toast('Paste');
  }

  function runEditAction(key) {
    if (key === 'copy') copySelection();
    else if (key === 'paste') pasteClipboard();
    else if (key === 'undo') undo();
    else if (key === 'redo') redo();
    st.lastAct = performance.now();
  }

  function wpm() {
    if (typedTimes.length < 3) return 0;
    const span = (typedTimes[typedTimes.length - 1] - typedTimes[0]) / 1000;
    if (span < 0.8) return 0;
    return Math.round((typedTimes.length / 5) / (span / 60));
  }

  function updateStats() {
    $('#statChars').textContent = String(typedTimes.length);
    $('#statWpm').textContent = String(wpm());
    $('#statPad').textContent = gp.name ? gp.name : '—';
  }

  function scrollDoc(delta) {
    const body = docEl.parentElement;
    const max = Math.max(0, docEl.scrollHeight - body.clientHeight + 24);
    docScroll = clamp(docScroll + delta, 0, max);
    docEl.style.transform = `translateY(${-docScroll}px)`;
  }

  /* ---------------- commit ---------------- */

  function currentChar() {
    const gs = ringGroups();
    if (!gs || st.sector < 0) return '';
    let ch = gs[st.group].chars[st.sector] || '';
    if (st.shiftLock && /^[a-z]$/.test(ch)) ch = ch.toUpperCase();
    return ch;
  }

  function commitChar() {
    if (isQuick()) {
      const q = QUICK[st.sector];
      if (q) {
        toast(`${q.label} · ${q.combo}`);
        st.lastAct = performance.now();
      }
      return;
    }
    const ch = currentChar();
    if (!ch) return;
    st.lastChar = ch;
    insertText(ch);
  }

  /* ---------------- pointer / mouse / nav ---------------- */

  function setPointer(x, y) {
    pointer.x = clamp(x, 0, window.innerWidth - 1);
    pointer.y = clamp(y, 0, window.innerHeight - 1);
    pointerEl.style.left = pointer.x + 'px';
    pointerEl.style.top = pointer.y + 'px';
  }

  // HUD ตรึงที่ pivot (ไม่ตามเมาส์) — Nav mode ใช้จัดตำแหน่ง
  function setPivot(x, y) {
    const m = 172 * cfg.scale;
    pivot.x = clamp(x, m, window.innerWidth - m);
    pivot.y = clamp(y, m, window.innerHeight - m);
    pivotEl.style.left = pivot.x + 'px';
    pivotEl.style.top = pivot.y + 'px';
  }

  function resetPivot() {
    setPivot(window.innerWidth * 0.62, window.innerHeight * 0.55);
    toast('UI back to default position');
  }

  function recenterPointer() {
    setPointer(window.innerWidth / 2, window.innerHeight / 2);
    pointerEl.classList.add('is-flash');
    setTimeout(() => pointerEl.classList.remove('is-flash'), 420);
    toast('Pointer centered');
  }

  function ripple(kind) {
    const d = document.createElement('div');
    d.className = 'ripple' + (kind === 2 ? ' ripple--right' : '');
    d.style.left = pointer.x + 'px';
    d.style.top = pointer.y + 'px';
    document.body.appendChild(d);
    setTimeout(() => d.remove(), 520);
  }

  /// แปลงพิกัดหน้าจอ → offset ในข้อความ (ใช้กับ click/drag ในเอกสาร)
  function docOffsetAt(x, y) {
    if (!document.caretRangeFromPoint || !docEl) return null;
    const range = document.caretRangeFromPoint(x, y);
    if (!range || !docEl.contains(range.startContainer)) return null;
    const pre = document.createRange();
    pre.selectNodeContents(docEl);
    pre.setEnd(range.startContainer, range.startOffset);
    return pre.toString().length;
  }

  function clickAt(kind) {
    ripple(kind);

    if (kind === 2) {
      toast('Context menu (Shift+F10)');
      return;
    }

    // คลิกในเอกสาร = วาง caret ตรงจุดนั้น (ลากต่อได้ถ้ากด RB ค้าง)
    const off = docOffsetAt(pointer.x, pointer.y);
    if (off !== null) {
      caretPos = off;
      selAnchor = -1;
      drag.active = true;
      drag.anchor = off;
      renderDoc();
      st.lastAct = performance.now();
      return;
    }

    const el = document.elementFromPoint(pointer.x, pointer.y);
    if (!el) return;

    const hit = el.closest('button, a, input, label');
    if (hit) {
      if (hit.tagName === 'INPUT' && hit.type === 'range') {
        const r = hit.getBoundingClientRect();
        const frac = clamp((pointer.x - r.left) / r.width, 0, 1);
        const min = Number(hit.min);
        const max = Number(hit.max);
        hit.value = String(Math.round(min + frac * (max - min)));
        hit.dispatchEvent(new Event('input', { bubbles: true }));
      } else if (hit.tagName === 'INPUT' && hit.type === 'checkbox') {
        hit.click();
      } else {
        hit.click();
      }
    }
  }

  function updateMouse(dt) {
    const dz = 0.15;
    const m = mag(pad.rs);

    if (m > dz) {
      const s = (m - dz) / (1 - dz);
      const vmax = 1600 * (pad.lt ? 0.3 : 1) * (pad.lb ? 1.8 : 1);
      const v = vmax * Math.pow(s, 1.7);
      const dirX = pad.rs.x / m;
      const dirY = pad.rs.y / m;
      setPointer(pointer.x + dirX * v * dt, pointer.y + dirY * v * dt);
    }

    // LS = scroll แนวตั้ง/นอน (03-mouse-and-navigation §5)
    if (Math.abs(pad.ls.y) > 0.2) scrollDoc(pad.ls.y * 520 * dt);

    // RB = คลิกซ้าย / กดค้าง + RS = ลากเลือกข้อความ, RT = คลิกขวา
    const rb = pad.rb;
    const rt = pad.rt;

    if (rb && !btnPrev.rb) clickAt(1);

    if (rb && drag.active) {
      const off = docOffsetAt(pointer.x, pointer.y);
      if (off !== null && off !== caretPos) {
        caretPos = off;
        selAnchor = drag.anchor;
        renderDoc();
        st.lastAct = performance.now();
      }
    }

    if (!rb && btnPrev.rb) {
      drag.active = false;
      if (selAnchor === caretPos) selAnchor = -1;
      if (selectionRange()) {
        const sel = selectionRange();
        toast(`Selected ${sel[1] - sel[0]} chars`);
      }
      renderDoc();
    }

    if (rt && !btnPrev.rt) clickAt(2);
    btnPrev.rb = rb;
    btnPrev.rt = rt;
  }

  function updateNav(dt) {
    // Nav = Pivot mode: LS = เลื่อน UI หยาบ, RS = เลื่อนละเอียด, R3 = กลับตำแหน่งเดิม
    const l = mag(pad.ls);
    const r = mag(pad.rs);
    let vec = null;
    let speed = 0;
    if (l > 0.2) {
      const s = (l - 0.2) / 0.8;
      vec = pad.ls;
      speed = 900 * Math.pow(s, 1.5) * (pad.lt ? 0.35 : 1);
    } else if (r > 0.15) {
      const s = (r - 0.15) / 0.85;
      vec = pad.rs;
      speed = 320 * Math.pow(s, 1.5) * (pad.lt ? 0.35 : 1);
    }
    if (!vec) return;
    const m = Math.hypot(vec.x, vec.y) || 1;
    setPivot(pivot.x + (vec.x / m) * speed * dt, pivot.y + (vec.y / m) * speed * dt);
  }

  /* ---------------- RS = ลูกศร (ต้องเปิดด้วย R3 ก่อน) ---------------- */

  const caretNav = { dir: -1, next: 0 };
  const dirMode = { enabled: false };

  function updateCaretNav() {
    if (!dirMode.enabled && !selMode.active) {
      caretNav.dir = -1;
      return;
    }

    if (demo.active) {
      caretNav.dir = -1;
      return;
    }

    if (mag(pad.rs) < 0.45) {
      caretNav.dir = -1;
      return;
    }

    let dir;
    if (Math.abs(pad.rs.x) > Math.abs(pad.rs.y)) dir = pad.rs.x > 0 ? 2 : 6;
    else dir = pad.rs.y > 0 ? 4 : 0;

    const now = performance.now();
    if (caretNav.dir !== dir) {
      caretNav.dir = dir;
      caretNav.next = now + 360;
      moveCaret(dir, selMode.active);
    } else if (now >= caretNav.next) {
      caretNav.next = now + 90;
      moveCaret(dir, selMode.active);
    }
  }

  function toggleDirMode() {
    dirMode.enabled = !dirMode.enabled;
    if (!dirMode.enabled) {
      selMode.active = false;
      if (selAnchor >= 0 && selAnchor === caretPos) selAnchor = -1;
      caretNav.dir = -1;
    }
    toast(dirMode.enabled ? 'Arrows on · RS moves caret' : 'Arrows off · RS idle');
    renderDoc();
    st.lastAct = performance.now();
  }

  function toggleSelectMode() {
    if (selMode.active) {
      selMode.active = false;
      toast('Selection done');
    } else {
      selMode.active = true;
      dirMode.enabled = true;
      selAnchor = caretPos;
      toast('Select mode · move RS to extend');
    }
    renderDoc();
    st.lastAct = performance.now();
  }

  scene.addEventListener('pointermove', (e) => {
    pointerUserMoved = true;
    setPointer(e.clientX, e.clientY);
  });

  scene.addEventListener('contextmenu', (e) => e.preventDefault());

  /* ---------------- input: keyboard ---------------- */

  const KEY_LS = { w: 'up', a: 'left', s: 'down', d: 'right' };
  const KEY_RS = { arrowup: 'up', arrowleft: 'left', arrowdown: 'down', arrowright: 'right' };

  function axisFromSet(set) {
    let x = 0;
    let y = 0;
    if (set.has('up')) y -= 1;
    if (set.has('down')) y += 1;
    if (set.has('left')) x -= 1;
    if (set.has('right')) x += 1;
    const m = Math.hypot(x, y) || 1;
    return { x: x / m, y: y / m };
  }

  const HANDLED = new Set([
    'w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright',
    ' ', 'backspace', 'enter', 'tab', '1', '2', '3', '4', '5', '6', '7', '8',
    'p', 'r', 'm', 'n', 'c', '[', ']', 'shift', 'control', 'q', 'e'
  ]);

  window.addEventListener('keydown', (e) => {
    // keymap editor captures the next single key
    if (mapEdit.gi >= 0) {
      e.preventDefault();
      if (e.key === 'Escape') {
        mapEdit.gi = -1;
        mapEdit.ci = -1;
        renderMapGrid();
      } else if (e.key.length === 1) {
        applyMapKey(e.key);
      }
      return;
    }
    if (e.key === 'Escape' && !$('#settings').hidden) {
      closeSettings();
      return;
    }

    const k = e.key.toLowerCase();
    if (!HANDLED.has(k)) return;
    e.preventDefault();
    if (e.repeat) return;

    if (k in KEY_LS) { kb.ls.add(KEY_LS[k]); return; }
    if (k in KEY_RS) { kb.rs.add(KEY_RS[k]); return; }

    if (k === 'shift') { kb.lt = true; return; }
    if (k === 'control') { kb.lb = true; return; }
    if (k === 'q') { kb.rb = true; return; }
    if (k === 'e') { kb.rt = true; return; }

    if (demo.active) return;

    if (k === 'm' || k === 'c') { toggleCmd(); return; }
    if (k === 'n') { onButton('rs'); return; }
    if (k >= '1' && k <= '4') { setMode(MODE_ORDER[Number(k) - 1]); return; }
    if (k === '5') {
      layer.pinned = !layer.pinned;
      toast(layer.pinned ? 'Layer 2 · locked' : 'Layer 1');
      return;
    }
    if (k === '6') { setMode('quickbar'); toast('Quick bar · choose with RS'); return; }
    if (k === '7') {
      cfg.hudHidden = !cfg.hudHidden;
      toast(cfg.hudHidden ? 'HUD hidden (D-pad ← to show)' : 'HUD shown');
      return;
    }
    if (k === '8') { setMode('disabled'); toast('Paused · D-pad ← to resume'); return; }
    if (k === ' ') { onButton('a'); return; }
    if (k === 'backspace') { onButton('b'); return; }
    if (k === 'enter') { onButton('x'); return; }
    if (k === 'tab') { onButton('y'); return; }
    if (k === 'p') { toggleDemo(); return; }
    if (k === 'r') {
      if (cfg.mode === 'nav') resetPivot(); else recenterPointer();
      return;
    }
    if (k === '[') { setOpacity(cfg.opacity - 0.06); return; }
    if (k === ']') { setOpacity(cfg.opacity + 0.06); return; }
  });

  window.addEventListener('keyup', (e) => {
    const k = e.key.toLowerCase();
    if (k in KEY_LS) kb.ls.delete(KEY_LS[k]);
    if (k in KEY_RS) kb.rs.delete(KEY_RS[k]);
    if (k === 'shift') kb.lt = false;
    if (k === 'control') kb.lb = false;
    if (k === 'q') kb.rb = false;
    if (k === 'e') kb.rt = false;
  });

  window.addEventListener('blur', () => {
    kb.ls.clear();
    kb.rs.clear();
    kb.lt = false;
    kb.lb = false;
    kb.rb = false;
    kb.rt = false;
  });

  /* ---------------- input: gamepad ---------------- */

  const GP_BUTTONS = {
    a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, lt: 6, rt: 7,
    ls: 10, rs: 11, dup: 12, ddown: 13, dleft: 14, dright: 15
  };

  function pressed(p, idx) {
    return !!(p.buttons[idx] && p.buttons[idx].pressed);
  }

  function trigger(p, idx) {
    const b = p.buttons[idx];
    if (!b) return false;
    const v = typeof b.value === 'number' ? b.value : (b.pressed ? 1 : 0);
    return v > 0.4;
  }

  function pollGamepad() {
    const pads = navigator.getGamepads ? navigator.getGamepads() : [];
    let p = null;
    for (const cand of pads) if (cand && cand.connected) { p = cand; break; }

    if (!p) {
      if (gp.name) { gp.name = null; updateStats(); }
      gp.ls = { x: 0, y: 0 };
      gp.rs = { x: 0, y: 0 };
      gp.btn = {};
      gp.last = {};
      gp.lt = gp.lb = gp.rb = gp.rt = false;
      return;
    }

    if (p.id !== gp.name) {
      gp.name = shortPadName(p.id);
      updateStats();
      st.lastAct = performance.now();
    }

    const dz = 0.12;
    const ax = (i) => {
      const v = p.axes[i] || 0;
      return Math.abs(v) < dz ? 0 : v;
    };
    gp.ls = { x: ax(0), y: ax(1) };
    gp.rs = { x: ax(2), y: ax(3) };
    gp.lt = trigger(p, 6);
    gp.rt = trigger(p, 7);
    gp.lb = pressed(p, 4);
    gp.rb = pressed(p, 5);

    const b = {
      a: pressed(p, 0), b: pressed(p, 1), x: pressed(p, 2), y: pressed(p, 3),
      up: pressed(p, 12), down: pressed(p, 13), left: pressed(p, 14), right: pressed(p, 15),
      ls: pressed(p, 10), rs: pressed(p, 11)
    };
    gp.btn = b;

    if (Math.abs(gp.ls.x) + Math.abs(gp.ls.y) + Math.abs(gp.rs.x) + Math.abs(gp.rs.y) > 0.2) {
      gp.lastAct = performance.now();
    }

    for (const name of ['a', 'b', 'x', 'y', 'ls', 'rs']) {
      const now = b[name];
      const was = !!gp.last[name];
      gp.last[name] = now;
      if (now && !was && !demo.active) onButton(name);
    }
  }

  function shortPadName(id) {
    if (/xbox/i.test(id)) return 'Xbox';
    if (/dualsense|dualshock|wireless controller/i.test(id)) return 'DualSense';
    if (/8bitdo/i.test(id)) return '8BitDo';
    return id.split('(')[0].trim().slice(0, 12);
  }

  /* ---------------- buttons / modes ---------------- */

  function onButton(name) {
    if (langWheel.open) {
      if (name === 'a') confirmLang();
      else if (name === 'b') { closeLangWheel(); toast('Canceled'); }
      return;
    }
    if (cmdWheel.open) {
      if (name === 'a') confirmCmd();
      else if (name === 'b') {
        closeCmd();
        toast('Canceled');
      }
      return;
    }

    if (cfg.mode === 'mouse') {
      if (name === 'ls' || name === 'rs') recenterPointer();
      return;
    }

    // R3: กด 1 ครั้ง = เปิด/ปิดลูกศร, กด 2 ครั้งติดกัน = โหมดเลือกข้อความ
    if (name === 'rs') {
      pressR3();
      return;
    }

    if (selMode.active) {
      if (name === 'a') {
        selMode.active = false;
        toast('Selection confirmed');
      } else if (name === 'b') {
        selMode.active = false;
        selAnchor = -1;
        toast('Selection canceled');
      }
      renderDoc();
      return;
    }

    if (cfg.mode === 'disabled') return;

    switch (name) {
      case 'a': typeButton(' '); break;
      case 'b': typeButton('back'); break;
      case 'x': typeButton('\n'); break;
      case 'y': typeButton('  '); break;
      case 'ls': recenterPointer(); break;
      default: break;
    }
    st.lastAct = performance.now();
  }

  const MODE_ORDER = ['type-en', 'type-th', 'mouse', 'nav'];

  function pressR3() {
    if (cfg.mode === 'mouse') {
      recenterPointer();
      return;
    }
    if (cfg.mode === 'nav') {
      resetPivot();
      return;
    }

    if (r3Pending) {
      clearTimeout(r3Timer);
      r3Pending = false;
      toggleSelectMode();
      return;
    }

    r3Pending = true;
    r3Timer = setTimeout(() => {
      r3Pending = false;
      toggleDirMode();
    }, 300);
  }

  function applyCmd(i) {
    const slot = CMD_SLOTS[i];
    if (!slot) return;
    if (slot.key === 'language') {
      const others = otherLangs();
      if (!others.length) return;
      if (others.length === 1) {
        setMode('type-' + others[0].key);
        toast(others[0].name);
      } else {
        openLangWheel();
      }
      return;
    }
    if (slot.kind === 'mode') {
      setMode(slot.key);
      toast(slot.name);
    } else {
      runEditAction(slot.key);
    }
    st.lastAct = performance.now();
  }

  function setMode(mode) {
    cfg.mode = mode;
    if (isTyping()) lastTypeMode = mode;
    document.body.dataset.mode = mode;
    chipMode.textContent = modeLabel(mode);
    document.querySelectorAll('#segMode button').forEach((b) => {
      b.classList.toggle('is-on', b.dataset.mode === mode);
    });
    st.name = 'idle';
    st.sector = -1;
    st.group = 0;
    st.lock = false;
    layer.down = false;
    layer.pinned = false;
    selMode.active = false;
    selAnchor = -1;
    drag.active = false;
    caretNav.dir = -1;
    if (cmdWheel.open) closeCmd();
    if (langWheel.open) closeLangWheel();
    st.lastAct = performance.now();
    refreshLabels();
  }

  function modeLabel(mode) {
    if (mode && mode.startsWith('type-')) {
      const l = langByKey(langKeyOfMode(mode));
      return `Type ${l.code}`;
    }
    return {
      mouse: 'Mouse',
      nav: 'Place UI',
      quickbar: 'Quick bar',
      disabled: 'Paused'
    }[mode] || mode;
  }

  function setOpacity(v) {
    cfg.opacity = clamp(v, 0.25, 1);
    hud.style.setProperty('--hud-opacity', cfg.opacity.toFixed(2));
    $('#inOpacity').value = String(Math.round(cfg.opacity * 100));
    $('#outOpacity').textContent = Math.round(cfg.opacity * 100) + '%';
  }

  function setScale(v) {
    cfg.scale = clamp(v, 0.7, 1.4);
    hud.style.setProperty('--hud-scale', cfg.scale.toFixed(2));
    $('#inScale').value = String(Math.round(cfg.scale * 100));
    $('#outScale').textContent = Math.round(cfg.scale * 100) + '%';
    setPivot(pivot.x, pivot.y);
  }

  function refreshLabels() {
    const gs = ringGroups();
    groupLabels.forEach((el, i) => {
      el.textContent = gs ? gs[i].label : '';
    });
    if (!gs || st.name === 'idle') {
      charTexts.forEach((el) => {
        el.textContent = '';
        el.classList.add('is-dim');
        el.classList.remove('is-selected', 'is-compact');
      });
    }
    document.body.dataset.layer = String(layerNo());
  }

  function typeButton(kind) {
    if (cfg.mode === 'disabled') return;
    if (kind === 'back') { backspace(); return; }
    insertText(kind);
  }

  /* ---------------- D-pad ← = วงคำสั่งในวงใน (RS เลือก, ปล่อย = ยืนยัน) ---------------- */

  let dpadLeftPrev = false;

  function openCmd() {
    cmdWheel.open = true;
    cmdWheel.idx = -1;
    cmdWheel.sector = -1;
    cmdWheel.commit = 0;
    // ถ้า RS ถูกดันค้างตอนเปิด ต้องปล่อยกลับศูนย์ก่อน กันคำสั่งทำงานทันที
    cmdWheel.armed = mag(pad.rs) < P.rsRel;
    st.lastAct = performance.now();
  }

  function closeCmd() {
    cmdWheel.open = false;
    cmdWheel.idx = -1;
    cmdWheel.sector = -1;
    cmdWheel.commit = 0;
    cmdWheel.armed = false;
  }

  function toggleCmd() {
    if (langWheel.open) { closeLangWheel(); return; }
    if (cmdWheel.open) {
      closeCmd();
      // ปิดวงขณะ LS ยังค้าง → กันกลับไปเลือกกลุ่มอักษรทันที
      if (mag(pad.ls) >= P.lsRel) st.lock = true;
    } else {
      openCmd();
    }
  }

  function confirmCmd() {
    const idx = cmdWheel.idx;
    if (idx < 0) {
      toast('Move the right stick to choose');
      return;
    }
    const slot = CMD_SLOTS[idx];
    if (slot && slot.key === 'language' && !langSlotAvailable()) return;
    closeCmd();
    applyCmd(idx);
    if (mag(pad.ls) >= P.lsRel) st.lock = true;
  }

  /* ---------------- language wheel (level 2) ---------------- */

  function activeLangs() {
    return LANGS.filter((l) => langEnabled[l.key]);
  }

  // Languages other than the English base — this is what the "Other language" slot means
  function otherLangs() {
    return LANGS.filter((l) => l.key !== 'en' && langEnabled[l.key]);
  }

  function langSlotLabel() {
    const others = otherLangs();
    if (!others.length) return '';
    return others.length === 1 ? others[0].code : 'Lang';
  }

  function langSlotAvailable() {
    return otherLangs().length > 0;
  }

  // Mockup detection: a Thai Windows PC → English + Thai.
  // Production: enumerate GetKeyboardLayoutList + installed input languages.
  function detectLanguages() {
    const detected = ['en', 'th'];
    for (const l of LANGS) langEnabled[l.key] = detected.includes(l.key);
    return detected;
  }

  function layoutLangRing() {
    const n = Math.max(1, langList.length);
    const step = (Math.PI * 2) / n;
    for (let i = 0; i < 8; i++) {
      const show = i < n;
      langSectors[i].style.display = show ? '' : 'none';
      langLabels[i].style.display = show ? '' : 'none';
      if (!show) continue;
      const a0 = -Math.PI / 2 - step / 2 + i * step;
      const a1 = a0 + step;
      langSectors[i].setAttribute('d', sectorPath(34, 96, a0, a1));
      const mid = (a0 + a1) / 2;
      langLabels[i].setAttribute('x', (Math.cos(mid) * 66).toFixed(2));
      langLabels[i].setAttribute('y', (Math.sin(mid) * 66).toFixed(2));
    }
  }

  function openLangWheel() {
    closeCmd();
    langList = otherLangs().slice(0, 8);
    if (!langList.length) return;
    langWheel.open = true;
    langWheel.idx = Math.max(0, langList.findIndex((l) => l.key === langKeyOfMode(cfg.mode)));
    langWheel.sector = -1;
    langWheel.commit = 0;
    langWheel.armed = mag(pad.rs) < P.rsRel;
    layoutLangRing();
    st.lastAct = performance.now();
  }

  function closeLangWheel() {
    langWheel.open = false;
    langWheel.idx = -1;
    langWheel.sector = -1;
    langWheel.commit = 0;
    langWheel.armed = false;
  }

  function confirmLang() {
    const lang = langList[langWheel.idx];
    closeLangWheel();
    if (!lang) return;
    setMode('type-' + lang.key);
    toast(`${lang.name}`);
  }

  function updateLang(dt) {
    const rm = mag(pad.rs);
    if (!langWheel.armed) {
      if (rm < P.rsRel) langWheel.armed = true;
      return;
    }
    const n = Math.max(1, langList.length);
    if (rm >= P.rsAct) {
      const seg = 360 / n;
      const s = Math.round(angleFromUp(pad.rs) / seg) % n;
      if (s !== langWheel.sector) {
        langWheel.sector = s;
        langWheel.idx = s;
        langWheel.commit = 0;
      }
    } else if (langWheel.sector >= 0 && rm < P.rsRel) {
      langWheel.commit += dt;
      if (langWheel.commit >= P.commit) confirmLang();
    }
  }

  function updateWheels(dt) {
    // D-pad ← = open/close command wheel · closes language wheel first
    const left = !!gp.btn.left;
    if (left && !dpadLeftPrev) {
      if (langWheel.open) closeLangWheel();
      else if (cmdWheel.open) {
        closeCmd();
        if (mag(pad.ls) >= P.lsRel) st.lock = true;
      } else {
        openCmd();
      }
    }
    dpadLeftPrev = left;

    if (langWheel.open) { updateLang(dt); return; }
    if (!cmdWheel.open) return;

    // RS selects; LS has no effect in this wheel
    const rm = mag(pad.rs);
    if (!cmdWheel.armed) {
      if (rm < P.rsRel) cmdWheel.armed = true;
      return;
    }

    if (rm >= P.rsAct) {
      const s = sectorOf(pad.rs, cmdWheel.sector);
      if (s === 1 && !langSlotAvailable()) return;
      if (s !== cmdWheel.sector) {
        cmdWheel.sector = s;
        cmdWheel.idx = s;
        cmdWheel.commit = 0;
      }
    } else if (cmdWheel.sector >= 0 && rm < P.rsRel) {
      cmdWheel.commit += dt;
      if (cmdWheel.commit >= P.commit) confirmCmd();
    }
  }

  /* ---------------- state machine ---------------- */

  // เลื่อน LS ไปกลุ่มอื่นขณะยังดันค้าง = เปลี่ยนกลุ่มทันที (ไม่ต้องปล่อยแล้วดันใหม่)
  function retargetGroup(lm) {
    if (!isTyping() || lm < P.lsAct) return false;
    const g2 = sectorOf(pad.ls, st.group);
    if (g2 === st.group) return false;
    st.group = g2;
    st.sector = -1;
    st.hold = 0;
    st.lastAct = performance.now();
    if (st.name === 'char' || st.name === 'commit') st.name = 'group';
    return true;
  }

  function update(dt) {
    const now = performance.now();

    // รวม input: demo > gamepad (ถ้ามีการใช้งานล่าสุด) > keyboard
    const usePad = gp.name && now - gp.lastAct < 2500;

    if (demo.active) {
      target.ls = demo.ls;
      target.rs = demo.rs;
      pad.lt = false;
      pad.lb = false;
      pad.rb = false;
      pad.rt = false;
    } else if (usePad) {
      target.ls = gp.ls;
      target.rs = gp.rs;
      pad.lt = gp.lt;
      pad.lb = gp.lb;
      pad.rb = gp.rb;
      pad.rt = gp.rt;
    } else {
      target.ls = axisFromSet(kb.ls);
      target.rs = axisFromSet(kb.rs);
      pad.lt = kb.lt;
      pad.lb = kb.lb;
      pad.rb = kb.rb;
      pad.rt = kb.rt;
    }

    // ความนุ่มของแกน
    const k = 1 - Math.exp(-dt * 20);
    pad.ls.x += (target.ls.x - pad.ls.x) * k;
    pad.ls.y += (target.ls.y - pad.ls.y) * k;
    pad.rs.x += (target.rs.x - pad.rs.x) * k;
    pad.rs.y += (target.rs.y - pad.rs.y) * k;

    // LT/Shift: กดครั้งเดียว = ล็อกตัวพิมพ์ใหญ่ · กดอีกครั้ง = ตัวพิมพ์เล็ก
    if (isTyping() && !cmdWheel.open && !langWheel.open && pad.lt && !ltPrev) {
      st.shiftLock = !st.shiftLock;
      st.lastAct = now;
      toast(st.shiftLock ? 'UPPERCASE locked' : 'Lowercase');
    }
    ltPrev = pad.lt;

    updateWheels(dt);

    // วงคำสั่ง/วงภาษาเปิดอยู่ = หยุด gesture พิมพ์/เมาส์ชั่วคราว
    if (cmdWheel.open || langWheel.open) {
      st.name = 'idle';
      st.sector = -1;
      return;
    }

    if (cfg.mode === 'disabled') {
      st.name = 'idle';
      st.sector = -1;
      return;
    }

    // layering ด้วย RB/RT (เฉพาะ Type mode): ค้าง = ชั่วคราว, แตะ = ล็อก
    if (isTyping()) {
      const down = pad.rb || pad.rt;
      if (down && !layer.down) layer.at = now;
      if (!down && layer.down && now - layer.at < 220) layer.pinned = !layer.pinned;
      layer.down = down;
    } else {
      layer.down = false;
    }

    document.body.dataset.layer = String(layerNo());

    if (cfg.mode === 'mouse') {
      st.name = 'idle';
      st.sector = -1;
      updateMouse(dt);
      return;
    }

    if (cfg.mode === 'nav') {
      st.name = 'idle';
      st.sector = -1;
      updateNav(dt);
      return;
    }

    if (!isTyping() && !isQuick()) {
      st.name = 'idle';
      st.sector = -1;
      return;
    }

    const lm = mag(pad.ls);
    const rm = mag(pad.rs);

    // LS ไม่ถูกผลัก = RS ทำหน้าที่ปุ่มลูกศร (ย้าย caret / ขยาย selection)
    if (isTyping() && st.name === 'idle' && lm < P.lsRel) {
      updateCaretNav();
    } else {
      caretNav.dir = -1;
    }

    switch (st.name) {
      case 'idle': {
        if (st.lock) {
          if (lm < P.lsRel) st.lock = false;
          else break;
        }
        if (isQuick()) {
          if (rm >= P.rsAct) {
            st.name = 'char';
            st.group = 0;
            st.sector = sectorOf(pad.rs);
            st.hold = 0;
            st.lastAct = now;
          }
          break;
        }
        if (lm >= P.lsAct) {
          st.name = 'group';
          st.group = sectorOf(pad.ls);
          st.sector = -1;
          st.hold = 0;
          st.lock = false;
          st.lastAct = now;
        }
        break;
      }

      case 'group':
        if (lm < P.lsRel) { st.name = 'idle'; st.sector = -1; break; }
        retargetGroup(lm);
        if (rm >= P.rsAct) {
          st.name = 'char';
          st.sector = sectorOf(pad.rs);
          st.hold = 0;
          st.lastAct = now;
        }
        break;

      case 'char': {
        st.hold += dt;

        if (isQuick()) {
          if (rm >= P.rsAct) {
            const s = sectorOf(pad.rs, st.sector);
            if (s !== st.sector) { st.sector = s; st.hold = 0; }
            if (st.hold > P.cancel) { st.name = 'idle'; st.sector = -1; }
          } else if (rm < P.rsRel) {
            st.name = 'commit';
            st.commit = 0;
          }
          break;
        }

        // เลื่อน LS ไปกลุ่มอื่น = เปลี่ยนกลุ่ม แล้วกลับไปเลือกตัวอักษรในกลุ่มใหม่
        if (retargetGroup(lm)) break;

        if (lm < P.lsRel) { st.name = 'idle'; st.sector = -1; st.lock = true; break; }

        if (rm < P.rsRel) {
          st.name = 'commit';
          st.commit = 0;
          break;
        }
        if (rm >= P.rsAct) {
          const s = sectorOf(pad.rs, st.sector);
          if (s !== st.sector) { st.sector = s; st.hold = 0; }
          if (st.hold > P.cancel) { st.name = 'group'; st.sector = -1; }
        }
        break;
      }

      case 'commit':
        if (!isQuick() && lm < P.lsRel) { st.name = 'idle'; st.sector = -1; st.lock = true; break; }
        if (!isQuick() && retargetGroup(lm)) break;
        st.commit += dt;
        if (rm >= P.rsAct) { st.name = 'char'; st.hold = 0; break; }
        if (st.commit >= P.commit) {
          commitChar();
          if (isQuick()) {
            st.name = 'idle';
            st.sector = -1;
          } else {
            st.name = 'group';
            st.sector = -1;
          }
        }
        break;

      default:
        st.name = 'idle';
    }
  }

  /* ---------------- render ---------------- */

  let lastChipSel = '';
  let lastChipMod = '';

  // ช่องที่ยังใช้ไม่ได้ (คลิปบอร์ดว่าง / ไม่มี history) → จางในวง
  function cmdDisabled(slot) {
    return slot.kind === 'edit' && (
      (slot.key === 'paste' && !clipboard) ||
      (slot.key === 'undo' && history.undo.length === 0) ||
      (slot.key === 'redo' && history.redo.length === 0)
    );
  }

  function render() {
    const live = st.name !== 'idle';
    const cmdOpen = cmdWheel.open;
    const langOpen = langWheel.open;
    const gs = ringGroups();
    const typing = isTyping();

    const idleFor = performance.now() - st.lastAct;
    const visible = cmdOpen || langOpen || (!cfg.hudHidden &&
      (live || demo.active || !cfg.autohide || typedTimes.length === 0 || idleFor < 900));

    // idle = status pill only (progressive disclosure) unless learning mode
    const rest = !cfg.ringAlways && !cmdOpen && !langOpen && !demo.active &&
      st.name === 'idle' && !selMode.active && !selectionRange();

    hud.classList.toggle('is-live', visible);
    hud.classList.toggle('is-rest', rest);
    hud.classList.toggle('is-aim', cfg.aim);
    hud.classList.toggle('show-index', cfg.index);
    hud.classList.toggle('is-cmd', cmdOpen);
    hud.classList.toggle('is-lang', langOpen);
    hud.classList.toggle('is-mouse', cfg.mode === 'mouse');
    hud.classList.toggle('is-quick', isQuick());
    hud.classList.toggle('is-off', cfg.mode === 'disabled');

    hud.style.left = pivot.x + 'px';
    hud.style.top = pivot.y + 'px';

    // command wheel (inner ring) — "other language" slot: label is the second language
    const hasOtherLang = langSlotAvailable();
    if (cmdLabels[1]) cmdLabels[1].textContent = hasOtherLang ? langSlotLabel() : '';
    cmdSectors[1].classList.toggle('is-hidden', !hasOtherLang);
    cmdLabels[1].classList.toggle('is-hidden', !hasOtherLang);
    cmdSectors.forEach((p, i) => {
      const off = cmdDisabled(CMD_SLOTS[i]);
      p.classList.toggle('is-selected', cmdOpen && i === cmdWheel.idx);
      p.classList.toggle('is-disabled', cmdOpen && off);
    });
    cmdLabels.forEach((el, i) => {
      el.classList.toggle('is-selected', cmdOpen && i === cmdWheel.idx);
      el.classList.toggle('is-disabled', cmdOpen && cmdDisabled(CMD_SLOTS[i]));
    });

    // language wheel (level 2)
    langSectors.forEach((p, i) => {
      const has = langOpen && !!langList[i];
      p.classList.toggle('is-hidden', !has);
      p.classList.toggle('is-selected', has && i === langWheel.idx);
    });
    langLabels.forEach((el, i) => {
      const lang = langOpen ? langList[i] : null;
      el.textContent = lang ? lang.code : '';
      el.classList.toggle('is-selected', !!lang && i === langWheel.idx);
    });

    // groups
    groupPaths.forEach((p, i) => {
      p.classList.toggle('is-armed', live && !isQuick() && i === st.group);
    });
    groupLabels.forEach((el, i) => {
      el.textContent = gs ? gs[i].label : '';
      el.classList.toggle('is-empty', !gs);
      // โฟกัสกลุ่มที่ armed — กลุ่มอื่นจางลง
      el.classList.toggle('is-muted', !!gs && live && i !== st.group);
    });

    // chars
    const chars = gs ? gs[st.group].chars : [];
    charTexts.forEach((el, i) => {
      if (isQuick()) {
        el.textContent = live ? QUICK[i].label : '';
        el.classList.toggle('is-dim', !live);
        el.classList.add('is-compact');
      } else if (live && typing) {
        const ch = chars[i] || '';
        el.textContent = st.shiftLock ? ch.toUpperCase() : ch;
        el.classList.remove('is-dim');
        el.classList.remove('is-compact');
      } else {
        el.textContent = '';
        el.classList.add('is-dim');
        el.classList.remove('is-compact');
      }

      // ขีดกำกับสำหรับสระล่าง (ุ ู) — ขีดอยู่บน ตัวสระอยู่ล่าง
      const below = (!isQuick() && live && typing) ? isBelowMark(chars[i]) : false;
      const by = charBaseY[i];
      const guide = charGuides[i];
      el.setAttribute('y', by.toFixed(2));
      if (below) {
        guide.setAttribute('y1', (by - 7).toFixed(2));
        guide.setAttribute('y2', (by - 7).toFixed(2));
      }
      guide.classList.toggle('is-on', below);
      guide.classList.toggle('is-selected', below && live && i === st.sector);

      el.classList.toggle('is-selected', live && i === st.sector);
      charPaths[i].classList.toggle('is-selected', live && i === st.sector && st.name === 'char');
      charPaths[i].classList.toggle('is-commit', live && i === st.sector && st.name === 'commit');
    });

    // aim wedges (ทิศจริงของ Analog) — หมุนด้วย SVG transform รอบจุดศูนย์ (0,0)
    const lmA = mag(pad.ls);
    const rmA = mag(pad.rs);
    aimLS.setAttribute('transform', `rotate(${angleFromUp(pad.ls).toFixed(1)} 0 0)`);
    aimRS.setAttribute('transform', `rotate(${angleFromUp(pad.rs).toFixed(1)} 0 0)`);
    aimLS.style.opacity = cfg.aim && lmA > 0.12 ? String(clamp(lmA * 1.2, 0, 1)) : '0';
    aimRS.style.opacity = cfg.aim && rmA > 0.12 ? String(clamp(rmA * 1.2, 0, 1)) : '0';

    // commit dwell
    const prog = st.name === 'commit' ? clamp(st.commit / P.commit, 0, 1) : 0;
    commitRing.style.strokeDashoffset = String(169.6 * (1 - prog));

    // selected name in the center (command slot or language)
    const cmdSlot = cmdOpen && cmdWheel.idx >= 0 ? CMD_SLOTS[cmdWheel.idx] : null;
    const langSlot = langOpen ? langList[langWheel.idx] : null;
    cmdCenter.textContent = cmdSlot ? cmdSlot.label : '';

    // hint: next action only (not long tutorials)
    let hint;
    if (langOpen) hint = langWheel.sector >= 0 ? 'Release RS = switch · B cancel' : 'Move RS to choose';
    else if (cmdOpen) hint = cmdSlot ? 'Release RS = confirm · B cancel' : 'Move RS to choose';
    else if (cfg.mode === 'disabled') hint = 'Paused · D-pad ← to resume';
    else if (cfg.mode === 'mouse') hint = 'RS move · RB click · RT right-click';
    else if (cfg.mode === 'nav') hint = 'LS/RS move UI · R3 reset';
    else if (selMode.active) hint = 'RS extend · A confirm · B cancel';
    else if (selectionRange()) {
      const sel = selectionRange();
      hint = `${sel[1] - sel[0]} chars selected`;
    }
    else if (dirMode.enabled) hint = 'RS moves caret';
    else if (isQuick()) hint = live && st.sector >= 0 ? QUICK[st.sector].label : 'RS to choose';
    else if (live && gs) hint = `${gs[st.group].name}${layerNo() === 2 ? ' · Layer 2' : ''}`;
    else hint = '';
    hudHint.textContent = hint;

    // status: single line (mode · selection · modifiers)
    const sel = langOpen
      ? (langSlot ? langSlot.name : 'Language')
      : cmdOpen
        ? ''
        : isQuick()
          ? (live && st.sector >= 0 ? `${QUICK[st.sector].label}` : 'RS to choose')
          : (live && st.sector >= 0
            ? `RS ${st.sector + 1}`
            : live ? 'RS to choose' : (cfg.mode === 'mouse' ? 'Click enabled' : cfg.mode === 'nav' ? 'Place UI' : ''));
    if (sel !== lastChipSel) { chipSel.textContent = sel; lastChipSel = sel; }

    const mods = [
      layerNo() === 2 ? 'Layer 2' : '',
      isTyping() && st.shiftLock ? 'UPPER' : '',
      langByKey(langKeyOfMode(cfg.mode)).kind === 'ime' && isTyping() ? 'IME' : '',
      dirMode.enabled ? 'Arrows' : '',
      pad.lb ? 'Ctrl' : ''
    ].filter(Boolean).join(' · ');
    if (mods !== lastChipMod) { chipMod.textContent = mods; lastChipMod = mods; }

    updatePreview();
  }

  function updatePreview() {
    const el = document.getElementById('preview');
    if (!el || !caretEl) return;
    const ch = currentChar();
    const show = (st.name === 'char' || st.name === 'commit') && ch && isTyping();
    el.textContent = show ? ch : '';
    el.classList.toggle('preview--dwell', st.name === 'commit');
    caretEl.classList.toggle('caret--busy', st.name === 'char' || st.name === 'commit');
  }

  /* ---------------- demo ---------------- */

  async function runDemo() {
    const token = ++demo.token;
    demo.active = true;
    demo.ls = { x: 0, y: 0 };
    demo.rs = { x: 0, y: 0 };
    $('#btnDemo').textContent = '■ Stop';

    // [group, sector] ตาม keymap ใหม่ (กลุ่ม ↑ = สระ AEIOU + T N S)
    const script = [
      { mode: 'type-th', keys: [[0, 0], [4, 7], [4, 6], [0, 4]] },        // ก ข ค ง
      { mode: 'type-en', keys: [[0, 4], [4, 6]] },                        // (U T)
      { space: true },
      { mode: 'type-en', keys: [[1, 0], [0, 1], [1, 3], [1, 3], [0, 3]] }, // hello
      { space: true },
      { mode: 'type-en', keys: [[1, 7], [0, 3], [1, 1], [1, 3], [1, 2]] }  // world
    ];

    for (const step of script) {
      if (token !== demo.token) return;
      if (step.mode) setMode(step.mode);

      if (step.space) {
        demo.rs = { x: 0, y: 0 };
        demo.ls = { x: 0, y: 0 };
        await sleep(240);
        if (token !== demo.token) return;
        typeButton(' ');
        await sleep(180);
        continue;
      }

      for (const [grp, sec] of step.keys) {
        if (token !== demo.token) return;
        demo.ls = aimVec(grp * 45);
        demo.rs = aimVec(sec * 45);
        await sleep(300);
        if (token !== demo.token) return;
        demo.rs = { x: 0, y: 0 };
        await sleep(230);
        if (token !== demo.token) return;
        demo.ls = { x: 0, y: 0 };
        await sleep(140);
      }
    }

    demo.ls = { x: 0, y: 0 };
    demo.rs = { x: 0, y: 0 };
    await sleep(400);
    if (token === demo.token) stopDemo();
  }

  function stopDemo() {
    demo.token++;
    demo.active = false;
    demo.ls = { x: 0, y: 0 };
    demo.rs = { x: 0, y: 0 };
    const btn = $('#btnDemo');
    if (btn) btn.textContent = '▶ Play demo';
    st.lastAct = performance.now() - 5000;
  }

  function toggleDemo() {
    if (demo.active) stopDemo();
    else runDemo();
  }

  /* ---------------- settings window ---------------- */

  const mapEdit = { lang: 'ru', gi: -1, ci: -1 };

  /* ---------- controller map (Keymap tab reference) ----------
     Both pads share one physical layout: LS/RS + D-pad ← + four face buttons
     + LB/RB/LT/RT. Percentages below are measured on the source PNGs. */

  const DIR_GLYPHS = ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'];

  const PADS = {
    xbox: {
      label: 'Xbox',
      img: 'assets/pad-xbox-front.png',
      alt: 'Xbox controller — front',
      pins: [
        { key: 'ls', label: 'LS', tone: 'ls', x: 24.2, y: 34.3, d: 19 },
        { key: 'rs', label: 'RS', tone: 'rs', x: 62.3, y: 50.9, d: 19 },
        { key: 'dleft', label: '', tone: 'dpad', x: 32, y: 50.5, d: 7.8 },
        { key: 'a', label: '', tone: 'green', x: 76.8, y: 42.9, d: 9 },
        { key: 'b', label: '', tone: 'red', x: 84, y: 35.4, d: 9 },
        { key: 'x', label: '', tone: 'blue', x: 69.6, y: 35.2, d: 9 },
        { key: 'y', label: '', tone: 'yellow', x: 76.8, y: 27.9, d: 9 }
      ]
    },
    ps: {
      label: 'DualSense',
      img: 'assets/pad-dualsense-front.png',
      alt: 'DualSense controller — front',
      pins: [
        { key: 'ls', label: 'LS', tone: 'ls', x: 33.4, y: 54, d: 18.5 },
        { key: 'rs', label: 'RS', tone: 'rs', x: 64.5, y: 54, d: 18.5 },
        { key: 'dleft', label: '', tone: 'dpad', x: 18.3, y: 38.2, d: 7.5 },
        { key: 'a', label: '', tone: 'blue', x: 81, y: 46.2, d: 10 },
        { key: 'b', label: '', tone: 'red', x: 88.3, y: 38.3, d: 10 },
        { key: 'x', label: '', tone: 'pink', x: 73.9, y: 39.3, d: 10 },
        { key: 'y', label: '', tone: 'green', x: 81, y: 32, d: 10 }
      ]
    }
  };

  // chip = primary label on the current pad · sub = the same button on the other pad
  // tone follows the physical button colour of the selected pad
  const PAD_LEGEND = [
    { key: 'ls', tone: 'ls', chip: 'LS', text: 'Left stick — <b>group</b> (grid rows 1–8)' },
    { key: 'rs', tone: 'rs', chip: 'RS', text: 'Right stick — <b>character</b> (grid columns 1–8)' },
    { key: 'dleft', tone: 'dpad', chip: '←', text: 'D-pad left — <b>command wheel</b> (mode · copy · paste · undo · redo)' },
    { key: 'a', tone: { xbox: 'green', ps: 'blue' }, chip: { xbox: 'A', ps: '✕' }, sub: { xbox: '✕', ps: 'A' }, text: '<b>Space</b>' },
    { key: 'b', tone: { xbox: 'red', ps: 'red' }, chip: { xbox: 'B', ps: '○' }, sub: { xbox: '○', ps: 'B' }, text: '<b>Backspace</b>' },
    { key: 'x', tone: { xbox: 'blue', ps: 'pink' }, chip: { xbox: 'X', ps: '□' }, sub: { xbox: '□', ps: 'X' }, text: '<b>Enter</b>' },
    { key: 'y', tone: { xbox: 'yellow', ps: 'green' }, chip: { xbox: 'Y', ps: '△' }, sub: { xbox: '△', ps: 'Y' }, text: '<b>Tab</b>' }
  ];

  const padView = { key: 'xbox' };

  function hotPadTarget(key, on) {
    document
      .querySelectorAll(`.padpin[data-target="${key}"], .padrow[data-target="${key}"]`)
      .forEach((el) => el.classList.toggle('is-hot', on));
  }

  function renderPadMap() {
    const padCfg = PADS[padView.key];
    const toneOf = (t) => (typeof t === 'string' ? t : t[padView.key]);

    const img = $('#padImg');
    img.src = padCfg.img;
    img.alt = padCfg.alt;

    const pinHost = $('#padPins');
    pinHost.innerHTML = '';
    for (const pin of padCfg.pins) {
      const el = document.createElement('span');
      el.className = `padpin tone-${toneOf(pin.tone)}`;
      el.dataset.target = pin.key;
      el.style.setProperty('--x', pin.x + '%');
      el.style.setProperty('--y', pin.y + '%');
      el.style.setProperty('--d', pin.d + '%');
      if (pin.label) el.dataset.label = pin.label;
      pinHost.appendChild(el);
    }

    const legend = $('#padLegend');
    legend.innerHTML = '';
    for (const row of PAD_LEGEND) {
      const li = document.createElement('li');
      li.className = 'padrow';
      li.dataset.target = row.key;

      const chip = document.createElement('span');
      chip.className = `padrow__chip tone-${toneOf(row.tone)}`;
      chip.textContent = typeof row.chip === 'string' ? row.chip : row.chip[padView.key];
      if (row.sub) {
        const sub = document.createElement('i');
        sub.textContent = row.sub[padView.key];
        chip.appendChild(sub);
      }

      const text = document.createElement('span');
      text.className = 'padrow__text';
      text.innerHTML = row.text;

      li.append(chip, text);
      legend.appendChild(li);
    }

    $('#padNote').textContent =
      padView.key === 'ps'
        ? 'DualSense ✕ ○ □ △ sit in the same positions as A B X Y on an Xbox pad.'
        : 'Xbox A B X Y = DualSense ✕ ○ □ △ in the same positions.';
  }

  function setPad(key) {
    if (!PADS[key]) return;
    padView.key = key;
    document.querySelectorAll('#padSwitch button').forEach((b) => {
      b.classList.toggle('is-on', b.dataset.pad === key);
    });
    renderPadMap();
  }

  function setTab(tab) {
    document.querySelectorAll('#settingsTabs button').forEach((b) => {
      b.classList.toggle('is-on', b.dataset.tab === tab);
    });
    document.querySelectorAll('.settings__pane').forEach((p) => {
      p.hidden = p.dataset.panel !== tab;
    });
    if (tab === 'keymap') { renderPadMap(); renderMapLangSelect(); renderMapGrid(); }
    if (tab === 'languages') renderLangList();
  }

  function openSettings() {
    $('#settings').hidden = false;
    setTab('general');
  }

  function closeSettings() {
    $('#settings').hidden = true;
  }

  function renderLangList() {
    const host = $('#langList');
    host.innerHTML = '';
    for (const l of LANGS) {
      const row = document.createElement('label');
      row.className = 'langrow';
      const kind = l.kind === 'ime' ? 'IME · keys' : 'Layout';
      const tag = l.sample
        ? '<i class="langrow__tag">sample</i>'
        : '<i class="langrow__tag langrow__tag--on">detected</i>';
      row.innerHTML =
        `<span class="langrow__code">${l.code}</span>` +
        `<span class="langrow__name">${l.name}${tag}</span>` +
        `<span class="langrow__kind ${l.kind === 'ime' ? 'langrow__kind--ime' : ''}">${kind}</span>` +
        `<input type="checkbox" ${langEnabled[l.key] ? 'checked' : ''} ${l.base ? 'disabled' : ''} />`;
      row.querySelector('input').addEventListener('change', (e) => {
        langEnabled[l.key] = e.target.checked;
        if (!e.target.checked && langKeyOfMode(cfg.mode) === l.key) setMode('type-en');
        toast(`${l.name} ${e.target.checked ? 'enabled' : 'disabled'}`);
      });
      host.appendChild(row);
    }
  }

  function renderMapLangSelect() {
    const sel = $('#mapLang');
    sel.innerHTML = '';
    for (const l of LANGS) {
      const o = document.createElement('option');
      o.value = l.key;
      o.textContent = `${l.code} — ${l.name}${l.kind === 'ime' ? ' (IME)' : ''}`;
      sel.appendChild(o);
    }
    sel.value = mapEdit.lang;
  }

  function renderMapGrid() {
    const lang = langByKey(mapEdit.lang);
    const grid = $('#mapGrid');
    const hint = $('#mapHint');
    grid.innerHTML = '';
    grid.classList.toggle('is-readonly', lang.kind === 'ime');
    const gs = LAYERS['type-' + lang.key][0];
    const ov = langOverrides[lang.key] || {};

    // แถวหัว: คอลัมน์ = ทิศ RS (ลำดับตัวอักษรในกลุ่ม)
    const head = document.createElement('div');
    head.className = 'mapgrid__row mapgrid__row--head';
    const lsHead = document.createElement('div');
    lsHead.className = 'mapgrid__label';
    lsHead.textContent = 'LS';
    head.appendChild(lsHead);
    DIR_GLYPHS.forEach((dir, i) => {
      const d = document.createElement('div');
      d.className = 'mapgrid__dir';
      d.textContent = dir;
      d.title = `RS ${i + 1} · ${dir}`;
      head.appendChild(d);
    });
    grid.appendChild(head);

    gs.forEach((gr, gi) => {
      const row = document.createElement('div');
      row.className = 'mapgrid__row';
      const lab = document.createElement('div');
      lab.className = 'mapgrid__label';
      const arrow = document.createElement('i');
      arrow.textContent = DIR_GLYPHS[gi] || '';
      const name = document.createElement('b');
      name.textContent = gr.label;
      lab.append(arrow, name);
      lab.title = `LS ${DIR_GLYPHS[gi]} — ${gr.name}`;
      row.appendChild(lab);
      gr.chars.forEach((ch, ci) => {
        const cell = document.createElement('button');
        cell.type = 'button';
        cell.className = 'mapcell';
        const edited = ov[gi + '-' + ci] != null;
        cell.classList.toggle('is-edited', edited);
        cell.classList.toggle('is-listening', mapEdit.gi === gi && mapEdit.ci === ci);
        cell.textContent = ch || '·';
        cell.title = `${gr.label} · RS ${DIR_GLYPHS[ci]}`;
        if (lang.kind !== 'ime') {
          cell.addEventListener('click', () => {
            mapEdit.gi = gi;
            mapEdit.ci = ci;
            renderMapGrid();
            hint.textContent = 'Press a key to map this cell… (Esc cancels)';
          });
        }
        row.appendChild(cell);
      });
      grid.appendChild(row);
    });
    if (lang.kind === 'ime') {
      hint.textContent = 'IME language: the HUD sends physical keys and Windows IME composes. Mapping is the QWERTY key grid (read-only).';
    } else if (!hint.textContent.startsWith('Press')) {
      hint.textContent = 'Rows = LS group · columns = RS direction. Click a cell, then press a key. Esc cancels.';
    }
  }

  function applyMapKey(key) {
    if (!mapEdit.lang || mapEdit.gi < 0) return;
    const ov = langOverrides[mapEdit.lang] || (langOverrides[mapEdit.lang] = {});
    ov[mapEdit.gi + '-' + mapEdit.ci] = key;
    mapEdit.gi = -1;
    mapEdit.ci = -1;
    LAYERS = buildLayers();
    refreshLabels();
    renderMapGrid();
    toast(`Mapped to "${key}"`);
  }

  function resetLangMap(key) {
    delete langOverrides[key];
    LAYERS = buildLayers();
    refreshLabels();
    renderMapGrid();
    toast(`${langByKey(key).name} mapping reset`);
  }

  function resetAllMaps() {
    for (const k of Object.keys(langOverrides)) delete langOverrides[k];
    LAYERS = buildLayers();
    refreshLabels();
    renderMapGrid();
    toast('All mappings reset');
  }

  function resetSettings(scope) {
    if (scope === 'general' || scope === 'all') {
      setOpacity(0.92);
      setScale(1);
      cfg.autohide = true;
      $('#inAutohide').checked = true;
    }
    if (scope === 'hud' || scope === 'all') {
      cfg.aim = false;
      cfg.index = false;
      cfg.ringAlways = false;
      $('#inAim').checked = false;
      $('#inIndex').checked = false;
      $('#inRing').checked = false;
    }
    if (scope === 'languages' || scope === 'all') {
      detectLanguages();
      renderLangList();
    }
    if (scope === 'all') resetAllMaps();
    if (scope === 'all') {
      setMode('type-en');
      st.shiftLock = false;
      st.lastAct = performance.now();
    }
    toast(scope === 'all' ? 'All settings reset to defaults' : 'Section reset');
  }

  function wireSettings() {
    $('#btnSettings').addEventListener('click', openSettings);
    $('#btnSettingsClose').addEventListener('click', closeSettings);
    document.querySelectorAll('#settingsTabs button').forEach((b) => {
      b.addEventListener('click', () => setTab(b.dataset.tab));
    });
    document.querySelectorAll('[data-reset]').forEach((b) => {
      b.addEventListener('click', () => resetSettings(b.dataset.reset));
    });
    $('#btnResetAll').addEventListener('click', () => resetSettings('all'));
    $('#btnRescanLangs').addEventListener('click', () => {
      const found = detectLanguages();
      renderLangList();
      if (!langEnabled[langKeyOfMode(cfg.mode)]) setMode('type-en');
      toast(`Detected: ${found.map((k) => langByKey(k).name).join(', ')}`);
    });
    $('#mapLang').addEventListener('change', (e) => {
      mapEdit.lang = e.target.value;
      mapEdit.gi = -1;
      mapEdit.ci = -1;
      renderMapGrid();
    });
    $('#btnResetLang').addEventListener('click', () => resetLangMap(mapEdit.lang));
    $('#btnResetMaps').addEventListener('click', resetAllMaps);

    // controller reference: switch pad + link rings and legend rows on hover
    document.querySelectorAll('#padSwitch button').forEach((b) => {
      b.addEventListener('click', () => setPad(b.dataset.pad));
    });
    const padmap = $('#padSwitch').closest('.padmap');
    padmap.addEventListener('pointerover', (e) => {
      const hit = e.target.closest('[data-target]');
      if (!hit || (e.relatedTarget && hit.contains(e.relatedTarget))) return;
      hotPadTarget(hit.dataset.target, true);
    });
    padmap.addEventListener('pointerout', (e) => {
      const hit = e.target.closest('[data-target]');
      if (!hit || (e.relatedTarget && hit.contains(e.relatedTarget))) return;
      hotPadTarget(hit.dataset.target, false);
    });
  }

  function wirePanel() {
    document.querySelectorAll('#segMode button').forEach((b) => {
      b.addEventListener('click', () => setMode(b.dataset.mode));
    });

    $('#inOpacity').addEventListener('input', (e) => setOpacity(Number(e.target.value) / 100));
    $('#inScale').addEventListener('input', (e) => setScale(Number(e.target.value) / 100));

    $('#inAim').addEventListener('change', (e) => { cfg.aim = e.target.checked; });
    $('#inRing').addEventListener('change', (e) => {
      cfg.ringAlways = e.target.checked;
      st.lastAct = performance.now();
      toast(cfg.ringAlways ? 'Ring always on (learning)' : 'Ring appears on LS');
    });
    $('#inIndex').addEventListener('change', (e) => { cfg.index = e.target.checked; });
    $('#inAutohide').addEventListener('change', (e) => {
      cfg.autohide = e.target.checked;
      st.lastAct = performance.now();
    });
    $('#btnDemo').addEventListener('click', toggleDemo);
    $('#btnClear').addEventListener('click', () => {
      docText = '';
      caretPos = 0;
      typedTimes.length = 0;
      docScroll = 0;
      renderDoc();
      st.lastAct = performance.now();
    });

    window.addEventListener('gamepadconnected', (e) => {
      gp.name = shortPadName(e.gamepad.id);
      updateStats();
      st.lastAct = performance.now();
      toast('Controller connected · D-pad ← opens the command wheel');
    });
    window.addEventListener('gamepaddisconnected', () => {
      gp.name = null;
      updateStats();
    });

    window.addEventListener('resize', () => {
      setPointer(pointer.x, pointer.y);
      setPivot(pivot.x, pivot.y);
    });
  }

  /* ---------------- main loop ---------------- */

  let lastT = performance.now();

  function frame(now) {
    const dt = Math.min(0.05, (now - lastT) / 1000);
    lastT = now;

    pollGamepad();
    update(dt);
    render();

    raf(frame);
  }

  /* ---------------- boot ---------------- */

  buildRings();
  setMode('type-en');
  setOpacity(0.92);
  setScale(1);
  wirePanel();
  wireSettings();
  renderDoc();
  setPivot(window.innerWidth * 0.62, window.innerHeight * 0.55);
  setPointer(window.innerWidth * 0.5, window.innerHeight * 0.5);
  st.lastAct = performance.now();
  hudHint.setAttribute('y', '-186');

  /* ---------------- debug hook (ใช้โดย tools/tests/hud-smoke.mjs) ----------------
     ไม่มีผลกับการใช้งานปกติ: เปิดให้ test เรียก step() แทน rAF ได้ */
  window.__rni = {
    cfg, st, pad, kb, pointer, pivot, layer, QUICK, history, dirMode, CMD_SLOTS, LANGS,
    langEnabled, langOverrides, langWheel,
    cmd: cmdWheel,
    setMode, applyCmd, toggleCmd, openCmd, closeCmd, confirmCmd, setPivot, resetPivot,
    openLangWheel, closeLangWheel, confirmLang, resetSettings,
    pads: PADS, padView, setPad, renderPadMap,
    step: (dt = 1 / 60) => { update(dt); render(); },
    setPointer,
    clipboard: () => clipboard,
    chars: () => charTexts.map((el) => el.textContent),
    labels: () => groupLabels.map((el) => el.textContent),
    doc: () => ({ caret: caretPos, anchor: selAnchor, sel: !!selectionRange(), selMode: selMode.active, len: docText.length })
  };

  raf(frame);
})();
