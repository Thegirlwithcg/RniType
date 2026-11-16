//! Runtime session — ต่อของจริง: `PadState` → [`RuntimeEngine`] → injector → `hud:state`
//!
//! โมดูลนี้ไม่รู้จัก Tauri/gilrs เลย (ทดสอบได้ด้วย `NullInjector` + pad ที่ป้อนเอง)
//! หน้าที่มี 3 อย่าง
//! 1. เรียก `RuntimeEngine::tick` แล้วแปลง `Action` เป็นการส่งออกจริง
//! 2. เก็บสถานะที่ HUD ต้องใช้ (pivot/pointer/สถิติ)
//! 3. สร้าง `HudState` (contract v1) จาก engine — HUD ไม่ต้องคำนวณเอง
//!
//! ลำดับ `Action` ที่ไม่เข้ากล่อง injector:
//! - `Command::Copy/Paste/Undo/Redo` → คีย์ลัด Ctrl+C/V/Z/Y (plan แปลงเป็น VK)
//! - `Command::Mouse/TypeEn/PlaceUi` · `SetMode` · `SetLang` → engine เปลี่ยนเองแล้ว
//! - `MovePivot` / `ResetPivot` / `RecenterPointer` → ขยับ pivot/pointer ของ HUD (Nav/Mouse math อยู่ใน core)

use rnitype_core::keymap::{group_names_for, label_of};
use rnitype_core::normalizer::Dir8;
use rnitype_core::runtime::{LangSlot, RuntimeEngine, WheelKind};
use rnitype_core::{
    Action, Command, GestureState, Key, KeyPress, Lang, Mode, MouseButton, PadState, QuickAction,
};
use rnitype_win::Injector;
use rnitype_win::target::BlockReason;
use serde_json::{Value, json};

/// ค่าที่ HUD ต้องใช้แต่ไม่ใช่ state ของ engine (มาจาก config)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudStyle {
    pub scale: f64,
    pub opacity: f64,
}

impl Default for HudStyle {
    fn default() -> Self {
        Self {
            scale: 1.0,
            opacity: 0.92,
        }
    }
}

pub struct Session {
    engine: RuntimeEngine,
    injector: Box<dyn Injector + Send>,
    /// ภาษาที่ผู้ใช้เปิดไว้ (config `languages`) — ใช้สร้างวงภาษา
    enabled: Vec<Lang>,
    /// ตำแหน่ง HUD (พิกัดหน้าจอจริง; แปลงเป็นพิกัดหน้าต่างตอนสร้าง HudState)
    pivot: (f32, f32),
    /// ตำแหน่ง pivot เริ่มต้น (RS click ในโหมด Nav = กลับมาที่นี่)
    pivot_home: (f32, f32),
    pointer: (f32, f32),
    /// กลางจอหลัก (RS click ในโหมด Mouse = pointer กลับมาที่นี่)
    pointer_home: (f32, f32),
    /// ขอบเขต pivot ที่ปลอดภัย (พิกัดหน้าจอ) — `None` = ไม่จำกัด
    pivot_clamp: Option<((f32, f32), (f32, f32))>,
    /// เหตุผลที่หยุดส่ง input ชั่วคราว (exclusion / UIPI) — HUD แสดงเป็น `disabled`
    guard: Option<BlockReason>,
    /// mode ที่ต้องกลับไปหลังพักจาก tray
    paused_mode: Option<Mode>,
    /// จำนวนคีย์ที่ส่งออกแล้ว (log/สถิติ)
    sent: u32,
    /// ตัวอักษรที่พิมพ์สำเร็จทั้ง session (WPM lifetime)
    chars: u64,
    /// เวลาของ commit ในหน้าต่าง 60 วิ (คำนวณ WPM)
    commits: std::collections::VecDeque<std::time::Instant>,
}

impl Session {
    /// ทางลัดสำหรับ test/dev: pivot และ pointer เริ่มที่จุดเดียวกัน
    #[allow(dead_code)] // runtime ใช้ `with_homes` (bin crate ไม่เห็นการเรียกจาก test)
    pub fn new(
        engine: RuntimeEngine,
        injector: Box<dyn Injector + Send>,
        enabled: Vec<Lang>,
        pivot: (f32, f32),
    ) -> Self {
        Self::with_homes(engine, injector, enabled, pivot, pivot)
    }

    /// สร้าง session พร้อมจุดกลับบ้านของ pivot และ pointer (ปกติ = กลางจอหลัก)
    pub fn with_homes(
        engine: RuntimeEngine,
        injector: Box<dyn Injector + Send>,
        enabled: Vec<Lang>,
        pivot: (f32, f32),
        pointer_home: (f32, f32),
    ) -> Self {
        let mut session = Self {
            engine,
            injector,
            enabled,
            pivot,
            pivot_home: pivot,
            pointer: pointer_home,
            pointer_home,
            pivot_clamp: None,
            guard: None,
            paused_mode: None,
            sent: 0,
            chars: 0,
            commits: std::collections::VecDeque::new(),
        };
        session.refresh_languages();
        session
    }

    pub fn engine(&self) -> &RuntimeEngine {
        &self.engine
    }

    pub fn sent(&self) -> u32 {
        self.sent
    }

    /// ตัวอักษรที่พิมพ์สำเร็จทั้ง session
    pub fn chars(&self) -> u64 {
        self.chars
    }

    /// WPM จาก commits ใน 60 วินาทีสุดท้าย (5 ตัวอักษร = 1 คำ)
    pub fn wpm(&self) -> f64 {
        crate::stats::wpm(self.commits.len(), 60.0)
    }

    /// เก็บ commit ของตัวอักษร + ตัดรายการที่เก่ากว่า 60 วิ
    fn note_commit(&mut self) {
        self.chars = self.chars.saturating_add(1);
        let now = std::time::Instant::now();
        self.commits.push_back(now);
        if let Some(cutoff) = now.checked_sub(std::time::Duration::from_secs(60)) {
            while self.commits.front().is_some_and(|time| *time < cutoff) {
                self.commits.pop_front();
            }
        }
    }

