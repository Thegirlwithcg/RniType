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

use rnitype_core::keymap::{group_names_for, label_of, lookup_layer};
use rnitype_core::normalizer::Dir8;
use rnitype_core::runtime::{LangSlot, RuntimeEngine, WheelKind};
use rnitype_core::{Action, Command, GestureState, Key, KeyPress, Lang, Mode, PadState};
use rnitype_win::Injector;
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
    /// จำนวนคีย์ที่ส่งออกแล้ว (log/สถิติ)
    sent: u32,
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
            sent: 0,
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

    /// ตั้งตำแหน่ง pointer ที่ HUD ใช้ (ตอนเริ่มแอป = ตำแหน่งจริงของ OS)
    pub fn set_pointer(&mut self, pointer: (f32, f32)) {
        self.pointer = pointer;
    }

    /// 1 เฟรมของ engine — คืน action ที่ "ส่งออกจริง" (สำหรับ log/ทดสอบ)
    pub fn step(&mut self, pad: &PadState, dt_ms: u32) -> Vec<Action> {
        let mut out = Vec::new();
        if let Some(action) = self.engine.tick(pad, dt_ms) {
            self.apply(action, &mut out);
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
                self.pivot = (self.pivot.0 + dx, self.pivot.1 + dy);
                None
            }
            Action::ResetPivot => {
                self.pivot = self.pivot_home;
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
        let mode = hud_mode(engine.mode(), engine.wheel(), lang);

        let group = engine.typing_group();
        let sector = engine.typing_sector();
        let typing = engine.mode() == Mode::Type && engine.wheel().is_none();
        let shift_lock = engine.shift_lock();
        let layer = engine.layer();
        let repeating = engine.gesture_state() == GestureState::Repeating;

        let (inner, outer) = self.rings(mode, lang, group, sector, typing, layer);
        let preview = if typing {
            match (group, sector) {
                (Some(g), Some(s)) => lookup_layer(lang, layer, g, s)
                    .map(|key| shifted(label_of(key), shift_lock))
                    .unwrap_or_default(),
                _ => String::new(),
            }
        } else {
            String::new()
        };
        // ADR-13: ล็อกตัวใหญ่แล้ว pill ต้องบอก (HUD แสดงเป็นข้อความใน pill)
        let status_text = if shift_lock && typing {
            "ตัวใหญ่"
        } else {
            ""
        };
        let repeat_count = if repeating { engine.repeat_count() } else { 0 };

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
                "visible": engine.mode() == Mode::Mouse,
            },
            "status": { "text": status_text, "badges": Vec::<String>::new() },
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
                        let text = lookup_layer(lang, layer, group, *dir)
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

fn shortcut(ch: char) -> Action {
    Action::Key(KeyPress {
        key: Key::Char(ch),
        shift: false,
        ctrl: true,
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

fn hud_mode(mode: Mode, wheel: Option<WheelKind>, lang: Lang) -> &'static str {
    match wheel {
        Some(WheelKind::Command) => "cmd",
        Some(WheelKind::Language) => "lang",
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
    struct SharedInjector(Arc<Mutex<Vec<Action>>>);

    impl Injector for SharedInjector {
        fn apply(&mut self, action: Action) -> Result<(), rnitype_win::InjectError> {
            self.0.lock().unwrap().push(action);
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
    fn typing_a_letter_reaches_the_injector() {
        let (mut session, shared) = session();
        let actions = type_char(&mut session, (0.0, 0.9), (0.0, 0.9));
        assert_eq!(actions.len(), 1, "ต้องส่ง 1 คีย์: {actions:?}");
        assert_eq!(
            *shared.0.lock().unwrap(),
            vec![Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false
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
                ctrl: false
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
                ctrl: true
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
                ctrl: false
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
                .0
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
}
