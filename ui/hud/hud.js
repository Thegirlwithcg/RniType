// P1 contract harness — วาด HudState จาก state player เท่านั้น
// ไม่มี logic ของแป้น/gesture ที่นี่ (นั่นคือ P2/P4)

if (new URLSearchParams(location.search).has('preview')) {
  document.documentElement.classList.add('preview-bg');
}

const hud = document.getElementById('hud');
const innerEl = document.getElementById('inner');
const outerEl = document.getElementById('outer');
const previewEl = document.getElementById('preview');
const previewChar = document.getElementById('previewChar');
const previewCount = document.getElementById('previewCount');
const pillMode = document.getElementById('pillMode');
const pillLang = document.getElementById('pillLang');
const pillBadges = document.getElementById('pillBadges');
const pivotEl = document.getElementById('pivot');
const pointerEl = document.getElementById('pointer');

const MODE_LABEL = {
  'type-en': 'Type EN',
  'type-th': 'Type TH',
  'type-other': 'Type',
  cmd: 'Command',
  lang: 'Language',
  mouse: 'Mouse',
  nav: 'Place UI',
  disabled: 'พักอยู่',
};

let lastJson = '';
let lastState = null;

async function poll() {
  try {
    const res = await fetch('/api/state', { cache: 'no-store' });
    if (res.ok) {
      const text = await res.text();
      if (text !== lastJson) {
        lastJson = text;
        lastState = JSON.parse(text);
        render(lastState);
      }
    }
  } catch {
    // state player ปิดอยู่ — เงียบไว้
  }
  setTimeout(poll, 120);
}

function render(s) {
  hud.dataset.mode = s.mode;
  hud.dataset.visible = String(s.visible && !s.hudHidden);

  renderSlots(innerEl, s.inner, 62);
  renderSlots(outerEl, s.outer, 150);

  const showPreview = ['char', 'commit', 'repeat'].includes(s.state) && s.preview?.char;
  previewEl.hidden = !showPreview;
  previewChar.textContent = s.preview?.char ?? '';
  previewCount.textContent = s.preview?.count > 0 ? `×${s.preview.count}` : '';

  pillMode.textContent = MODE_LABEL[s.mode] ?? s.mode;
  pillLang.textContent = s.language ? s.language.code.toUpperCase() : '';

  const badges = [...(s.status?.badges ?? [])];
  if (s.layer > 1) badges.push(`layer ${s.layer}`);
  if (s.shiftLock) badges.push('shift');
  pillBadges.replaceChildren(
    ...[...new Set(badges)].map((text) => {
      const el = document.createElement('span');
      el.className = 'pill-badge';
      el.textContent = text;
      return el;
    }),
  );

  pivotEl.style.left = `${s.pivot.x}px`;
  pivotEl.style.top = `${s.pivot.y}px`;

  const showPointer = s.mode === 'mouse' && s.pointer?.visible;
  pointerEl.hidden = !showPointer;
  if (showPointer) {
    pointerEl.style.left = `${s.pointer.x}px`;
    pointerEl.style.top = `${s.pointer.y}px`;
  }

  document.title = `RniType HUD — ${s.mode}/${s.state}`;
}

function renderSlots(container, ring, radius) {
  if (!ring || ring.kind === 'none' || ring.labels.length === 0) {
    container.replaceChildren();
    return;
  }

  const nodes = ring.labels.map((label, i) => {
    const angle = (i * Math.PI) / 4;
    const x = Math.sin(angle) * radius;
    const y = -Math.cos(angle) * radius;

    const el = document.createElement('div');
    el.className = 'slot';
    el.style.left = `calc(50% + ${x.toFixed(1)}px)`;
    el.style.top = `calc(50% + ${y.toFixed(1)}px)`;
    if (i === ring.active) el.classList.add('active');
    if (ring.active >= 0 && i !== ring.active) el.classList.add('dim');

    const text = document.createElement('span');
    text.textContent = label.text;
    el.append(text);

    if (label.sub) {
      const sub = document.createElement('span');
      sub.className = 'sub';
      sub.textContent = label.sub;
      el.append(sub);
    }
    if (label.badge) {
      const badge = document.createElement('span');
      badge.className = 'badge';
      badge.textContent = label.badge;
      el.append(badge);
    }
    return el;
  });

  container.replaceChildren(...nodes);
}

addEventListener('keydown', (event) => {
  if (event.key.toLowerCase() === 'n') {
    fetch('/api/next', { method: 'POST' }).catch(() => {});
  }
});

poll();