    /// ตัวเลขสำหรับ test (Settings อ่าน payload ที่ loop ประกอบเอง)
    #[cfg(test)]
    pub fn stats(&self) -> Value {
        json!({
            "chars": self.chars,
            "wpm": (self.wpm() * 10.0).round() / 10.0,
            "actions": self.sent,
        })
    }

    /// ตั้งตำแหน่ง pointer ที่ HUD ใช้ (ตอนเริ่มแอป = ตำแหน่งจริงของ OS)
    pub fn set_pointer(&mut self, pointer: (f32, f32)) {
        self.pointer = pointer;
    }

    /// จำกัด pivot ให้วง HUD ยังอยู่ใน virtual desktop (ADR-23)
    ///
    /// `min`/`max` เป็นพิกัดหน้าจอของจุดศูนย์กลางที่ยังปลอดภัย (เว้นรัศมีวงแล้ว)
    pub fn set_pivot_clamp(&mut self, min: (f32, f32), max: (f32, f32)) {
        self.pivot_clamp = Some((min, (max.0.max(min.0), max.1.max(min.1))));
        self.pivot = self.clamp_pivot(self.pivot);
    }

    fn clamp_pivot(&self, point: (f32, f32)) -> (f32, f32) {
        match self.pivot_clamp {
            Some(((minx, miny), (maxx, maxy))) => {
                (point.0.clamp(minx, maxx), point.1.clamp(miny, maxy))
            }
            None => point,
        }
    }

    /// ตั้ง/ล้าง guard จาก exclusion list หรือ UIPI (ADR-24)
    ///
    /// - เข้า guard: ปล่อยทุกอย่างที่ค้าง + ยกเลิก gesture (ไม่แตะ mode เดิม)
    /// - ออก guard: ยกเลิก gesture อีกครั้ง เพื่อบังคับให้ปล่อย LS ก่อนพิมพ์ต่อ
    pub fn set_guard(&mut self, reason: Option<BlockReason>) {
        if reason.is_some() && self.guard.is_none() {
            self.release_all();
            let mode = self.engine.mode();
            if mode != Mode::Disabled {
                self.engine.set_mode(mode);
            }
        } else if reason.is_none() && self.guard.is_some() {
            let mode = self.engine.mode();
            if mode != Mode::Disabled {
                self.engine.set_mode(mode);
            }
        }
        self.guard = reason;
    }

    pub fn guard(&self) -> Option<BlockReason> {
        self.guard
    }

    /// พักการพิมพ์ (tray): จำ mode เดิมไว้แล้วเข้า `Disabled`
    ///
    /// ถ้าถูกปิดด้วย emergency อยู่แล้ว (`disabled_remaining_ms > 0`) จะไม่รีเซ็ตตัวจับเวลา
    pub fn pause(&mut self) {
        self.release_all();
        let mode = self.engine.mode();
        if mode != Mode::Disabled {
            self.paused_mode = Some(mode);
            self.engine.set_mode(Mode::Disabled);
        }
    }

    /// กลับมาพิมพ์หลังพัก — คืน mode เดิม
    pub fn unpause(&mut self) {
        if let Some(mode) = self.paused_mode.take() {
            self.engine.set_mode(mode);
        } else if self.engine.mode() == Mode::Disabled && self.engine.disabled_remaining_ms() == 0
        {
            self.engine.set_mode(Mode::Type);
        }
    }

    /// 1 เฟรมของ engine — คืน action ที่ "ส่งออกจริง" (สำหรับ log/ทดสอบ)
    pub fn step(&mut self, pad: &PadState, dt_ms: u32) -> Vec<Action> {
        let mut out = Vec::new();
        let sticky_before = self.engine.sticky_drag();
        if let Some(action) = self.engine.tick(pad, dt_ms) {
            self.apply(action, &mut out);
        }

        // sticky drag ถูกล้างโดย engine (สลับ mode / emergency / จอยหลุด)
        // ต้องปล่อยปุ่มซ้ายจริง ไม่งั้นเมาส์ค้างกดทั้งเครื่อง
        if sticky_before && !self.engine.sticky_drag() {
            let already_released = out.iter().any(|action| {
                matches!(
                    action,
                    Action::MouseButton {
                        button: MouseButton::Left,
                        down: false
                    }
                )
            });
            if !already_released {
                let release = Action::MouseButton {
                    button: MouseButton::Left,
                    down: false,
                };
                if let Err(err) = self.injector.apply(release) {
                    eprintln!("[session] ปล่อย sticky drag ไม่สำเร็จ: {err}");
                }
                self.sent = self.sent.saturating_add(1);
                out.push(release);
            }
        }

        out
    }

    /// ปล่อยทุกอย่างที่ค้าง (จอยหลุด / ปิดแอป / emergency)
    pub fn release_all(&mut self) {
        if let Err(err) = self.injector.release_all() {
            eprintln!("[session] release_all ไม่สำเร็จ: {err}");
        }
    }

    /// จอยหลุด: ปล่อยคีย์ + พัก engine ไว้ที่ Disabled
    pub fn on_pad_lost(&mut self) {
        self.release_all();
        self.engine.set_mode(Mode::Disabled);
    }

    /// จอยกลับมา: ออกจาก Disabled ที่เกิดจาก "จอยหลุด"
    /// (`disabled_remaining_ms > 0` = ถูกปิดด้วย emergency → ยังต้องรอ deadman)
    pub fn on_pad_found(&mut self) {
        if self.engine.mode() == Mode::Disabled && self.engine.disabled_remaining_ms() == 0 {
            self.engine.set_mode(Mode::Type);
        }
    }

    /// ตั้งภาษาที่เปิดไว้ (config) — สร้าง slot ของ "ภาษาอื่น" ใหม่ตามภาษาปัจจุบัน
    #[allow(dead_code)] // ใช้เมื่อ Settings เปลี่ยนภาษาแบบ hot reload (P5 ถัดไป)
    pub fn set_enabled_languages(&mut self, enabled: Vec<Lang>) {
        self.enabled = enabled;
        self.refresh_languages();
    }

