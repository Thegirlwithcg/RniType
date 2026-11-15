/* ============================================================
   RniType — font stacks ที่แอปรองรับ (แหล่งเดียวของ HUD + Settings)
   id ต้องตรงกับ contract `ui.hudFontFamily` และไฟล์ใน ui/shared/fonts.css
   ============================================================ */
(() => {
  'use strict';

  const stacks = {
    'ibm-plex-sans-thai':
      '"IBM Plex Sans Thai", "Noto Sans Thai", "Segoe UI", system-ui, sans-serif',
    'noto-sans-thai':
      '"Noto Sans Thai", "IBM Plex Sans Thai", "Segoe UI", system-ui, sans-serif',
    system: '"Segoe UI", "Leelawadee UI", system-ui, sans-serif',
  };

  window.RniFonts = {
    default: 'ibm-plex-sans-thai',
    stacks,
    ids: Object.keys(stacks),
  };
})();
