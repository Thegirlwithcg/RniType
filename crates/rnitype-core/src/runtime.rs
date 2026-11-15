//! RuntimeEngine — mode arbiter + วงคำสั่ง + วงภาษา ครอบท่อการพิมพ์
//!
//! อ้างอิง `Design/02-design/04-state-machine-spec.md` §3.1–3.2 / §5
//! และ ADR-10 (วงคำสั่งใช้ RS) · ADR-11 (Nav = Place UI) · ADR-14 (วงภาษา)
//!
//! ขอบเขตของ slice 1 (P2 Track A):
//! - วงคำสั่ง (D-pad ←): เปิด/ปิด · RS เลือก · ปล่อย RS = ยืนยัน · A ยืนยันทันที ·
//!   B / D-pad ← = ยกเลิก · arm หลัง RS กลับศูนย์ · `lock` เมื่อยืนยันด้วย A ขณะ LS ค้าง
//! - วงภาษา (level 2): มีภาษาอื่น 1 ภาษา = สลับทันที · มากกว่า 1 = วงภาษา
//! - mode arbiter Type / Mouse / Nav / Disabled · สลับ mode = ตัด gesture ที่ค้าง ·
//!   emergency LB+RB ค้าง 1.5 s → Disabled 3 s
//!
//! ขอบเขตของ slice 2 (P2 Track A — typing completion):
//! - Shift toggle (LT): rising edge ใน Type mode = สลับ [`RuntimeEngine::shift_lock`]
//!   (ADR-13: toggle ไม่ใช่ hold) · ล็อกแล้วตัวอักษร a–z ออกเป็น A–Z พร้อม `shift = false`
//! - ปุ่มหน้าแบบแตะ ทุก mode ที่ไม่ใช่วง: A = Space · B = Backspace · X = Enter · Y = Tab
//!   · ตัด gesture ที่ค้างก่อนส่งเสมอ · B ขณะมี gesture = ยกเลิกอย่างเดียว (spec §2)
//!
//! ยังไม่ทำใน slice นี้ (TODO ของ P2 รอบถัดไป):
//! - pointer math/accel ของ Mouse mode · การเลื่อน pivot ของ Nav mode
//! - auto-repeat (REPEATING) · layer 2 · quick bar (LB ค้าง) · caret nav
//! - ปุ่มหน้าแบบผสม: A แตะเร็ว 2 ครั้ง = Enter · B ค้าง 400 ms = ลบทั้งคำ · Y ค้าง = Esc
//! - LT/LB ตอน Mouse/Nav = precision hold (ADR-13 ยังไม่กำหนดค่า)
//! - ตรวจ exclusion list / จอยหลุด (เป็นงานฝั่ง app–win ไม่ใช่ core)

use crate::action::{Action, Command, Key, KeyPress, Mode, NamedKey};
use crate::gesture::{GestureConfig, GestureState};
use crate::keymap::Lang;
use crate::normalizer::{Dir8, SectorTracker};
use crate::pad::{PadState, Vec2};
use crate::typing::TypingPipeline;

/// ภาษาที่สองสูงสุดที่วงภาษารองรับ (spec §3.2 — สูงสุด 8)
pub const MAX_LANGUAGES: usize = 8;

/// ค่าคงที่เวลาของ runtime — ค่าเริ่มต้นตาม spec §4
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeConfig {
    /// deadzone / dwell / hysteresis ของ gesture การพิมพ์ (ใช้ร่วมกับวงคำสั่ง)
    pub gesture: GestureConfig,
    /// LB+RB ต้องค้างนานเท่านี้จึงเข้า `DISABLED` (spec §4: 1.5 s)
    pub emergency_hold_ms: u32,
    /// ระยะเวลาที่หยุดรับ input หลังกดปิดฉุกเฉิน (spec §4: `deadman` 3 s)
    pub deadman_ms: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            gesture: GestureConfig::default(),
            emergency_hold_ms: 1500,
            deadman_ms: 3000,
        }
    }
}

/// ภาษาในวงภาษา: index ใน `config.languages[]` (ค่าที่ส่งออกใน [`Action::SetLang`])
/// + ตาราง keymap ที่ core รู้จัก
///
/// TODO(P5a): `Lang` ยังเป็น enum ตายตัว (EN/TH) — P5a ต้องขยายการ lookup keymap
/// ให้รับ layout ที่ติดตั้งจริงใน Windows (HKL → 8×8) แทนการเพิ่ม variant ที่นี่
/// ตอนนั้น `LangSlot` อาจเปลี่ยนจาก `Lang` เป็น id ของ layout
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LangSlot {
    pub index: u8,
    pub lang: Lang,
}

impl LangSlot {
    pub const fn new(index: u8, lang: Lang) -> Self {
        Self { index, lang }
    }
}

/// วงที่เปิดอยู่ (HUD ใช้วาดวงใน/level 2)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelKind {
    /// วงคำสั่ง 8 ช่อง (D-pad ←)
    Command,
    /// วงภาษา level 2 (จากช่อง Language ของวงคำสั่ง)
    Language,
}

/// เครื่องยนต์ระดับบนสุด: ถือ mode + วง + ท่อการพิมพ์
///
/// ลำดับความสำคัญของ event ตาม spec §3:
/// 1) LB+RB 1.5 s → Disabled · 2) Disabled = รับแต่เวลาถอยหลัง ·
/// 3) D-pad ← เปิด/ปิดวงคำสั่ง (ตัด gesture พิมพ์) · 4) A/B ในวง ·
/// 5) modifier LT = Shift toggle (ADR-13) · 6) ปุ่มหน้า A/B/X/Y (ตัด gesture) ·
/// 7) gesture บนแกน (LS/RS) เฉพาะโหมดพิมพ์ที่ไม่มีวงเปิด
#[derive(Clone, Debug)]
pub struct RuntimeEngine {
    cfg: RuntimeConfig,
    mode: Mode,
    typing: TypingPipeline,
    languages: Vec<LangSlot>,
    wheel: Option<WheelKind>,
    wheel_armed: bool,
    /// index ที่ไฮไลต์อยู่ (วงคำสั่ง = index ของ [`Dir8`], วงภาษา = index ภาษา)
    wheel_index: Option<u8>,
    /// ทิศที่ไฮไลต์ (ใช้กับ HUD ของทั้งสองวง)
    wheel_dir: Option<Dir8>,
    wheel_commit_ms: u32,
    cmd_sector: SectorTracker,
    /// mode ที่จะกลับไปหลัง `deadman` หมด (เฉพาะกรณี emergency)
    resume_mode: Mode,
    disabled_remaining_ms: u32,
    emergency_hold_ms: u32,
    prev_dpad_left: bool,
    prev_a: bool,
    prev_b: bool,
    prev_x: bool,
    prev_y: bool,
    prev_lt: bool,
    /// ADR-13: LT = สลับตัวพิมพ์ใหญ่ (toggle) — คงอยู่จนกดอีกครั้ง ไม่รีเซ็ตหลังพิมพ์
    shift_lock: bool,
}

/// rising edge ของปุ่มที่ต้องนับครั้งเดียวต่อการกด (กัน auto-repeat ของจอย)
#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    dpad_left: bool,
    a: bool,
    b: bool,
    x: bool,
    y: bool,
    lt: bool,
}