    /// ตั้ง override ของ keymap (จาก config) — ล้างของเดิมแล้วใส่ชุดใหม่
    pub fn set_keymap_overrides(
        &mut self,
        overrides: &[(Lang, u8, u8, u8, Option<Key>)],
    ) {
        self.engine.clear_keymap_overrides();
        for (lang, layer, group, sector, key) in overrides {
            self.engine
                .set_keymap_override(*lang, *layer, *group, *sector, *key);
        }
    }

    /* ---------------- internal ---------------- */

    fn refresh_languages(&mut self) {
        // core: `languages` = ภาษาอื่นที่เปิดใช้ (ไม่รวม EN) — ต้องคงภาษาปัจจุบันไว้
        // ไม่งั้น `set_languages` จะตีกลับเป็น EN (interaction model §7)
        let slots: Vec<LangSlot> = self
            .enabled
            .iter()
            .filter(|lang| **lang != Lang::En)
            .enumerate()
            .map(|(index, lang)| LangSlot::new(index as u8, *lang))
            .collect();
        self.engine.set_languages(slots);
    }

    fn apply(&mut self, action: Action, out: &mut Vec<Action>) {
        let mut changed_language = false;

        let to_inject = match action {
            // คีย์ลัด: engine ตัดสินแล้ว ที่เหลือคือส่ง Ctrl+key
            Action::Command(Command::Copy) => Some(shortcut('c')),
            Action::Command(Command::Paste) => Some(shortcut('v')),
            Action::Command(Command::Undo) => Some(shortcut('z')),
            Action::Command(Command::Redo) => Some(shortcut('y')),
            // engine เปลี่ยน mode/ภาษาของตัวเองแล้ว — ไม่มีอะไรต้องส่งถึง OS
            Action::Command(Command::Mouse)
            | Action::Command(Command::PlaceUi)
            | Action::Command(Command::Nav)
            | Action::Command(Command::TypeEn)
            | Action::Command(Command::TypeTh)
            | Action::Command(Command::Language)
            | Action::SetMode(_)
            | Action::SetLayer(_)
            | Action::Noop => None,
            Action::SetLang { .. } => {
                changed_language = true;
                None
            }
            Action::MovePivot { dx, dy } => {
                self.pivot = self.clamp_pivot((self.pivot.0 + dx, self.pivot.1 + dy));
                None
            }
            Action::ResetPivot => {
                self.pivot = self.clamp_pivot(self.pivot_home);
                None
            }
            Action::RecenterPointer => {
                self.pointer = self.pointer_home;
                Some(Action::MouseMoveAbs {
                    x: self.pointer_home.0 as i32,
                    y: self.pointer_home.1 as i32,
                })
            }
            Action::MouseMoveAbs { x, y } => {
                self.pointer = (x as f32, y as f32);
                Some(action)
            }
            Action::MouseMove { dx, dy } => {
                self.pointer = (self.pointer.0 + dx as f32, self.pointer.1 + dy as f32);
                Some(action)
            }
            other => Some(other),
        };

        if let Some(effect) = to_inject {
            if let Err(err) = self.injector.apply(effect) {
                eprintln!("[session] ส่ง {effect:?} ไม่สำเร็จ: {err}");
            }
            self.sent = self.sent.saturating_add(1);
            // WPM นับเฉพาะตัวอักษรที่พิมพ์ (Key::Char) — ปุ่ม named/ลูกศรไม่นับ
            if let Action::Key(press) = effect
                && matches!(press.key, Key::Char(_))
            {
                self.note_commit();
            }
            out.push(effect);
        }

        if changed_language {
            // วงภาษาต้องไม่ชี้กลับไปภาษาที่เพิ่งเลือก
            self.refresh_languages();
        }
    }

    /* ---------------- HudState (contract v1) ---------------- */

    /// สร้าง `HudState` — พิกัด pivot/pointer ถูกแปลงเป็นพิกัดหน้าต่าง overlay แล้ว
    pub fn hud_state(&self, style: HudStyle, origin: (i32, i32)) -> Value {
        let engine = &self.engine;
        let lang = engine.lang();
        let blocked = self.guard.is_some();
        let caret = engine.caret_active() && !blocked;
        let mode = if blocked {
            "disabled"
        } else {
            hud_mode(engine.mode(), engine.wheel(), lang, caret)
        };

        let group = engine.typing_group();
        let sector = engine.typing_sector();
        let typing = engine.mode() == Mode::Type && engine.wheel().is_none() && !caret && !blocked;
        let shift_lock = engine.shift_lock();
        let layer = engine.layer();
        let repeating = engine.gesture_state() == GestureState::Repeating;

        let (inner, outer) = if blocked {
            (
                json!({ "kind": "none", "active": -1, "labels": [] }),
                json!({ "kind": "none", "active": -1, "labels": [] }),
            )
        } else {
            self.rings(mode, lang, group, sector, typing, layer)
        };
        let preview = if typing {
            match (group, sector) {
                (Some(g), Some(s)) => engine
                    .preview_key(g, s)
                    .map(|key| shifted(label_of(key), shift_lock))
                    .unwrap_or_default(),
                _ => String::new(),
            }
        } else {
            String::new()
        };
        // ADR-13: ล็อกตัวใหญ่แล้ว pill ต้องบอก (HUD แสดงเป็นข้อความใน pill)
        let status_text = if let Some(reason) = self.guard {
            guard_text(reason)
        } else if caret {
            if engine.caret_selecting() {
                "เลือกข้อความ"
            } else {
                "ลูกศร"
            }
        } else if shift_lock && typing {
            "ตัวใหญ่"
        } else {
            ""
        };
        let repeat_count = if repeating { engine.repeat_count() } else { 0 };

        let mut badges: Vec<String> = Vec::new();
        if engine.sticky_drag() {
            badges.push("Drag".to_string());
        }

        json!({
            "version": 1,
            "visible": true,
            "mode": mode,
            "ringAlways": false,
            "hudHidden": false,
            "scale": style.scale,
            "opacity": style.opacity,
            "pivot": {
                "x": f64::from(self.pivot.0) - f64::from(origin.0),
                "y": f64::from(self.pivot.1) - f64::from(origin.1),
            },
            "layer": layer,
            "shiftLock": engine.shift_lock(),
            "state": hud_state_name(engine.gesture_state()),
            "group": group.map_or(-1, |dir| dir.index() as i32),
            "sector": sector.map_or(-1, |dir| dir.index() as i32),
            "language": {
                "code": lang_code(lang),
                "kind": "layout",
                "enabled": self.enabled.iter().map(|lang| lang_code(*lang)).collect::<Vec<_>>(),
            },
            "inner": inner,
            "outer": outer,
            "preview": { "char": preview, "count": repeat_count },
            "pointer": {
                "x": f64::from(self.pointer.0) - f64::from(origin.0),
                "y": f64::from(self.pointer.1) - f64::from(origin.1),
                "visible": engine.mode() == Mode::Mouse && !blocked,
            },
            "status": { "text": status_text, "badges": badges },
        })
    }

    /// วงใน/วงนอกตาม mode — `inner.kind` = group | command | language | none
    fn rings(
        &self,
        mode: &str,
        lang: Lang,
        group: Option<Dir8>,
        sector: Option<Dir8>,
        typing: bool,
        layer: u8,
    ) -> (Value, Value) {
        let engine = &self.engine;
        let direction = engine
            .wheel_highlight_dir()
            .map_or(-1, |dir| dir.index() as i32);

        let inner = match engine.wheel() {
            Some(WheelKind::Command) => {
                let labels: Vec<Value> = Dir8::ALL
                    .iter()
                    .map(|dir| command_label(engine, *dir))
                    .collect();
                json!({ "kind": "command", "active": direction, "labels": labels })
            }
            Some(WheelKind::Language) => {
                // index ของภาษา (engine.wheel_index) ตรงกับ labels[], ไม่ใช่ทิศ
                let labels: Vec<Value> = engine
                    .languages()
                    .iter()
                    .map(|slot| {
                        json!({
                            "text": lang_code(slot.lang).to_uppercase(),
                            "sub": lang_label(slot.lang),
                        })
                    })
                    .collect();
                let active = engine.wheel_index().map_or(-1, i32::from);
                json!({ "kind": "language", "active": active, "labels": labels })
            }
            Some(WheelKind::QuickBar) => {
                let labels: Vec<Value> = Dir8::ALL
                    .iter()
                    .map(|dir| quickbar_label(*dir))
                    .collect();
                json!({ "kind": "command", "active": direction, "labels": labels })
            }
            None if typing => {
                let labels: Vec<Value> = group_names_for(lang, layer)
                    .iter()
                    .map(|name| json!({ "text": name }))
                    .collect();
                let active = group.map_or(-1, |dir| dir.index() as i32);
                json!({ "kind": "group", "active": active, "labels": labels })
            }
            None => json!({ "kind": "none", "active": -1, "labels": [] }),
        };

        let outer = match (mode, typing, group) {
            (m, true, Some(group)) if m.starts_with("type-") => {
                let shift_lock = engine.shift_lock();
                let labels: Vec<Value> = Dir8::ALL
                    .iter()
                    .map(|dir| {
                        let text = engine
                            .preview_key(group, *dir)
                            .map(|key| shifted(label_of(key), shift_lock))
                            .unwrap_or_default();
                        json!({ "text": text })
                    })
                    .collect();
                let active = sector.map_or(-1, |dir| dir.index() as i32);
                json!({ "kind": "char", "active": active, "labels": labels })
            }
            _ => json!({ "kind": "none", "active": -1, "labels": [] }),
        };

        (inner, outer)
    }
}

/// ข้อความใน pill เมื่อถูก guard — HUD `disabled` ต้องบอกเหตุผล (spec)
fn guard_text(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::Exclusion => "แอปนี้อยู่ในรายการกัน",
        BlockReason::AdminRequired => "ต้องรันแบบ admin",
    }
}

fn shortcut(ch: char) -> Action {
    Action::Key(KeyPress {
        key: Key::Char(ch),
        shift: false,
        ctrl: true,
        alt: false,
    })
}

/// ADR-13: `shiftLock` เปิด → ป้าย a–z บนวงแสดงเป็น A–Z (ตัวอักษรอื่น/ไทยคงเดิม)
/// กฎเดียวกับ `RuntimeEngine::with_shift_lock` ตอนส่งคีย์จริง
fn shifted(text: String, shift_lock: bool) -> String {
    if !shift_lock {
        return text;
    }
    text.chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() {
                ch.to_ascii_uppercase()
            } else {
                ch
            }
        })
        .collect()
}

fn hud_mode(mode: Mode, wheel: Option<WheelKind>, lang: Lang, caret: bool) -> &'static str {
    match wheel {
        Some(WheelKind::Command) => "cmd",
        Some(WheelKind::Language) => "lang",
        Some(WheelKind::QuickBar) => "quickbar",
        None if caret => "caret",
        None => match (mode, lang) {
            (Mode::Type, Lang::Th) => "type-th",
            (Mode::Type, Lang::En) => "type-en",
            (Mode::Mouse, _) => "mouse",
            (Mode::Nav, _) => "nav",
            (Mode::Disabled, _) => "disabled",
        },
    }
}

fn hud_state_name(state: GestureState) -> &'static str {
    match state {
        GestureState::Idle => "idle",
        GestureState::GroupArmed => "group",
        GestureState::CharSelected => "char",
        GestureState::Committing => "commit",
        GestureState::Repeating => "repeat",
    }
}

fn lang_code(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Th => "th",
    }
}

fn lang_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::Th => "ไทย",
    }
}