/// ปุ่มหน้าที่ถูกแตะในเฟรมนี้ — สูงสุด 1 ปุ่มต่อเฟรม (spec ไม่ได้กำหนดกรณีชนกัน)
///
/// เรียง B ก่อนเพื่อให้ "ยกเลิก" ชนะปุ่มอื่น · X/Y ไม่ถูกใช้ในวงคำสั่ง
fn face_key(edges: Edges) -> Option<NamedKey> {
    if edges.b {
        Some(NamedKey::Backspace)
    } else if edges.a {
        Some(NamedKey::Space)
    } else if edges.x {
        Some(NamedKey::Enter)
    } else if edges.y {
        Some(NamedKey::Tab)
    } else {
        None
    }
}

impl RuntimeEngine {
    pub fn new(cfg: RuntimeConfig) -> Self {
        let gesture = cfg.gesture;
        Self {
            cfg,
            // แอปเริ่มที่โหมดพิมพ์ EN — `Mode::default()` คือ `Disabled` จึงไม่ใช้
            mode: Mode::Type,
            typing: TypingPipeline::new(Lang::En, gesture),
            languages: Vec::new(),
            wheel: None,
            wheel_armed: false,
            wheel_index: None,
            wheel_dir: None,
            wheel_commit_ms: 0,
            cmd_sector: SectorTracker::new(gesture.sector_hysteresis_deg),
            resume_mode: Mode::Type,
            disabled_remaining_ms: 0,
            emergency_hold_ms: 0,
            prev_dpad_left: false,
            prev_a: false,
            prev_b: false,
            prev_x: false,
            prev_y: false,
            prev_lt: false,
            shift_lock: false,
        }
    }

    pub fn config(&self) -> RuntimeConfig {
        self.cfg
    }

    /// mode ที่มีผลจริง (`Disabled` = ถูกปิดฉุกเฉิน หรือ app สั่งปิด)
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// ADR-13: `true` = ล็อกตัวพิมพ์ใหญ่ (LT แตะใน Type mode)
    /// คงอยู่จนกด LT อีกครั้ง — HUD ใช้แสดง pill `ตัวใหญ่` และตัวอักษรบนวงเป็น uppercase
    pub fn shift_lock(&self) -> bool {
        self.shift_lock
    }