/// ป้ายของช่องในวงคำสั่ง (ตามที่ engine มีจริง: ↑ EN · ↗ ภาษา · → Mouse · ↘ UI · ↓/↙/←/↖)
fn command_label(engine: &RuntimeEngine, dir: Dir8) -> Value {
    match engine.command_slot(dir) {
        Some(Command::TypeEn) => json!({ "text": "EN", "sub": "English" }),
        Some(Command::TypeTh) => json!({ "text": "TH", "sub": "ไทย" }),
        Some(Command::Language) => json!({ "text": "Lang", "sub": "Language" }),
        Some(Command::Mouse) => json!({ "text": "Mouse", "sub": "Move pointer" }),
        Some(Command::PlaceUi) => json!({ "text": "UI", "sub": "Place UI" }),
        Some(Command::Copy) => json!({ "text": "Copy", "sub": "Ctrl+C" }),
        Some(Command::Paste) => json!({ "text": "Paste", "sub": "Ctrl+V" }),
        Some(Command::Undo) => json!({ "text": "Undo", "sub": "Ctrl+Z" }),
        Some(Command::Redo) => json!({ "text": "Redo", "sub": "Ctrl+Y" }),
        Some(Command::Nav) => json!({ "text": "Nav", "sub": "Place UI" }),
        None => json!({ "text": "" }),
    }
}