    /// เปลี่ยน mode จากภายนอก (profile / exclusion list / ตั้งค่า)
    /// — ตัด gesture การพิมพ์ที่ค้างและปิดวงทั้งหมด (spec §3 ข้อ 1: event ที่สำคัญกว่าตัด gesture)
    ///
    /// `Mode::Disabled` ที่สั่งจากภายนอกจะค้างไว้จนกว่าจะสั่ง mode ใหม่
    /// (ต่างจาก emergency ที่กลับมาเองหลัง `deadman_ms`)
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.disabled_remaining_ms = 0;
        self.resume_mode = mode;
        self.typing.cancel();
        self.close_wheel();
    }

    pub fn lang(&self) -> Lang {
        self.typing.lang()
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.typing.set_lang(lang);
    }

    /// ภาษาอื่นที่เปิดใช้ (ไม่รวม EN) — เรียงตามลำดับใน config
    pub fn languages(&self) -> &[LangSlot] {
        &self.languages
    }

    /// ตั้งรายการภาษาที่สอง สูงสุด [`MAX_LANGUAGES`]
    ///
    /// ถ้าภาษาที่ใช้อยู่ถูกปิด → กลับ EN (interaction model §7)
    pub fn set_languages(&mut self, languages: Vec<LangSlot>) {
        self.languages = languages.into_iter().take(MAX_LANGUAGES).collect();
        let current = self.typing.lang();
        if current != Lang::En && !self.languages.iter().any(|slot| slot.lang == current) {
            self.typing.set_lang(Lang::En);
        }
        if self.languages.is_empty() && self.wheel == Some(WheelKind::Language) {
            self.close_wheel();
        }
    }

    /// วงที่เปิดอยู่ (`None` = ไม่มีวง)
    pub fn wheel(&self) -> Option<WheelKind> {
        self.wheel
    }

    /// `false` = เปิดวงขณะ RS ยังดันค้าง → ยังไม่รับการเลือกจนกว่า RS จะกลับศูนย์
    pub fn wheel_armed(&self) -> bool {
        self.wheel_armed
    }

    /// index ที่ไฮไลต์อยู่ (วงคำสั่ง = index ของ [`Dir8`], วงภาษา = index ภาษา)
    pub fn wheel_index(&self) -> Option<u8> {
        self.wheel_index
    }

    /// ช่องที่ไฮไลต์เป็นทิศ (ทั้งวงคำสั่งและวงภาษา)
    pub fn wheel_highlight_dir(&self) -> Option<Dir8> {
        self.wheel_dir
    }

    pub fn gesture_state(&self) -> GestureState {
        self.typing.state()
    }

    /// กลุ่มที่ LS เลือกอยู่ (สำหรับ HUD) — `None` = ยังไม่มี gesture
    pub fn typing_group(&self) -> Option<Dir8> {
        self.typing.group()
    }

    /// ช่องที่ RS เลือกอยู่ (สำหรับ HUD preview/วงนอก)
    pub fn typing_sector(&self) -> Option<Dir8> {
        self.typing.sector()
    }

    /// เวลาที่เหลือของการถูกปิด (`0` = ไม่ได้ถูกปิดด้วย emergency)
    pub fn disabled_remaining_ms(&self) -> u32 {
        self.disabled_remaining_ms
    }

    /// ช่องของวงคำสั่งในทิศนั้น — `None` = ไม่มีช่อง (เช่น Language ตอนมีแต่ EN)
    pub fn command_slot(&self, dir: Dir8) -> Option<Command> {
        match dir {
            // ↑ EN · ↗ ภาษาอื่น · → Mouse · ↘ Place UI · ↓ Copy · ↙ Paste · ← Undo · ↖ Redo
            Dir8::N => Some(Command::TypeEn),
            Dir8::NE => (!self.languages.is_empty()).then_some(Command::Language),
            Dir8::E => Some(Command::Mouse),
            Dir8::SE => Some(Command::PlaceUi),
            Dir8::S => Some(Command::Copy),
            Dir8::SW => Some(Command::Paste),
            Dir8::W => Some(Command::Undo),
            Dir8::NW => Some(Command::Redo),
        }
    }

    /// เรียกทุก poll (250 Hz) พร้อม `dt_ms` ที่ผ่านจริง — ไม่ใช้ wall clock
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32) -> Option<Action> {
        let edges = Edges {
            dpad_left: pad.buttons.dpad_left && !self.prev_dpad_left,
            a: pad.buttons.a && !self.prev_a,
            b: pad.buttons.b && !self.prev_b,
            x: pad.buttons.x && !self.prev_x,
            y: pad.buttons.y && !self.prev_y,
            lt: pad.buttons.lt && !self.prev_lt,
        };

        let action = self.step(pad, dt_ms, edges);

        self.prev_dpad_left = pad.buttons.dpad_left;
        self.prev_a = pad.buttons.a;
        self.prev_b = pad.buttons.b;
        self.prev_x = pad.buttons.x;
        self.prev_y = pad.buttons.y;
        self.prev_lt = pad.buttons.lt;
        action
    }

    fn step(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        // 1) ปุ่มปิดฉุกเฉิน (LB+RB ค้าง 1.5 s) — สำคัญที่สุด
        if self.mode != Mode::Disabled {
            self.emergency_hold_ms = if pad.buttons.lb && pad.buttons.rb {
                self.emergency_hold_ms.saturating_add(dt_ms)
            } else {
                0
            };
            if self.emergency_hold_ms >= self.cfg.emergency_hold_ms {
                return Some(self.enter_disabled());
            }
        }

        // 2) Disabled = ไม่รับ input ใด ๆ จนครบ deadman
        if self.mode == Mode::Disabled {
            return self.tick_disabled(dt_ms);
        }

        // 3) D-pad ← เปิด/ปิดวงคำสั่ง (ตัด gesture การพิมพ์ที่ค้าง)
        if edges.dpad_left {
            self.toggle_cmd_wheel(pad);
            return None;
        }

        // 4) วงเปิดอยู่ = A/B + RS เท่านั้น (LS ไม่มีผล · X/Y ไม่มีผล — วงชนะ)
        if self.wheel.is_some() {
            return self.tick_wheel(pad, dt_ms, edges.a, edges.b);
        }

        // 5) Modifier: LT แตะ = สลับ Shift lock (ADR-13)
        // เฉพาะ Type — Mouse/Nav ยังเป็น hold = precision (ยังไม่ทำใน slice นี้)
        if edges.lt && self.mode == Mode::Type {
            self.shift_lock = !self.shift_lock;
        }

        // 6) ปุ่มหน้า A/B/X/Y (spec §3 ข้อ 5) — ตัด gesture ก่อน แล้วจึงส่งคีย์
        // slice นี้ทำเฉพาะ "แตะ" — auto-repeat / double-tap / hold ยังไม่ทำ (ดู press_face_button)
        if let Some(key) = face_key(edges) {
            return self.press_face_button(pad, key);
        }

        // 7) gesture บนแกนตาม mode
        match self.mode {
            Mode::Type => self
                .typing
                .tick(pad, dt_ms)
                .map(|action| self.with_shift_lock(action)),
            // TODO(P2 ถัดไป): pointer math/accel (Mouse) และ pivot movement (Nav)
            Mode::Mouse | Mode::Nav => None,
            Mode::Disabled => None,
        }
    }

    /// ปุ่มหน้าแบบแตะ (spec §4): A = Space · B = Backspace · X = Enter · Y = Tab
    ///
    /// ทำงานทุก mode ที่ไม่ใช่วง/Disabled · ปุ่มหน้าตัด gesture ที่ค้างก่อนเสมอ (spec §3)
    /// - `B` ขณะมี gesture (GroupArmed/CharSelected/Committing): ยกเลิกอย่างเดียว ไม่ส่งคีย์ (spec §2)
    ///
    /// TODO(P2 ถัดไป — interaction model §4): A แตะเร็ว 2 ครั้ง = Enter ·
    /// B ค้าง 400 ms = ลบทั้งคำ · Y ค้าง = Esc · auto-repeat ของปุ่มหน้า
    fn press_face_button(&mut self, pad: &PadState, key: NamedKey) -> Option<Action> {
        let typing_active = self.typing.state() != GestureState::Idle;
        if typing_active {
            self.typing.cancel();
        }

        if typing_active && key == NamedKey::Backspace {
            return None;
        }

        Some(Action::Key(KeyPress {
            key: Key::Named(key),
            // ADR-13: LT เป็น toggle ไม่ใช่ hold → ไม่ส่ง Shift ของ OS ออกไป
            shift: false,
            // LB ยังเป็น Ctrl (spec §5) — เช่น Ctrl+Backspace (ลบทั้งคำ) ในอนาคต
            ctrl: pad.buttons.lb,
        }))
    }

    /// ADR-13: แปลงคีย์ที่พิมพ์ตาม `shift_lock` แทนการส่ง Shift ของ OS
    ///
    /// ล็อกอยู่: ตัวอักษรเล็ก a–z → ตัวใหญ่ · อักขระอื่น (`!` เลข ไทย) คงเดิม
    /// ไม่ล็อก: LT ที่กดค้างไม่มีผลต่อการพิมพ์ (ไม่ใช่ hold-shift)
    fn with_shift_lock(&self, action: Action) -> Action {
        let Action::Key(press) = action else {
            return action;
        };
        let key = match press.key {
            Key::Char(ch) if self.shift_lock && ch.is_ascii_lowercase() => {
                Key::Char(ch.to_ascii_uppercase())
            }
            key => key,
        };
        Action::Key(KeyPress {
            key,
            shift: false,
            ctrl: press.ctrl,
        })
    }

    fn tick_disabled(&mut self, dt_ms: u32) -> Option<Action> {
        if self.disabled_remaining_ms == 0 {
            return None;
        }
        self.disabled_remaining_ms = self.disabled_remaining_ms.saturating_sub(dt_ms);
        if self.disabled_remaining_ms > 0 {
            return None;
        }
        // ครบ deadman → กลับ mode เดิม (spec §4: deadman 3 s)
        let resume = self.resume_mode;
        self.mode = resume;
        Some(Action::SetMode(resume))
    }

    fn enter_disabled(&mut self) -> Action {
        self.resume_mode = if self.mode == Mode::Disabled {
            Mode::Type
        } else {
            self.mode
        };
        self.mode = Mode::Disabled;
        self.disabled_remaining_ms = self.cfg.deadman_ms;
        self.emergency_hold_ms = 0;
        self.close_wheel();
        self.typing.cancel();
        Action::SetMode(Mode::Disabled)
    }

    /* ---------------- วงคำสั่ง / วงภาษา ---------------- */

    fn toggle_cmd_wheel(&mut self, pad: &PadState) {
        if self.wheel.is_some() {
            // D-pad ← ซ้ำ = ยกเลิก (spec §3.1) · gesture ถูกล็อกไว้ตั้งแต่ตอนเปิดวง
            self.close_wheel();
        } else {
            self.open_cmd_wheel(pad);
        }
    }

    fn open_cmd_wheel(&mut self, pad: &PadState) {
        self.close_wheel();
        self.wheel = Some(WheelKind::Command);
        // เปิดวงขณะ RS ยังดันค้าง = ยังไม่ armed (spec §3.1 บรรทัดสุดท้าย)
        self.wheel_armed = pad.rs.magnitude() < self.cfg.gesture.rs_release;
        self.typing.cancel();
    }

    fn open_lang_wheel(&mut self, pad: &PadState) {
        self.close_wheel();
        if self.languages.is_empty() {
            return;
        }
        self.wheel = Some(WheelKind::Language);
        // เพิ่งใช้ RS เลือกช่องภาษาอื่น → ต้องปล่อย RS กลับศูนย์ก่อน (spec §3.2)
        self.wheel_armed = pad.rs.magnitude() < self.cfg.gesture.rs_release;
    }

    fn close_wheel(&mut self) {
        self.wheel = None;
        self.wheel_armed = false;
        self.wheel_index = None;
        self.wheel_dir = None;
        self.wheel_commit_ms = 0;
        self.cmd_sector.reset();
    }

    fn tick_wheel(&mut self, pad: &PadState, dt_ms: u32, a: bool, b: bool) -> Option<Action> {
        let cfg = self.cfg.gesture;
        let kind = self.wheel?;

        // B = ยกเลิกทุกวง (spec §3.1/§3.2)
        if b {
            self.close_wheel();
            return None;
        }

        if kind == WheelKind::Language && self.languages.is_empty() {
            self.close_wheel();
            return None;
        }

        // A = ยืนยันทันที (ถ้ามีช่องที่เลือกอยู่)
        if a {
            return match self.wheel_index {
                Some(index) => self.confirm_wheel(pad, index),
                None => None,
            };
        }

        // armed = false → รอ RS กลับเข้าใกล้ศูนย์ก่อน แล้วจึงรับการเลือก
        if !self.wheel_armed {
            if pad.rs.magnitude() < cfg.rs_release {
                self.wheel_armed = true;
            }
            return None;
        }

        let rm = pad.rs.magnitude();
        if rm >= cfg.rs_activate {
            let sector = self.cmd_sector.update(pad.rs, cfg.rs_activate);
            let index = match kind {
                // ช่องที่ไม่มี (เช่น Language ตอนมีแต่ EN) ไม่ถูกไฮไลต์
                WheelKind::Command => {
                    sector.and_then(|dir| self.command_slot(dir).map(|_| dir.index() as u8))
                }
                WheelKind::Language => Some(self.lang_index_for(pad.rs)),
            };
            // ทิศที่ไฮไลต์ (ทั้งสองวง) — ใช้กับ HUD
            self.wheel_dir = if index.is_some() { sector } else { None };
            if self.wheel_index != index {
                self.wheel_index = index;
                self.wheel_commit_ms = 0;
            }
            return None;
        }

        // ปล่อย RS → ยืนยันหลัง dwell (spec: 60 ms)
        if self.wheel_index.is_some() && rm < cfg.rs_release {
            self.wheel_commit_ms = self.wheel_commit_ms.saturating_add(dt_ms);
            if self.wheel_commit_ms >= cfg.commit_dwell_ms
                && let Some(index) = self.wheel_index
            {
                return self.confirm_wheel(pad, index);
            }
        }
        None
    }

    fn confirm_wheel(&mut self, pad: &PadState, index: u8) -> Option<Action> {
        match self.wheel? {
            WheelKind::Command => self.run_command(pad, index),
            WheelKind::Language => self.commit_lang(index),
        }
    }

    /// ทำคำสั่งของวงคำสั่ง (index = ทิศ)
    fn run_command(&mut self, pad: &PadState, index: u8) -> Option<Action> {
        let command = self.command_slot(Dir8::from_index(index as usize))?;
        self.close_wheel();
        // คำสั่งที่ยืนยันแล้วตัด gesture การพิมพ์ และล็อกไว้ขณะ LS ยังค้าง (spec §3.1)
        self.typing.cancel();

        let action = match command {
            Command::TypeEn => {
                self.apply_mode(Mode::Type);
                self.typing.set_lang(Lang::En);
                Action::Command(command)
            }
            Command::Language => {
                if self.languages.len() == 1 {
                    // มีภาษาอื่นภาษาเดียว = สลับทันที (spec §3.2)
                    let slot = self.languages[0];
                    self.apply_mode(Mode::Type);
                    self.typing.set_lang(slot.lang);
                    Action::SetLang { index: slot.index }
                } else {
                    self.open_lang_wheel(pad);
                    return None;
                }
            }
            Command::Mouse => {
                self.apply_mode(Mode::Mouse);
                Action::Command(command)
            }
            // ↘ Place UI = Nav mode (ADR-11) · `Command::Nav` สงวนไว้ให้ config อื่น
            Command::PlaceUi | Command::Nav => {
                self.apply_mode(Mode::Nav);
                Action::Command(Command::PlaceUi)
            }
            // TODO(P2 ถัดไป): Copy/Paste/Undo/Redo ต้องมี injector + modifier guard ฝั่ง app
            Command::Copy | Command::Paste | Command::Undo | Command::Redo => {
                Action::Command(command)
            }
            // EN/TH ที่เหลือยังไม่ถูกใช้เป็นช่องในวงคำสั่งของ slice นี้
            Command::TypeTh => Action::Command(command),
        };
        Some(action)
    }

    /// ยืนยันภาษาจากวงภาษา → `type-<lang>` (spec §3.2)
    fn commit_lang(&mut self, index: u8) -> Option<Action> {
        let slot = *self.languages.get(index as usize)?;
        self.close_wheel();
        self.typing.cancel();
        self.apply_mode(Mode::Type);
        self.typing.set_lang(slot.lang);
        Some(Action::SetLang { index: slot.index })
    }

    /// แบ่งวงภาษาเป็นช่องเท่ากันตามจำนวนภาษา (spec §3.2) แล้วคืน index
    fn lang_index_for(&self, rs: Vec2) -> u8 {
        let n = self.languages.len().max(1);
        let segment = 360.0 / n as f32;
        ((rs.angle_deg() / segment).round() as usize % n) as u8
    }

    /// เปลี่ยน mode ภายใน + ตัด gesture ที่ค้าง (spec §7: สลับ mode กลาง gesture = ยกเลิก)
    fn apply_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.typing.cancel();
        self.close_wheel();
    }
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Key, KeyPress, NamedKey};
    use crate::pad::Buttons;

    const D: u32 = 4;

    fn p(ls: (f32, f32), rs: (f32, f32)) -> PadState {
        PadState {
            ls: Vec2::new(ls.0, ls.1),
            rs: Vec2::new(rs.0, rs.1),
            ..PadState::default()
        }
    }

    fn with(mut pad: PadState, f: impl FnOnce(&mut Buttons)) -> PadState {
        f(&mut pad.buttons);
        pad
    }

    fn dpad_left() -> PadState {
        with(p((0.0, 0.0), (0.0, 0.0)), |b| b.dpad_left = true)
    }

    fn engine() -> RuntimeEngine {
        RuntimeEngine::default()
    }

    fn engine_with(langs: &[(u8, Lang)]) -> RuntimeEngine {
        let mut e = RuntimeEngine::default();
        e.set_languages(
            langs
                .iter()
                .copied()
                .map(|(index, lang)| LangSlot::new(index, lang))
                .collect(),
        );
        e
    }

    /// เปิดวงคำสั่งด้วย D-pad ← (edge) แล้วปล่อยปุ่ม
    fn open_cmd(e: &mut RuntimeEngine, rs: (f32, f32)) {
        let pad = with(p((0.0, 0.0), rs), |b| b.dpad_left = true);
        assert_eq!(e.tick(&pad, D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
        // ปล่อย D-pad ทิ้งไว้หนึ่งเฟรมเพื่อให้ edge ถัดไปทำงาน
        e.tick(&p((0.0, 0.0), rs), D);
    }

    /// ปล่อย RS แล้วรอ dwell ครบ 60 ms
    fn release_rs(e: &mut RuntimeEngine) -> Option<Action> {
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), 64)
    }

    fn hold(e: &mut RuntimeEngine, pad: PadState, mut total_ms: u32, step: u32) -> Option<Action> {
        while total_ms > 0 {
            let dt = step.min(total_ms);
            total_ms -= dt;
            if let Some(action) = e.tick(&pad, dt) {
                return Some(action);
            }
        }
        None
    }

    /* ---------------- วงคำสั่ง ---------------- */

    #[test]
    fn dpad_left_opens_and_closes_cmd_wheel() {
        let mut e = engine();
        assert_eq!(e.mode(), Mode::Type);
        assert_eq!(e.wheel(), None);

        // กด D-pad ← → เปิดวง (ไม่มี action ออกมา)
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
        assert!(e.wheel_armed());
        assert_eq!(e.wheel_index(), None);

        // กดค้างไม่เปิดซ้ำ
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        // ปล่อยแล้วกดซ้ำ = ปิดวง (toggle)
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), None);
    }

    #[test]
    fn ls_has_no_effect_in_cmd_wheel() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        assert_eq!(e.wheel_index(), None);

        // ดัน LS ไปทุกทิศ (ไม่แตะ RS) → ไม่มีช่องถูกเลือก ไม่มีคำสั่ง
        for ls in [(0.0, 0.9), (0.9, 0.0), (0.0, -0.9), (-0.9, 0.0)] {
            for _ in 0..20 {
                assert_eq!(e.tick(&p(ls, (0.0, 0.0)), D), None);
            }
            assert_eq!(e.wheel_index(), None);
        }

        // LS ค้างไว้ตอนกด RS → RS ยังเลือกได้ตามปกติ (LS ไม่มีผล)
        e.tick(&p((0.0, -0.9), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
        assert_eq!(e.wheel(), Some(WheelKind::Command));
    }

    #[test]
    fn cmd_wheel_opened_while_rs_held_does_not_arm_until_center() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.9));
        assert!(!e.wheel_armed(), "RS ค้างตอนเปิดวง → ต้องยังไม่ armed");
        assert_eq!(e.wheel_index(), None);

        // ค้าง RS ไว้นานเท่าไรก็ไม่เลือก และไม่ยืนยัน
        for _ in 0..100 {
            assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), D), None);
        }
        assert_eq!(e.wheel_index(), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        // ปล่อย RS กลับศูนย์ → armed
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(e.wheel_armed());

        // หลังจากนั้นจึงเลือกได้
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
    }

    #[test]
    fn rs_select_then_release_commits_exactly_once_after_dwell() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (-0.9, 0.0)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::W.index() as u8));

        // ยังไม่ครบ dwell → ยังไม่ทำคำสั่ง
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), 30), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        let action = e.tick(&p((0.0, 0.0), (0.0, 0.0)), 40).expect("ต้องทำคำสั่ง");
        assert_eq!(action, Action::Command(Command::Undo));
        assert_eq!(e.wheel(), None);

        // ไม่ทำซ้ำ
        for _ in 0..20 {
            assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
        }
    }

    #[test]
    fn a_confirms_immediately_without_dwell() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));

        let press_a = with(p((0.0, 0.0), (0.0, 0.9)), |b| b.a = true);
        let action = e.tick(&press_a, D).expect("A = ยืนยันทันที");
        assert_eq!(action, Action::Command(Command::TypeEn));
        assert_eq!(e.wheel(), None);
        assert_eq!(e.mode(), Mode::Type);
        assert_eq!(e.lang(), Lang::En);

        // กด A ค้างต่อ → ไม่ทำซ้ำ
        for _ in 0..10 {
            assert_eq!(e.tick(&press_a, D), None);
        }
    }

    #[test]
    fn a_without_selection_does_nothing() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        let press_a = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
    }

    #[test]
    fn b_and_dpad_left_cancel_the_cmd_wheel() {
        // B
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
        let press_b = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.b = true);
        assert_eq!(e.tick(&press_b, D), None);
        assert_eq!(e.wheel(), None);
        // ปล่อย RS ต่อ → ไม่มีคำสั่งหลุด
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.mode(), Mode::Type);

        // D-pad ← ซ้ำ
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (-0.9, 0.0)), D);
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), None);
        assert_eq!(release_rs(&mut e), None);
    }

    #[test]
    fn a_confirm_while_ls_held_sets_lock_and_types_nothing() {
        let mut e = engine();
        // LS ค้างอยู่ทั้งกระบวนการ
        let ls_held = (0.0, 0.9);
        e.tick(&p(ls_held, (0.0, 0.0)), D); // เริ่ม gesture พิมพ์
        assert_eq!(e.gesture_state(), GestureState::GroupArmed);

        // เปิดวงคำสั่ง → ตัด gesture
        open_cmd(&mut e, (0.0, 0.0));
        assert_eq!(e.gesture_state(), GestureState::Idle);

        e.tick(&p(ls_held, (0.0, 0.9)), D);
        assert_eq!(e.wheel_highlight_dir(), Some(Dir8::N));

        // A ขณะ LS ค้าง → ทำคำสั่ง + lock
        let press_a = with(p(ls_held, (0.0, 0.9)), |b| b.a = true);
        let action = e.tick(&press_a, D).expect("A ต้องทำคำสั่ง");
        assert_eq!(action, Action::Command(Command::TypeEn));
        assert_eq!(e.wheel(), None);

        // LS ยังค้าง + RS ดัน/ปล่อย → ต้องไม่มีตัวอักษรออกมา
        for _ in 0..30 {
            assert_eq!(e.tick(&p(ls_held, (0.0, 0.9)), D), None);
        }
        assert_eq!(e.tick(&p(ls_held, (0.0, 0.0)), D), None);
        assert_eq!(e.tick(&p(ls_held, (0.0, 0.0)), 200), None);
        assert_eq!(e.gesture_state(), GestureState::Idle);

        // ปล่อย LS → ปลด lock แล้วพิมพ์ได้ตามปกติ
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&p(ls_held, (0.0, 0.0)), D);
        e.tick(&p(ls_held, (0.0, 0.9)), D);
        e.tick(&p(ls_held, (0.0, 0.0)), D);
        let action = e.tick(&p(ls_held, (0.0, 0.0)), 64);
        assert_eq!(
            action,
            Some(Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false,
            }))
        );
    }

    #[test]
    fn opening_cmd_wheel_cancels_typing_gesture() {
        let mut e = engine();
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        // D-pad ← ระหว่างเลือกตัวอักษร → ตัด gesture ไม่พิมพ์
        open_cmd(&mut e, (0.0, 0.0));
        assert_eq!(e.gesture_state(), GestureState::Idle);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        // ปล่อย RS แล้วรอ dwell — ต้องไม่มีคีย์
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.tick(&p((0.0, 0.9), (0.0, 0.0)), 200), None);
        assert_eq!(e.gesture_state(), GestureState::Idle);
    }

    /* ---------------- วงคำสั่ง → โหมด ---------------- */

    #[test]
    fn mouse_and_place_ui_slots_switch_mode() {
        // → Mouse
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.0)), D);
        let action = release_rs(&mut e).expect("ต้องเลือก Mouse");
        assert_eq!(action, Action::Command(Command::Mouse));
        assert_eq!(e.mode(), Mode::Mouse);
        assert_eq!(e.wheel(), None);

        // Mouse mode: LS/RS ไม่พิมพ์
        for _ in 0..30 {
            assert_eq!(e.tick(&p((0.0, 0.9), (0.0, 0.9)), D), None);
        }

        // กลับมาที่ Type EN ผ่านช่อง ↑ แล้วไป ↘ Place UI
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(release_rs(&mut e), Some(Action::Command(Command::TypeEn)));
        assert_eq!(e.mode(), Mode::Type);

        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, -0.9)), D);
        assert_eq!(e.wheel_highlight_dir(), Some(Dir8::SE));
        let action = release_rs(&mut e).expect("ต้องเลือก Place UI");
        assert_eq!(action, Action::Command(Command::PlaceUi));
        assert_eq!(e.mode(), Mode::Nav);
    }

    /* ---------------- วงภาษา ---------------- */

    #[test]
    fn single_other_language_switches_immediately() {
        let mut e = engine_with(&[(1, Lang::Th)]);
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(e.wheel_highlight_dir(), Some(Dir8::NE));

        let action = release_rs(&mut e).expect("ต้องสลับภาษาทันที");
        assert_eq!(action, Action::SetLang { index: 1 });
        assert_eq!(e.lang(), Lang::Th);
        assert_eq!(e.mode(), Mode::Type);
        assert_eq!(e.wheel(), None, "มีภาษาเดียว = ไม่เปิดวงภาษา");
    }

    #[test]
    fn no_other_language_means_no_slot() {
        let mut e = engine(); // มีแต่ EN
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(e.wheel_highlight_dir(), None, "ช่องภาษาอื่นต้องไม่มี");
        assert_eq!(e.command_slot(Dir8::NE), None);

        // ปล่อย RS → ไม่มีอะไรเกิดขึ้น วงยังเปิด
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        // A ก็ไม่ทำอะไรเพราะไม่มีช่องที่เลือก
        let press_a = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
    }

    #[test]
    fn many_languages_open_lang_wheel_and_select_with_rs() {
        // ใช้ TH สองช่องที่ index ต่างกันเพื่อทดสอบกลไกวง + SetLang{index}
        // (ภาษาที่สองที่ต่างจริงจะมาจาก HKL ตอน P5a)
        let mut e = engine_with(&[(1, Lang::Th), (2, Lang::Th)]);

        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(
            release_rs(&mut e),
            None,
            "มากกว่า 1 ภาษา = เปิดวงภาษา ไม่สลับทันที"
        );
        assert_eq!(e.wheel(), Some(WheelKind::Language));
        assert_eq!(e.wheel_index(), None, "ยังไม่มีไฮไลต์");

        // ยังไม่เลือกใคร → ปล่อย RS ไม่เปลี่ยนภาษา
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.lang(), Lang::En);
        assert_eq!(e.wheel(), Some(WheelKind::Language));

        // RS ขึ้น = ช่องแรก (index 1)
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(0));
        let action = release_rs(&mut e).expect("ต้องเปลี่ยนภาษา");
        assert_eq!(action, Action::SetLang { index: 1 });
        assert_eq!(e.lang(), Lang::Th);
        assert_eq!(e.wheel(), None);

        // เปิดใหม่แล้วเลือกช่องที่สอง (index 2) ด้วย RS ลง
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.wheel(), Some(WheelKind::Language));
        e.tick(&p((0.0, 0.0), (0.0, -0.9)), D);
        assert_eq!(e.wheel_index(), Some(1));
        let action = release_rs(&mut e).expect("ต้องเปลี่ยนภาษา");
        assert_eq!(action, Action::SetLang { index: 2 });
        assert_eq!(e.lang(), Lang::Th);
        assert_eq!(e.mode(), Mode::Type);
    }

    #[test]
    fn lang_wheel_waits_for_rs_center_after_opening() {
        let mut e = engine_with(&[(1, Lang::Th), (2, Lang::Th)]);
        open_cmd(&mut e, (0.0, 0.0));
        // เลือกช่อง Language ด้วย RS ↗
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(e.wheel_highlight_dir(), Some(Dir8::NE));

        // A ขณะ RS ยังดันอยู่ → เปิดวงภาษา แต่ยังไม่ armed
        let press_a = with(p((0.0, 0.0), (0.9, 0.9)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), None);
        assert_eq!(e.wheel(), Some(WheelKind::Language));
        assert!(!e.wheel_armed());
        assert_eq!(e.wheel_index(), None);

        // ดัน RS ต่อ (ทิศขึ้น) → ต้องยังไม่เลือก
        for _ in 0..50 {
            assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), D), None);
        }
        assert_eq!(e.wheel_index(), None);

        // ปล่อย RS กลับศูนย์ → armed
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(e.wheel_armed());

        // แล้วจึงเลือกได้
        e.tick(&p((0.0, 0.0), (0.0, -0.9)), D);
        assert_eq!(e.wheel_index(), Some(1));
        assert_eq!(release_rs(&mut e), Some(Action::SetLang { index: 2 }));
        assert_eq!(e.lang(), Lang::Th);
    }

    #[test]
    fn lang_wheel_a_confirms_b_and_dpad_cancel() {
        // A ยืนยันทันที
        let mut e = engine_with(&[(1, Lang::Th), (2, Lang::Th)]);
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(release_rs(&mut e), None);
        e.tick(&p((0.0, 0.0), (0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(0));
        let press_a = with(p((0.0, 0.0), (0.0, 0.9)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), Some(Action::SetLang { index: 1 }));
        assert_eq!(e.lang(), Lang::Th);
        assert_eq!(e.wheel(), None);

        // B ยกเลิก — ภาษาเดิมไม่เปลี่ยน
        let mut e = engine_with(&[(1, Lang::Th), (2, Lang::Th)]);
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.wheel(), Some(WheelKind::Language));
        e.tick(&p((0.0, 0.0), (0.0, -0.9)), D);
        let press_b = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.b = true);
        assert_eq!(e.tick(&press_b, D), None);
        assert_eq!(e.wheel(), None);
        assert_eq!(e.lang(), Lang::En);
        assert_eq!(release_rs(&mut e), None);

        // D-pad ← ยกเลิก
        let mut e = engine_with(&[(1, Lang::Th), (2, Lang::Th)]);
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.9)), D);
        assert_eq!(release_rs(&mut e), None);
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), None);
        assert_eq!(e.lang(), Lang::En);
    }

    #[test]
    fn disabling_the_current_language_falls_back_to_en() {
        let mut e = engine_with(&[(1, Lang::Th)]);
        e.set_languages(vec![LangSlot::new(1, Lang::Th)]);
        e.set_lang(Lang::Th);
        assert_eq!(e.lang(), Lang::Th);

        e.set_languages(Vec::new());
        assert_eq!(e.lang(), Lang::En, "ปิดภาษาที่ใช้อยู่ → กลับ EN");
        assert_eq!(e.command_slot(Dir8::NE), None);
    }

    /* ---------------- mode arbiter / emergency ---------------- */

    #[test]
    fn switching_mode_cancels_in_progress_typing_gesture() {
        let mut e = engine();
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        e.set_mode(Mode::Mouse);
        assert_eq!(e.gesture_state(), GestureState::Idle);
        assert_eq!(e.mode(), Mode::Mouse);

        // แกนยังค้าง → ไม่มีคีย์
        for _ in 0..30 {
            assert_eq!(e.tick(&p((0.0, 0.9), (0.0, 0.9)), D), None);
        }

        // กลับ Type แล้วพิมพ์ได้ตามปกติ
        e.set_mode(Mode::Type);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        let action = e.tick(&p((0.0, 0.9), (0.0, 0.0)), 64);
        assert_eq!(
            action,
            Some(Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false,
            }))
        );
    }

    #[test]
    fn switching_mode_via_wheel_cancels_gesture_without_typing() {
        let mut e = engine();
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        open_cmd(&mut e, (0.0, 0.0));
        assert_eq!(e.gesture_state(), GestureState::Idle);

        e.tick(&p((0.0, 0.0), (0.9, 0.0)), D);
        assert_eq!(release_rs(&mut e), Some(Action::Command(Command::Mouse)));
        assert_eq!(e.mode(), Mode::Mouse);

        // ปล่อยแกนทั้งหมด → ไม่มีคีย์ค้าง
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        for _ in 0..10 {
            assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
        }
    }

    #[test]
    fn emergency_lb_rb_for_1_5s_disables_then_recovers_after_3s() {
        let mut e = engine();
        let both = with(p((0.0, 0.0), (0.0, 0.0)), |b| {
            b.lb = true;
            b.rb = true;
        });

        // ยังไม่ครบ 1.5 s → ยังไม่ปิด
        assert_eq!(hold(&mut e, both, 1400, 100), None);
        assert_eq!(e.mode(), Mode::Type);

        // ครบ 1.5 s → Disabled
        let action = hold(&mut e, both, 100, 100).expect("ต้องเข้า Disabled");
        assert_eq!(action, Action::SetMode(Mode::Disabled));
        assert_eq!(e.mode(), Mode::Disabled);
        assert_eq!(e.disabled_remaining_ms(), 3000);
        assert_eq!(e.gesture_state(), GestureState::Idle);

        // ระหว่าง Disabled: input ถูกเมิน
        assert_eq!(e.tick(&dpad_left(), D), None);
        assert_eq!(e.wheel(), None);
        let press_a = with(p((0.0, 0.9), (0.0, 0.9)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), None);
        assert_eq!(e.wheel(), None);
        assert_eq!(e.gesture_state(), GestureState::Idle);

        // ยังไม่ครบ 3 s → ยัง Disabled
        assert_eq!(hold(&mut e, p((0.0, 0.0), (0.0, 0.0)), 2900, 100), None);
        assert_eq!(e.mode(), Mode::Disabled);

        // ครบ 3 s → กลับ mode เดิม
        let action = hold(&mut e, p((0.0, 0.0), (0.0, 0.0)), 200, 100).expect("ต้องกลับมารับ input");
        assert_eq!(action, Action::SetMode(Mode::Type));
        assert_eq!(e.mode(), Mode::Type);
        assert_eq!(e.disabled_remaining_ms(), 0);
    }

    #[test]
    fn emergency_requires_both_buttons_held_continuously() {
        let mut e = engine();
        let lb_only = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.lb = true);
        assert_eq!(hold(&mut e, lb_only, 2000, 100), None);
        assert_eq!(e.mode(), Mode::Type);

        // LB+RB 1.4 s แล้วปล่อย RB → ตัวจับเวลาเริ่มใหม่
        let both = with(p((0.0, 0.0), (0.0, 0.0)), |b| {
            b.lb = true;
            b.rb = true;
        });
        assert_eq!(hold(&mut e, both, 1400, 100), None);
        e.tick(&lb_only, 100);
        assert_eq!(hold(&mut e, both, 1400, 100), None);
        assert_eq!(e.mode(), Mode::Type);
    }

    #[test]
    fn emergency_resumes_previous_non_typing_mode() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), (0.9, 0.0)), D);
        assert_eq!(release_rs(&mut e), Some(Action::Command(Command::Mouse)));
        assert_eq!(e.mode(), Mode::Mouse);

        let both = with(p((0.0, 0.0), (0.0, 0.0)), |b| {
            b.lb = true;
            b.rb = true;
        });
        assert_eq!(
            hold(&mut e, both, 1500, 100),
            Some(Action::SetMode(Mode::Disabled))
        );
        assert_eq!(e.mode(), Mode::Disabled);

        let action = hold(&mut e, p((0.0, 0.0), (0.0, 0.0)), 3000, 100).expect("ต้องกลับมา");
        assert_eq!(action, Action::SetMode(Mode::Mouse));
        assert_eq!(e.mode(), Mode::Mouse);
    }

    /* ---------------- slice 2: Shift toggle (LT, ADR-13) ---------------- */

    /// แตะปุ่ม 1 ครั้ง (rising edge) บนแกนที่ปล่อยอยู่ แล้วปล่อย — คืน action ของเฟรมที่กด
    fn tap(e: &mut RuntimeEngine, f: fn(&mut Buttons)) -> Option<Action> {
        let pressed = with(p((0.0, 0.0), (0.0, 0.0)), f);
        let action = e.tick(&pressed, D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        action
    }

    /// พิมพ์ตัวอักษร 1 ตัว (arm → select → ปล่อย RS → dwell)
    fn type_dir(e: &mut RuntimeEngine, ls: (f32, f32), rs: (f32, f32)) -> Option<Action> {
        e.tick(&p(ls, (0.0, 0.0)), D);
        e.tick(&p(ls, rs), D);
        e.tick(&p(ls, (0.0, 0.0)), D);
        e.tick(&p(ls, (0.0, 0.0)), 64)
    }

    fn key(k: Key) -> Option<Action> {
        Some(Action::Key(KeyPress {
            key: k,
            shift: false,
            ctrl: false,
        }))
    }

    const LS_N: (f32, f32) = (0.0, 0.9);

    #[test]
    fn lt_tap_toggles_shift_lock_and_hold_does_not_retrigger() {
        let mut e = engine();
        assert!(!e.shift_lock());

        // แตะ 1 = ล็อก
        assert_eq!(tap(&mut e, |b| b.lt = true), None);
        assert!(e.shift_lock());
        // แตะ 2 = ปลด
        assert_eq!(tap(&mut e, |b| b.lt = true), None);
        assert!(!e.shift_lock());

        // กดค้าง = สลับครั้งเดียวต่อการกด ไม่สลับซ้ำ
        let held = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.lt = true);
        assert_eq!(e.tick(&held, D), None);
        assert!(e.shift_lock());
        for _ in 0..250 {
            assert_eq!(e.tick(&held, D), None);
            assert!(e.shift_lock(), "LT ค้างต้องไม่สลับซ้ำ");
        }
    }

    #[test]
    fn shift_lock_uppercases_ascii_letters_only() {
        let mut e = engine();
        tap(&mut e, |b| b.lt = true);
        assert!(e.shift_lock());

        // 'a' → 'A' และไม่พึ่ง Shift ของ OS
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('A')));
        // ล็อกคงอยู่หลังพิมพ์ (ไม่รีเซ็ตอัตโนมัติ)
        assert!(e.shift_lock());
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('A')));

        // '!' ต้องคงเป็น '!' (ไม่กลายเป็นคีย์ผสม)
        let group_w = (-0.9, 0.0);
        assert_eq!(type_dir(&mut e, group_w, LS_N), key(Key::Char('!')));
        assert!(e.shift_lock());

        // ไทยไม่มีตัวพิมพ์ใหญ่ → คงเดิม (ไม่ถูก to_ascii_uppercase ทำพัง)
        e.set_lang(Lang::Th);
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('ก')));
    }

    #[test]
    fn shift_lock_keeps_ctrl_and_never_sets_shift() {
        let mut e = engine();
        tap(&mut e, |b| b.lt = true);

        let lb = |ls: (f32, f32), rs: (f32, f32)| with(p(ls, rs), |b| b.lb = true);
        e.tick(&lb(LS_N, (0.0, 0.0)), D);
        e.tick(&lb(LS_N, LS_N), D);
        e.tick(&lb(LS_N, (0.0, 0.0)), D);
        let action = e.tick(&lb(LS_N, (0.0, 0.0)), 64);
        assert_eq!(
            action,
            Some(Action::Key(KeyPress {
                key: Key::Char('A'),
                shift: false,
                ctrl: true,
            })),
            "LB ต้องยังเป็น Ctrl"
        );
    }

    #[test]
    fn lt_held_while_unlocked_is_not_hold_shift() {
        let mut e = engine();
        // LT ถูกกดตอนอยู่ Mouse (ไม่ toggle) แล้วสลับมา Type ขณะยังค้าง → ยังปลดล็อก
        e.set_mode(Mode::Mouse);
        let held = |ls: (f32, f32), rs: (f32, f32)| with(p(ls, rs), |b| b.lt = true);
        assert_eq!(e.tick(&held((0.0, 0.0), (0.0, 0.0)), D), None);
        assert!(!e.shift_lock());
        e.set_mode(Mode::Type);
        // set_mode ตัด gesture + lock ไว้ → ปล่อย LS ให้ปลดก่อน
        assert_eq!(e.tick(&held((0.0, 0.0), (0.0, 0.0)), D), None);

        // พิมพ์ 'a' ขณะ LT ค้างอยู่ แต่ไม่ได้ล็อก → ตัวเล็ก ไม่มี Shift
        e.tick(&held(LS_N, (0.0, 0.0)), D);
        e.tick(&held(LS_N, LS_N), D);
        e.tick(&held(LS_N, (0.0, 0.0)), D);
        let action = e.tick(&held(LS_N, (0.0, 0.0)), 64);
        assert_eq!(action, key(Key::Char('a')));
        assert!(!e.shift_lock());
    }

    #[test]
    fn lt_tap_in_mouse_and_nav_leaves_shift_lock_unchanged() {
        let mut e = engine();
        e.set_mode(Mode::Mouse);
        assert_eq!(tap(&mut e, |b| b.lt = true), None);
        assert!(!e.shift_lock(), "Mouse: LT ยังเป็น hold (ยังไม่ทำ) ไม่ใช่ toggle");

        e.set_mode(Mode::Nav);
        assert_eq!(tap(&mut e, |b| b.lt = true), None);
        assert!(!e.shift_lock());

        // ล็อกอยู่แล้ว → Mouse/Nav ต้องไม่ปลด
        e.set_mode(Mode::Type);
        tap(&mut e, |b| b.lt = true);
        assert!(e.shift_lock());
        for mode in [Mode::Mouse, Mode::Nav] {
            e.set_mode(mode);
            assert_eq!(tap(&mut e, |b| b.lt = true), None);
            assert!(e.shift_lock(), "{mode:?} ต้องไม่แตะ shift_lock");
        }
    }

    /* ---------------- slice 2: ปุ่มหน้า (tap) ---------------- */

    #[test]
    fn face_buttons_tap_in_type_mode_emit_named_keys_only() {
        type Case = (&'static str, fn(&mut Buttons), NamedKey);
        let mut e = engine();
        let cases: [Case; 4] = [
            ("A", |b| b.a = true, NamedKey::Space),
            ("B", |b| b.b = true, NamedKey::Backspace),
            ("X", |b| b.x = true, NamedKey::Enter),
            ("Y", |b| b.y = true, NamedKey::Tab),
        ];

        for (name, press, expected) in cases {
            let action = tap(&mut e, press).unwrap_or_else(|| panic!("{name} ต้องส่งคีย์"));
            assert_eq!(Some(action), key(Key::Named(expected)));
            // หลังแตะ: ต้องไม่มีตัวอักษรหลุดออกมาอีก
            for _ in 0..10 {
                assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
            }
            assert_eq!(e.gesture_state(), GestureState::Idle);
        }
    }

    #[test]
    fn face_buttons_work_in_nav_mode() {
        let mut e = engine();
        e.set_mode(Mode::Nav);
        assert_eq!(
            tap(&mut e, |b| b.x = true),
            key(Key::Named(NamedKey::Enter))
        );
        assert_eq!(
            tap(&mut e, |b| b.b = true),
            key(Key::Named(NamedKey::Backspace))
        );
        assert_eq!(
            tap(&mut e, |b| b.a = true),
            key(Key::Named(NamedKey::Space))
        );
        assert_eq!(tap(&mut e, |b| b.y = true), key(Key::Named(NamedKey::Tab)));
        assert_eq!(e.gesture_state(), GestureState::Idle);
    }

    #[test]
    fn b_cancels_gesture_and_emits_nothing() {
        // GROUP_ARMED
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.gesture_state(), GestureState::GroupArmed);
        let press_b = with(p(LS_N, (0.0, 0.0)), |b| b.b = true);
        assert_eq!(e.tick(&press_b, D), None, "B ขณะ armed = ยกเลิกอย่างเดียว");
        assert_eq!(e.gesture_state(), GestureState::Idle);
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 200), None);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), 200), None);

        // CHAR_SELECTED
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        e.tick(&p(LS_N, LS_N), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);
        let press_b = with(p(LS_N, LS_N), |b| b.b = true);
        assert_eq!(e.tick(&press_b, D), None, "B ขณะเลือก = ยกเลิก");
        assert_eq!(e.gesture_state(), GestureState::Idle);
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 200), None);

        // COMMITTING — ใช้ dt ยาวพอที่จะ commit ถ้าไม่ถูกตัด
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        e.tick(&p(LS_N, LS_N), D);
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.gesture_state(), GestureState::Committing);
        let press_b = with(p(LS_N, (0.0, 0.0)), |b| b.b = true);
        assert_eq!(
            e.tick(&press_b, 100),
            None,
            "B ขณะ committing = ยกเลิก ไม่พิมพ์"
        );
        assert_eq!(e.gesture_state(), GestureState::Idle);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 100), None);
    }

    #[test]
    fn b_when_idle_is_backspace_and_cancels_nothing() {
        let mut e = engine();
        assert_eq!(
            tap(&mut e, |b| b.b = true),
            key(Key::Named(NamedKey::Backspace))
        );
        // แตะซ้ำได้ (ไม่ค้าง state)
        assert_eq!(
            tap(&mut e, |b| b.b = true),
            key(Key::Named(NamedKey::Backspace))
        );
    }

    #[test]
    fn a_while_selecting_cancels_gesture_and_emits_space_only() {
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        e.tick(&p(LS_N, LS_N), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        let press_a = with(p(LS_N, LS_N), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), key(Key::Named(NamedKey::Space)));
        assert_eq!(e.gesture_state(), GestureState::Idle);

        // ปล่อย RS แล้วรอ dwell → ต้องไม่มีตัวอักษรออกมา
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 200), None);

        // ปล่อย LS → พิมพ์ใหม่ได้ตามปกติ
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('a')));
    }

    #[test]
    fn face_buttons_do_not_fire_while_wheel_is_open() {
        let mut e = engine();
        open_cmd(&mut e, (0.0, 0.0));

        // A ตอนยังไม่มีช่อง → ไม่ส่ง Space และไม่ปิดวง
        let press_a = with(p((0.0, 0.0), (0.0, 0.0)), |b| b.a = true);
        assert_eq!(e.tick(&press_a, D), None);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.wheel(), Some(WheelKind::Command));

        // X/Y ไม่ส่ง Enter/Tab (วงใช้ A/B เท่านั้น)
        for press in [|b: &mut Buttons| b.x = true, |b: &mut Buttons| b.y = true] {
            let pad = with(p((0.0, 0.0), (0.0, 0.0)), press);
            assert_eq!(e.tick(&pad, D), None);
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
            assert_eq!(e.wheel(), Some(WheelKind::Command));
        }

        // LT ในวง = ไม่ toggle (AMBIGUOUS: ดูรายงาน slice 2)
        assert_eq!(tap(&mut e, |b| b.lt = true), None);
        assert!(!e.shift_lock());

        // B = ยกเลิกวง ไม่ส่ง Backspace
        assert_eq!(tap(&mut e, |b| b.b = true), None);
        assert_eq!(e.wheel(), None);
    }
}