/// ป้ายของช่อง quick bar (LB ค้าง) — ทิศตรงกับ interaction model §5
fn quickbar_label(dir: Dir8) -> Value {
    match RuntimeEngine::quickbar_slot(dir) {
        QuickAction::Copy => json!({ "text": "Copy", "sub": "Ctrl+C" }),
        QuickAction::Paste => json!({ "text": "Paste", "sub": "Ctrl+V" }),
        QuickAction::Cut => json!({ "text": "Cut", "sub": "Ctrl+X" }),
        QuickAction::Undo => json!({ "text": "Undo", "sub": "Ctrl+Z" }),
        QuickAction::Redo => json!({ "text": "Redo", "sub": "Ctrl+Y" }),
        QuickAction::SelectAll => json!({ "text": "All", "sub": "Ctrl+A" }),
        QuickAction::Save => json!({ "text": "Save", "sub": "Ctrl+S" }),
        QuickAction::SwitchWindow => json!({ "text": "Switch", "sub": "Alt+Tab" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnitype_core::gesture::GestureConfig;
    use rnitype_core::runtime::RuntimeConfig;
    use rnitype_core::{Buttons, Vec2};
    use rnitype_win::NullInjector;
    use std::sync::{Arc, Mutex};

    /// injector ที่เก็บ action ไว้ให้ test อ่าน (ข้าม injector จริง)
    #[derive(Clone, Default)]
    struct SharedInjector {
        sent: Arc<Mutex<Vec<Action>>>,
        releases: Arc<Mutex<u32>>,
    }

    impl Injector for SharedInjector {
        fn apply(&mut self, action: Action) -> Result<(), rnitype_win::InjectError> {
            self.sent.lock().unwrap().push(action);
            Ok(())
        }

        fn release_all(&mut self) -> Result<(), rnitype_win::InjectError> {
            *self.releases.lock().unwrap() += 1;
            Ok(())
        }
    }

    fn engine() -> RuntimeEngine {
        RuntimeEngine::new(RuntimeConfig::default())
    }

    fn session_with_lang(lang: Lang) -> (Session, SharedInjector) {
        let shared = SharedInjector::default();
        let mut engine = RuntimeEngine::new(RuntimeConfig::default());
        engine.set_lang(lang);
        let session = Session::new(
            engine,
            Box::new(shared.clone()),
            vec![Lang::En, Lang::Th],
            (960.0, 540.0),
        );
        (session, shared)
    }

    fn session() -> (Session, SharedInjector) {
        session_with_lang(Lang::En)
    }

    fn pad(ls: (f32, f32), rs: (f32, f32)) -> PadState {
        PadState {
            ls: Vec2::new(ls.0, ls.1),
            rs: Vec2::new(rs.0, rs.1),
            ..PadState::default()
        }
    }

    /// กดกลุ่ม → เลือกช่อง → ปล่อย RS → dwell = พิมพ์ 1 ตัว
    fn type_char(session: &mut Session, ls: (f32, f32), rs: (f32, f32)) -> Vec<Action> {
        session.step(&pad(ls, rs), 4);
        session.step(&pad(ls, rs), 4);
        session.step(&pad(ls, (0.0, 0.0)), 4);
        session.step(&pad(ls, (0.0, 0.0)), 64)
    }

    #[test]
    fn keymap_overrides_reach_the_injector() {
        let (mut session, shared) = session();
        session.set_keymap_overrides(&[(Lang::En, 1, 0, 0, Some(Key::Char('z')))]);
        type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        assert_eq!(
            *shared.sent.lock().unwrap(),
            vec![Action::Key(KeyPress {
                key: Key::Char('z'),
                shift: false,
                ctrl: false,
                alt: false,
            })]
        );
    }

    #[test]
    fn typing_a_letter_counts_toward_wpm() {
        let (mut session, _) = session();
        assert_eq!(session.chars(), 0);
        assert_eq!(session.wpm(), 0.0);

        type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        assert_eq!(session.chars(), 1);
        assert_eq!(session.stats()["chars"], 1);
        // 1 ตัวอักษรในหน้าต่าง 60 วิ = 0.2 WPM (5 ตัวอักษร/คำ)
        assert!((session.wpm() - 0.2).abs() < 1e-9);
        assert_eq!(session.stats()["wpm"], 0.2);

        // ปุ่มอื่น (Space) ไม่นับเป็นตัวอักษรของ WPM
        let mut p = pad((0.0, 0.0), (0.0, 0.0));
        p.buttons.a = true;
        session.step(&p, 4);
        assert_eq!(session.chars(), 1);
        assert_eq!(session.stats()["actions"], 2);
    }

    #[test]
    fn typing_a_letter_reaches_the_injector() {
        let (mut session, shared) = session();
        let actions = type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        assert_eq!(actions.len(), 1, "ต้องส่ง 1 คีย์: {actions:?}");
        assert_eq!(
            *shared.sent.lock().unwrap(),
            vec![Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false,
                alt: false,
            })]
        );
    }

    #[test]
    fn face_button_reaches_the_injector() {
        let (mut session, _) = session();
        let mut p = pad((0.0, 0.0), (0.0, 0.0));
        p.buttons.a = true;
        let actions = session.step(&p, 4);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Named(rnitype_core::NamedKey::Space),
                shift: false,
                ctrl: false,
                alt: false,
            })]
        );
    }

    #[test]
    fn copy_command_becomes_ctrl_c() {
        let (mut session, _) = session();
        // เปิดวงคำสั่ง (D-pad ←) แล้วดัน RS ลง (Copy) + ปล่อย
        let mut open = pad((0.0, 0.0), (0.0, 0.0));
        open.buttons.dpad_left = true;
        session.step(&open, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&pad((0.0, 0.0), (0.0, -0.9)), 4);
        let actions = session.step(&pad((0.0, 0.0), (0.0, 0.0)), 80);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Char('c'),
                shift: false,
                ctrl: true,
                alt: false,
            })]
        );
    }

    #[test]
    fn hud_state_describes_the_armed_group_and_its_characters() {
        let (mut session, _) = session_with_lang(Lang::Th);
        session.step(&pad((0.0, 0.9), (0.0, 0.0)), 4); // LS ↑ = กลุ่มพยัญชนะ
        session.step(&pad((0.0, 0.9), (0.9, 0.0)), 4); // RS → = ช่องที่ 3
        let hud = session.hud_state(HudStyle::default(), (0, 0));

        assert_eq!(hud["version"], 1);
        assert_eq!(hud["mode"], "type-th");
        assert_eq!(hud["state"], "char");
        assert_eq!(hud["group"], 0);
        assert_eq!(hud["sector"], 2);
        assert_eq!(hud["inner"]["kind"], "group");
        assert_eq!(hud["inner"]["labels"][0]["text"], "พยัญชนะ");
        assert_eq!(hud["outer"]["kind"], "char");
        assert_eq!(hud["outer"]["labels"][0]["text"], "ก");
        assert_eq!(hud["outer"]["active"], 2);
        assert_eq!(hud["preview"]["char"], "ร");
        assert_eq!(hud["language"]["code"], "th");
        assert_eq!(hud["language"]["enabled"][0], "en");
    }

    #[test]
    fn hud_state_uses_window_coordinates_for_the_pivot() {
        let (session, _) = session();
        let hud = session.hud_state(HudStyle::default(), (-1920, 0));
        assert_eq!(hud["pivot"]["x"], 2880.0);
        assert_eq!(hud["pivot"]["y"], 540.0);
    }

    #[test]
    fn command_wheel_is_rendered_with_all_eight_slots() {
        let (mut session, _) = session();
        let mut open = pad((0.0, 0.0), (0.0, 0.0));
        open.buttons.dpad_left = true;
        session.step(&open, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "cmd");
        assert_eq!(hud["inner"]["kind"], "command");
        let labels = hud["inner"]["labels"].as_array().unwrap();
        assert_eq!(labels.len(), 8);
        assert_eq!(labels[0]["text"], "EN");
        assert_eq!(labels[1]["text"], "Lang");
        assert_eq!(labels[4]["sub"], "Ctrl+C");
    }

    #[test]
    fn switching_language_updates_the_wheel_slots() {
        let (mut session, _) = session();
        let mut open = pad((0.0, 0.0), (0.0, 0.0));
        open.buttons.dpad_left = true;
        session.step(&open, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&pad((0.0, 0.0), (0.7, 0.7)), 4); // ↗ = Language
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 80); // ปล่อย = เปลี่ยนภาษา (มี TH ภาษาเดียว)

        assert_eq!(session.engine().lang(), Lang::Th);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "type-th");
        assert_eq!(hud["language"]["code"], "th");
    }

    #[test]
    fn pad_lost_releases_everything_and_pauses() {
        let (mut session, _) = session();
        session.on_pad_lost();
        assert_eq!(session.engine().mode(), Mode::Disabled);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "disabled");
    }

    #[test]
    fn hud_state_validates_against_the_contract_sample_shape() {
        // กัน field หาย: เทียบ key ที่ contract บังคับ (`required`)
        let (session, _) = session();
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        for key in [
            "version",
            "visible",
            "mode",
            "ringAlways",
            "scale",
            "opacity",
            "pivot",
            "layer",
            "shiftLock",
            "state",
            "group",
            "sector",
            "language",
            "inner",
            "outer",
            "preview",
        ] {
            assert!(hud.get(key).is_some(), "ขาด field {key}");
        }
    }

    #[test]
    fn null_injector_session_still_counts_sent_actions() {
        let mut session = Session::new(
            engine(),
            Box::new(NullInjector::default()),
            vec![Lang::En, Lang::Th],
            (0.0, 0.0),
        );
        type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        assert_eq!(session.sent(), 1);
    }

    #[test]
    fn shift_lock_uppercases_the_ring_labels_and_the_preview() {
        let (mut session, _) = session();
        // LT แตะ = toggle ตัวพิมพ์ใหญ่ (ADR-13)
        let mut p = pad((0.0, 0.9), (0.0, 0.0));
        p.buttons.lt = true;
        session.step(&p, 4);
        session.step(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        // LS ↑ + RS ↑ = ช่องแรกของกลุ่ม AEIOU
        session.step(&pad((0.0, 0.9), (0.0, 0.9)), 4);

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["shiftLock"], true);
        assert_eq!(hud["outer"]["labels"][0]["text"], "A");
        assert_eq!(hud["preview"]["char"], "A");
        assert_eq!(hud["status"]["text"], "ตัวใหญ่");
    }

    #[test]
    fn gesture_config_is_respected() {
        // sanity: config ที่ใช้ใน test ต้องเป็นค่า default ตาม spec
        let cfg = GestureConfig::default();
        assert_eq!(cfg.commit_dwell_ms, 60);
        assert!(Buttons::default().names().next().is_none());
    }

    /* ---------------- P2 slice 3 ---------------- */

    fn switch_mode(session: &mut Session, rs: (f32, f32)) {
        let mut open = pad((0.0, 0.0), (0.0, 0.0));
        open.buttons.dpad_left = true;
        session.step(&open, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&pad((0.0, 0.0), rs), 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 80);
    }

    #[test]
    fn hud_state_shows_layer_two_labels_and_preview() {
        let (mut session, _) = session();
        // แตะ RB = ล็อกชั้น 2
        let mut rb = pad((0.0, 0.0), (0.0, 0.0));
        rb.buttons.rb = true;
        session.step(&rb, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);

        session.step(&pad((0.0, 0.9), (0.0, 0.0)), 4); // LS ↑
        session.step(&pad((0.0, 0.9), (0.0, 0.9)), 4); // RS ↑
        let hud = session.hud_state(HudStyle::default(), (0, 0));

        assert_eq!(hud["layer"], 2);
        assert_eq!(hud["mode"], "type-en");
        assert_eq!(hud["inner"]["labels"][0]["text"], "A–H");
        assert_eq!(hud["outer"]["labels"][0]["text"], "A");
        assert_eq!(hud["preview"]["char"], "A");
    }

    #[test]
    fn repeating_state_and_count_reach_the_hud() {
        let (mut session, _) = session();
        type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        // ดัน RS กลับทิศเดิมแล้วค้าง 350 ms = repeat
        session.step(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        session.step(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        let actions = session.step(&pad((0.0, 0.9), (0.0, 0.9)), 350);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false,
                alt: false,
            })]
        );

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["state"], "repeat");
        assert_eq!(hud["preview"]["count"], 1);
        assert_eq!(hud["preview"]["char"], "a");
    }

    #[test]
    fn mouse_recenter_moves_the_pointer_and_injects_an_absolute_move() {
        let shared = SharedInjector::default();
        let mut session = Session::with_homes(
            engine(),
            Box::new(shared.clone()),
            vec![Lang::En, Lang::Th],
            (960.0, 540.0),
            (1000.0, 500.0),
        );
        switch_mode(&mut session, (0.9, 0.0));

        let mut click = pad((0.0, 0.0), (0.0, 0.0));
        click.buttons.rs_click = true;
        let actions = session.step(&click, 4);
        assert_eq!(actions, vec![Action::MouseMoveAbs { x: 1000, y: 500 }]);

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "mouse");
        assert_eq!(hud["pointer"]["x"], 1000.0);
        assert_eq!(hud["pointer"]["visible"], true);
        assert!(
            shared
                .sent
                .lock()
                .unwrap()
                .contains(&Action::MouseMoveAbs { x: 1000, y: 500 })
        );
    }

    #[test]
    fn nav_mode_moves_and_resets_the_pivot_in_the_hud() {
        let (mut session, _) = session();
        switch_mode(&mut session, (0.9, -0.9));
        session.step(&pad((0.0, 0.9), (0.0, 0.0)), 100);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "nav");
        assert!(hud["pivot"]["y"].as_f64().unwrap() < 540.0);

        let mut click = pad((0.0, 0.0), (0.0, 0.0));
        click.buttons.rs_click = true;
        session.step(&click, 4);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["pivot"]["y"], 540.0);
    }

    /* ---------------- P2 slice 4 ---------------- */

    fn with_lb(mut pad: PadState) -> PadState {
        pad.buttons.lb = true;
        pad
    }

    #[test]
    fn quickbar_reaches_the_hud_and_runs_ctrl_c() {
        let (mut session, _) = session();
        session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 4);
        session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 150);

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "quickbar");
        assert_eq!(hud["inner"]["kind"], "command");
        let labels = hud["inner"]["labels"].as_array().unwrap();
        assert_eq!(labels.len(), 8);
        assert_eq!(labels[0]["text"], "Copy");
        assert_eq!(labels[0]["sub"], "Ctrl+C");
        assert_eq!(labels[7]["text"], "Switch");
        assert_eq!(labels[7]["sub"], "Alt+Tab");
        assert_eq!(hud["inner"]["active"], -1);

        // RS ↑ = Copy + ปล่อย RS (LB ยังค้าง) = Ctrl+C จริง
        session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.9))), 4);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["inner"]["active"], 0);

        let actions = session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 80);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Char('c'),
                shift: false,
                ctrl: true,
                alt: false
            })]
        );
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "type-en");
        // กลับมาที่วงกลุ่มตามปกติ (ยังไม่มีกลุ่มถูกเลือก)
        assert_eq!(hud["inner"]["kind"], "group");
        assert_eq!(hud["inner"]["active"], -1);
        assert_eq!(hud["preview"]["char"], "");
    }

    #[test]
    fn quickbar_alt_tab_is_injected_with_alt() {
        let (mut session, _) = session();
        session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 4);
        session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 150);
        session.step(&with_lb(pad((0.0, 0.0), (-0.7, 0.7))), 4);
        let actions = session.step(&with_lb(pad((0.0, 0.0), (0.0, 0.0))), 80);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Named(rnitype_core::NamedKey::Tab),
                shift: false,
                ctrl: false,
                alt: true
            })]
        );
    }

    #[test]
    fn caret_mode_shows_in_the_hud_without_rings() {
        let (mut session, _) = session();
        let mut click = pad((0.0, 0.0), (0.0, 0.0));
        click.buttons.rs_click = true;
        session.step(&click, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "caret");
        assert_eq!(hud["status"]["text"], "ลูกศร");
        assert_eq!(hud["inner"]["kind"], "none");
        assert_eq!(hud["outer"]["kind"], "none");
        assert_eq!(hud["preview"]["char"], "");

        // แตะ R3 อีกครั้งเร็ว ๆ = เลือกข้อความ
        let mut click = pad((0.0, 0.0), (0.0, 0.0));
        click.buttons.rs_click = true;
        session.step(&click, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["status"]["text"], "เลือกข้อความ");

        // RS ← = Shift+Left ออกไปจริง
        let actions = session.step(&pad((0.0, 0.0), (-0.9, 0.0)), 4);
        assert_eq!(
            actions,
            vec![Action::Key(KeyPress {
                key: Key::Named(rnitype_core::NamedKey::Left),
                shift: true,
                ctrl: false,
                alt: false
            })]
        );
    }

    #[test]
    fn sticky_drag_shows_a_badge_and_releases_on_mode_switch() {
        let (mut session, _) = session();
        switch_mode(&mut session, (0.9, 0.0));
        assert_eq!(session.engine().mode(), Mode::Mouse);

        let mut rb = pad((0.0, 0.0), (0.0, 0.0));
        rb.buttons.rb = true;
        session.step(&rb, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&rb, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        assert!(session.engine().sticky_drag());

        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert!(hud["status"]["badges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|badge| badge == "Drag"));

        // สลับกลับ Type ผ่านวงคำสั่ง: session ต้องปล่อยปุ่มซ้ายให้ด้วย
        let mut open = pad((0.0, 0.0), (0.0, 0.0));
        open.buttons.dpad_left = true;
        session.step(&open, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.9)), 4);
        let actions = session.step(&pad((0.0, 0.0), (0.0, 0.0)), 80);

        assert_eq!(
            actions,
            vec![Action::MouseButton {
                button: MouseButton::Left,
                down: false
            }],
            "ต้องปล่อยปุ่มซ้ายเมื่อออกจาก Mouse"
        );
        assert!(!session.engine().sticky_drag());
        assert_eq!(session.engine().mode(), Mode::Type);
    }

    #[test]
    fn pivot_is_clamped_to_the_safe_area() {
        let mut session = Session::with_homes(
            engine(),
            Box::new(NullInjector::default()),
            vec![Lang::En, Lang::Th],
            (950.0, 520.0),
            (950.0, 520.0),
        );
        session.set_pivot_clamp((300.0, 200.0), (1700.0, 900.0));

        switch_mode(&mut session, (0.9, -0.9));
        // ดันขึ้นแรงและนาน → ต้องหยุดที่ขอบบน (y = 200) ไม่ลอยออกนอกจอ
        session.step(&pad((0.0, 1.0), (0.0, 0.0)), 5_000);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["pivot"]["y"], 200.0);
        assert_eq!(hud["pivot"]["x"], 950.0);

        // กด RS = กลับจุดเริ่มต้น (ยังอยู่ในขอบเขต)
        let mut click = pad((0.0, 0.0), (0.0, 0.0));
        click.buttons.rs_click = true;
        session.step(&click, 4);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["pivot"]["y"], 520.0);
    }

    #[test]
    fn pivot_clamp_moves_an_out_of_bounds_home_inside() {
        let mut session = Session::with_homes(
            engine(),
            Box::new(NullInjector::default()),
            vec![Lang::En, Lang::Th],
            (10.0, 10.0),
            (10.0, 10.0),
        );
        session.set_pivot_clamp((300.0, 200.0), (1700.0, 900.0));
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["pivot"]["x"], 300.0);
        assert_eq!(hud["pivot"]["y"], 200.0);
    }

    /* ---------------- P3 slice 2: guard / pause ---------------- */

    #[test]
    fn exclusion_guard_shows_disabled_with_reason_and_keeps_engine_mode() {
        let (mut session, _) = session();
        switch_mode(&mut session, (0.9, 0.0));
        assert_eq!(session.engine().mode(), Mode::Mouse);

        session.set_guard(Some(BlockReason::Exclusion));
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "disabled");
        assert_eq!(hud["status"]["text"], "แอปนี้อยู่ในรายการกัน");
        assert_eq!(hud["inner"]["kind"], "none");
        assert_eq!(hud["outer"]["kind"], "none");
        assert_eq!(hud["pointer"]["visible"], false);
        // engine ยังจำ mode เดิมไว้ (กลับมาแล้วทำงานต่อได้)
        assert_eq!(session.engine().mode(), Mode::Mouse);

        session.set_guard(None);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "mouse");
        assert_eq!(hud["status"]["text"], "");
    }

    #[test]
    fn admin_required_guard_uses_its_own_message() {
        let (mut session, _) = session();
        session.set_guard(Some(BlockReason::AdminRequired));
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "disabled");
        assert_eq!(hud["status"]["text"], "ต้องรันแบบ admin");
    }

    #[test]
    fn entering_guard_releases_held_mouse_buttons() {
        let (mut session, shared) = session();
        switch_mode(&mut session, (0.9, 0.0));
        // แตะ RB 2 ครั้ง = ล็อกปุ่มซ้ายค้าง
        let mut rb = pad((0.0, 0.0), (0.0, 0.0));
        rb.buttons.rb = true;
        session.step(&rb, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        session.step(&rb, 4);
        session.step(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        assert!(session.engine().sticky_drag());

        session.set_guard(Some(BlockReason::Exclusion));
        assert!(
            *shared.releases.lock().unwrap() >= 1,
            "เข้า guard ต้องเรียก release_all ปล่อยปุ่มที่ค้าง"
        );
    }

    #[test]
    fn pause_remembers_the_mode_and_unpause_restores_it() {
        let (mut session, _) = session();
        switch_mode(&mut session, (0.9, 0.0));
        assert_eq!(session.engine().mode(), Mode::Mouse);

        session.pause();
        assert_eq!(session.engine().mode(), Mode::Disabled);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "disabled");

        session.unpause();
        assert_eq!(session.engine().mode(), Mode::Mouse);
        let hud = session.hud_state(HudStyle::default(), (0, 0));
        assert_eq!(hud["mode"], "mouse");
    }

    #[test]
    fn pause_during_emergency_does_not_clear_the_deadman_timer() {
        let (mut session, _) = session();
        let mut both = pad((0.0, 0.0), (0.0, 0.0));
        both.buttons.lb = true;
        both.buttons.rb = true;
        for _ in 0..16 {
            session.step(&both, 100);
        }
        assert_eq!(session.engine().mode(), Mode::Disabled);
        let before = session.engine().disabled_remaining_ms();
        assert!(before > 0);

        session.pause();
        assert_eq!(session.engine().disabled_remaining_ms(), before);
    }
}
