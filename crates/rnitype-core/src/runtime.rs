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
//! ขอบเขตของ slice 3 (P2 Track A — mouse/Nav/repeat/layer):
//! - Mouse mode: RS → pointer (gamma/accel/smoothing) · RB/RT = คลิกซ้าย/ขวา ·
//!   RS click = recenter · LS = scroll (แนวนอน/แนวตั้ง/↖ Home/↘ End)
//! - Nav mode: LS หยาบ · RS ละเอียด · LT precision · RS click = pivot กลับจุดเริ่มต้น
//! - auto-repeat: หลัง commit ถ้าดัน RS กลับทิศเดิมค้าง 350 ms → ซ้ำทุก 90 ms
//! - layer 2: RB/RT (ค้าง = ชั่วคราว · แตะสั้นโดยไม่พิมพ์ = ล็อก)
//! - ปุ่มหน้า: A double-tap = Enter · B hold 400 ms = Ctrl+Backspace · Y hold = Esc
//!
//! ขอบเขตของ slice 4 (P2 Track A — ADR-23):
//! - quick bar (LB ค้าง): 8 ช่อง RS · ปล่อย RS = คีย์ลัด · ปล่อย LB/B = ยกเลิก ·
//!   ดัน LS ขณะ LB = กลับไปพิมพ์ Ctrl+ตัวอักษร · Alt+Tab ใช้ `KeyPress.alt`
//! - caret nav (R3): เปิด/ปิดลูกศร · แตะ 2 ครั้ง = Shift+ลูกศร · repeat 350/90 ms
//! - sticky drag (Mouse): RB แตะ 2 ครั้ง = ล็อกปุ่มซ้ายค้าง · แตะอีกครั้ง = ปล่อย
//!
//! ยังไม่ทำ (งานถัดไป):
//! - pivot clamp ต่อจอ/mixed-DPI (ตอนนี้ `Session` clamp ด้วย virtual desktop เดียว)
//! - ตรวจ exclusion list / จอยหลุด (เป็นงานฝั่ง app–win ไม่ใช่ core)

use crate::action::{Action, Command, Key, KeyPress, Mode, MouseButton, NamedKey, QuickAction};
use crate::gesture::{FAST_CENTER_OUT, FastInput, FastNav, FastPhase, FastZone};
use crate::gesture::{GestureConfig, GestureState};
use crate::keymap::{Keymap, Lang};
use crate::normalizer::{Dir8, SectorTracker};
use crate::pad::{PadState, Vec2};
use crate::pointer::{
    PivotConfig, PivotMover, PointerConfig, PointerMover, ScrollAxis, ScrollConfig, ScrollEngine,
};
use crate::typing::TypingPipeline;

/// ภาษาที่สองสูงสุดที่วงภาษารองรับ (spec §3.2 — สูงสุด 8)
pub const MAX_LANGUAGES: usize = 8;

/// ค่าคงที่เวลาของ runtime — ค่าเริ่มต้นตาม spec §4
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeConfig {
    /// deadzone / dwell / hysteresis ของ gesture การพิมพ์ (ใช้ร่วมกับวงคำสั่ง)
    pub gesture: GestureConfig,
    /// LB ต้องค้างนานเท่านี้จึงเปิด quick bar (แตะสั้นยังเป็น Ctrl chord ได้)
    pub quickbar_hold_ms: u32,
    /// LB+RB ต้องค้างนานเท่านี้จึงเข้า `DISABLED` (spec §4: 1.5 s)
    pub emergency_hold_ms: u32,
    /// ระยะเวลาที่หยุดรับ input หลังกดปิดฉุกเฉิน (spec §4: `deadman` 3 s)
    pub deadman_ms: u32,
    /// Mouse mode: สูตร pointer
    pub pointer: PointerConfig,
    /// Nav mode: ความเร็ว pivot
    pub pivot: PivotConfig,
    /// Mouse mode: LS → wheel
    pub scroll: ScrollConfig,
    /// A สองครั้งภายในเวลานี้ = Enter · RB/RT แตะ = ล็อกชั้น 2 (spec: 280 ms)
    pub double_tap_ms: u32,
    /// B ค้าง / Y ค้าง ต้องนานเท่านี้ (interaction model §4: 400 ms)
    pub face_hold_ms: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            gesture: GestureConfig::default(),
            quickbar_hold_ms: 150,
            emergency_hold_ms: 1500,
            deadman_ms: 3000,
            pointer: PointerConfig::default(),
            pivot: PivotConfig::default(),
            scroll: ScrollConfig::default(),
            double_tap_ms: 280,
            face_hold_ms: 400,
        }
    }
}

/// ภาษาในวงภาษา: index ใน `config.languages[]` (ค่าที่ส่งออกใน [`Action::SetLang`])
/// + ตาราง keymap ที่จะใช้ (built-in EN/TH หรือ layout ที่สร้างจาก Windows)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangSlot {
    pub index: u8,
    pub lang: Lang,
    /// รหัสภาษาที่แสดง/ใช้ lookup (`en`, `th`, `ru`, …)
    pub code: String,
    /// ชื่อที่แสดงบน HUD (จาก layout ที่ตรวจพบ)
    pub label: String,
    /// `None` = ใช้ตาราง built-in ของ `lang`
    pub keymap: Option<Keymap>,
}

impl LangSlot {
    pub fn new(index: u8, lang: Lang) -> Self {
        Self {
            index,
            lang,
            code: crate::keymap::lang_code(lang).to_string(),
            label: lang.label().to_string(),
            keymap: None,
        }
    }

    pub fn custom(index: u8, code: &str, label: &str, keymap: Keymap) -> Self {
        Self {
            index,
            lang: Lang::En,
            code: code.to_string(),
            label: label.to_string(),
            keymap: Some(keymap),
        }
    }
}

/// วงที่เปิดอยู่ (HUD ใช้วาดวงใน/level 2)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelKind {
    /// วงคำสั่ง 8 ช่อง (D-pad ←)
    Command,
    /// วงภาษา level 2 (จากช่อง Language ของวงคำสั่ง)
    Language,
    /// quick bar 8 ช่อง (LB ค้าง) — RS เลือก · ปล่อย RS = ทำ
    QuickBar,
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
    prev_dpad_up: bool,
    prev_dpad_down: bool,
    prev_dpad_right: bool,
    prev_a: bool,
    prev_b: bool,
    prev_x: bool,
    prev_y: bool,
    prev_lt: bool,
    prev_rb: bool,
    prev_rt: bool,
    prev_rs_click: bool,
    prev_ls_click: bool,
    prev_view: bool,
    /// Fast mode (Feature 1): LS เลือกกลุ่ม → ล็อก → LS เลือกตัวอักษร → A พิมพ์
    fast: bool,
    /// Fast mode (Feature 3): R3 สองครั้ง = ลูกศร + Shift (highlight)
    fast_select: bool,
    /// action ของ Fast mode ที่ชนกับ A ในเฟรมเดียว (ลูกศร/Backspace/Enter) — ส่งเฟรมถัดไป ไม่ทิ้ง
    fast_queued: Option<Action>,
    /// A ค้าง = พิมพ์ซ้ำ (เหมือนกด Spacebar ค้าง) — เก็บคีย์ที่เพิ่งพิมพ์
    a_repeat: Option<crate::action::KeyPress>,
    a_hold_ms: u32,
    a_repeat_ms: u32,
    /// B ค้าง = Backspace ซ้ำ
    b_repeat_ms: u32,
    /// อนุญาตให้ B ค้างลบซ้ำได้เฉพาะเมื่อการกดครั้งนี้เริ่มด้วย Backspace จริง
    b_repeat_ok: bool,
    /// ADR-13: LT = สลับตัวพิมพ์ใหญ่ (toggle) — คงอยู่จนกดอีกครั้ง ไม่รีเซ็ตหลังพิมพ์
    shift_lock: bool,
    /// ชั้น 2 (RB/RT): แตะ = ล็อก · ค้าง = ชั่วคราว
    layer2_lock: bool,
    layer2_held: bool,
    rb_hold_ms: u32,
    rt_hold_ms: u32,
    /// มี gesture เกิดขึ้นระหว่างกด RB/RT → ไม่นับเป็นการแตะล็อก
    layer2_used: bool,
    prev_layer: u8,
    /// A สองครั้งภายในหน้าต่างนี้ = Enter
    a_window_ms: u32,
    b_hold_ms: u32,
    b_hold_fired: bool,
    y_hold_ms: u32,
    y_pending: bool,
    y_hold_fired: bool,
    pointer: PointerMover,
    scroll: ScrollEngine,
    pivot: PivotMover,
    scroll_diag_fired: bool,
    /* ---- slice 4: quick bar / caret nav / sticky drag ---- */
    /// LB ค้างต่อเนื่องนานเท่าไร (ต้องถึง `quickbar_hold_ms` จึงเปิด quick bar)
    lb_hold_ms: u32,
    /// `true` = การกด LB ครั้งนี้ยังไม่ถูกใช้ (หลังเปิด/แตะแล้วจะไม่เปิดซ้ำจนปล่อย)
    lb_hold_armed: bool,
    /// R3 เปิดโหมดลูกศร (Type mode) — RS = ลูกศร + repeat
    caret: bool,
    /// R3 สองครั้งติดกัน = เลือกข้อความ (ลูกศร + Shift)
    caret_select: bool,
    r3_window_ms: u32,
    caret_sector: SectorTracker,
    caret_dir: Option<Dir8>,
    caret_ms: u32,
    caret_fired: bool,
    /// Mouse mode: RB แตะ 2 ครั้ง = ล็อกปุ่มซ้ายค้าง (ลากยาว)
    sticky_drag: bool,
    rb_tap_window_ms: u32,
    /// การกด RB ครั้งนี้เพิ่งปล่อย sticky drag ไปแล้ว — ห้ามปล่อยซ้ำตอนยกนิ้ว
    rb_sticky_released: bool,
}

/// rising edge ของปุ่มที่ต้องนับครั้งเดียวต่อการกด (กัน auto-repeat ของจอย)
#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    dpad_left: bool,
    dpad_up: bool,
    dpad_down: bool,
    dpad_right: bool,
    a: bool,
    b: bool,
    x: bool,
    y: bool,
    lt: bool,
    rb: bool,
    rt: bool,
    rs_click: bool,
    ls_click: bool,
    view: bool,
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
            prev_dpad_up: false,
            prev_dpad_down: false,
            prev_dpad_right: false,
            prev_a: false,
            prev_b: false,
            prev_x: false,
            prev_y: false,
            prev_lt: false,
            prev_rb: false,
            prev_rt: false,
            prev_rs_click: false,
            prev_ls_click: false,
            prev_view: false,
            fast: false,
            fast_select: false,
            fast_queued: None,
            a_repeat: None,
            a_hold_ms: 0,
            a_repeat_ms: 0,
            b_repeat_ms: 0,
            b_repeat_ok: false,
            shift_lock: false,
            layer2_lock: false,
            layer2_held: false,
            rb_hold_ms: 0,
            rt_hold_ms: 0,
            layer2_used: false,
            prev_layer: 1,
            a_window_ms: 0,
            b_hold_ms: 0,
            b_hold_fired: false,
            y_hold_ms: 0,
            y_pending: false,
            y_hold_fired: false,
            pointer: PointerMover::new(cfg.pointer),
            scroll: ScrollEngine::new(cfg.scroll),
            pivot: PivotMover::new(cfg.pivot),
            scroll_diag_fired: false,
            lb_hold_ms: 0,
            lb_hold_armed: true,
            caret: false,
            caret_select: false,
            r3_window_ms: 0,
            caret_sector: SectorTracker::new(gesture.sector_hysteresis_deg),
            caret_dir: None,
            caret_ms: 0,
            caret_fired: false,
            sticky_drag: false,
            rb_tap_window_ms: 0,
            rb_sticky_released: false,
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

    /// ชั้น keymap ที่ใช้อยู่ (1 = ปกติ · 2 = RB/RT)
    pub fn layer(&self) -> u8 {
        if self.layer2_lock || self.layer2_held {
            2
        } else {
            1
        }
    }

    /// `true` = ล็อกชั้น 2 อยู่ (แตะ RB/RT) — HUD ใช้บอกสถานะ
    pub fn layer2_locked(&self) -> bool {
        self.layer2_lock
    }

    /// จำนวนครั้งที่พิมพ์ซ้ำในรอบ REPEATING (HUD `preview.count`)
    pub fn repeat_count(&self) -> u32 {
        self.typing.repeat_count()
    }

    /* ---------------- Fast mode (Feature 1) ---------------- */

    /// Fast mode เปิดอยู่ (config `defaults.typing.fastMode`)
    pub fn fast(&self) -> bool {
        self.fast
    }

    /// เปิด/ปิด Fast mode จาก config — ปิดแล้วล้างไฮไลต์ทันที
    pub fn set_fast(&mut self, on: bool) {
        if self.fast == on {
            return;
        }
        self.fast = on;
        self.fast_select = false;
        self.fast_queued = None;
        self.typing.set_fast(on);
        self.typing.fast_reset();
        self.r3_window_ms = 0;
        self.caret = false;
        self.caret_select = false;
        self.caret_dir = None;
    }

    /// Fast mode: จังหวะปัจจุบัน (Group = เลือกกลุ่ม · Char = เลือกตัวอักษร)
    pub fn fast_phase(&self) -> FastPhase {
        self.typing.fast_phase()
    }

    /// Fast mode: โซนปัจจุบัน (ซ่อน / หมวด / ตัวอักษร)
    pub fn fast_zone(&self) -> FastZone {
        self.typing.fast_zone()
    }

    /// Fast mode: ระยะเอียง LS ล่าสุด (HUD gauge)
    pub fn fast_radius(&self) -> f32 {
        self.typing.fast_radius()
    }

    /// Fast mode: HUD ถูกซ่อนเพราะปล่อย Analog (ยังพิมพ์ต่อได้)
    pub fn fast_ui_hidden(&self) -> bool {
        self.typing.fast_ui_hidden()
    }

    /// สลับโหมดเลือกตัวอักษร (`letterPick`)
    pub fn set_fast_push(&mut self, push: bool) {
        self.typing.set_fast_push(push);
    }

    /// ย้อน 1 ระดับ (B / D-pad ←) — `true` = ใช้เป็น cancel แล้ว
    pub fn fast_cancel(&mut self) -> bool {
        self.typing.fast_cancel()
    }

    /// Fast mode: กลุ่มที่ไฮไลต์/ล็อกอยู่
    pub fn fast_group(&self) -> Option<Dir8> {
        self.typing.fast_group()
    }

    /// Fast mode: ตัวอักษรที่ไฮไลต์อยู่
    pub fn fast_sector(&self) -> Option<Dir8> {
        self.typing.fast_sector()
    }

    /// Fast mode: R3 สองครั้ง = ลูกศร + Shift (เลือกข้อความหลายตัว)
    pub fn fast_selecting(&self) -> bool {
        self.fast && self.fast_select
    }

    /// Fast mode: RS กำลังชี้ทิศอยู่ (ลูกศรทำงาน) — HUD ใช้วงล้อโชว์ตอนดัน Analog ขวา
    pub fn fast_arrows_active(&self) -> bool {
        self.fast && self.caret_dir.is_some()
    }

    /// Fast mode: จำนวนครั้งที่พิมพ์ซ้ำด้วยการค้าง LS
    pub fn fast_repeat_count(&self) -> u32 {
        self.typing.fast_repeat_count()
    }

    /// Type mode: R3 เปิดโหมดลูกศรอยู่ (RS = ลูกศร)
    pub fn caret_active(&self) -> bool {
        self.caret
    }

    /// โหมดลูกศรกำลังเลือกข้อความ (ลูกศร + Shift)
    pub fn caret_selecting(&self) -> bool {
        self.caret && self.caret_select
    }

    /// Mouse mode: ล็อกปุ่มซ้ายค้างอยู่ (RB แตะ 2 ครั้ง)
    pub fn sticky_drag(&self) -> bool {
        self.sticky_drag
    }

    /// ช่องของ quick bar ตามทิศ RS (interaction model §5)
    pub fn quickbar_slot(dir: Dir8) -> QuickAction {
        match dir {
            Dir8::N => QuickAction::Copy,
            Dir8::NE => QuickAction::Paste,
            Dir8::E => QuickAction::Cut,
            Dir8::SE => QuickAction::Undo,
            Dir8::S => QuickAction::Redo,
            Dir8::SW => QuickAction::SelectAll,
            Dir8::W => QuickAction::Save,
            Dir8::NW => QuickAction::SwitchWindow,
        }
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
        self.reset_mode_state();
    }

    /// ล้างสถานะชั่วคราวของ mode (ชั้น 2 / pointer / scroll / ปุ่มหน้าค้าง)
    fn reset_mode_state(&mut self) {
        self.layer2_lock = false;
        self.layer2_held = false;
        self.rb_hold_ms = 0;
        self.rt_hold_ms = 0;
        self.layer2_used = false;
        self.prev_layer = 1;
        self.pointer.reset();
        self.scroll.reset();
        self.scroll_diag_fired = false;
        self.a_window_ms = 0;
        self.b_hold_ms = 0;
        self.b_hold_fired = false;
        self.y_hold_ms = 0;
        self.y_pending = false;
        self.y_hold_fired = false;
        self.a_repeat = None;
        self.a_hold_ms = 0;
        self.a_repeat_ms = 0;
        self.b_repeat_ms = 0;
        self.b_repeat_ok = false;
        self.caret = false;
        self.caret_select = false;
        self.fast_select = false;
        self.fast_queued = None;
        self.r3_window_ms = 0;
        self.caret_dir = None;
        self.caret_ms = 0;
        self.caret_fired = false;
        self.caret_sector.reset();
        self.typing.fast_reset();
        self.lb_hold_ms = 0;
        self.lb_hold_armed = false;
        self.sticky_drag = false;
        self.rb_tap_window_ms = 0;
        self.rb_sticky_released = false;
    }

    pub fn lang(&self) -> Lang {
        self.typing.lang()
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.typing.set_lang(lang);
    }

    /// ตั้งภาษาปัจจุบันจาก slot (built-in หรือ layout ที่สร้างจาก Windows)
    pub fn activate_language(&mut self, slot: &LangSlot) {
        self.typing
            .set_profile(slot.lang, &slot.code, &slot.label, slot.keymap.clone());
    }

    /// override ของผู้ใช้ 1 ช่อง (config `languages[].overrides`) — `None` = ช่องว่าง
    pub fn set_keymap_override(
        &mut self,
        code: &str,
        layer: u8,
        group: u8,
        sector: u8,
        key: Option<Key>,
    ) {
        self.typing.set_override(code, layer, group, sector, key);
    }

    pub fn clear_keymap_overrides(&mut self) {
        self.typing.clear_overrides();
    }

    /// คีย์ที่จะพิมพ์จากช่องนี้ (ตามภาษาปัจจุบัน + layer ปัจจุบัน) — HUD ใช้ทำ preview
    pub fn preview_key(&self, group: Dir8, sector: Dir8) -> Option<Key> {
        self.typing.key_at(group, sector)
    }

    /// ภาษาอื่นที่เปิดใช้ (ไม่รวม EN) — เรียงตามลำดับใน config
    pub fn languages(&self) -> &[LangSlot] {
        &self.languages
    }

    /// รหัสภาษาที่ใช้อยู่ (`en`, `th`, `ru`, …) — HUD ใช้แสดง pill
    pub fn lang_code(&self) -> &str {
        self.typing.code()
    }

    /// ชื่อภาษาที่ใช้อยู่ — HUD ใช้วงภาษา
    pub fn lang_label(&self) -> &str {
        self.typing.label()
    }

    /// ตั้งรายการภาษาที่สอง สูงสุด [`MAX_LANGUAGES`]
    ///
    /// ถ้าภาษาที่ใช้อยู่ถูกปิด → กลับ EN (interaction model §7)
    pub fn set_languages(&mut self, languages: Vec<LangSlot>) {
        self.languages = languages.into_iter().take(MAX_LANGUAGES).collect();
        let current = self.typing.code().to_string();
        if current != "en" && !self.languages.iter().any(|slot| slot.code == current) {
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
        if self.fast {
            self.fast_group()
        } else {
            self.typing.group()
        }
    }

    /// ช่องที่ RS เลือกอยู่ (สำหรับ HUD preview/วงนอก) — Fast mode ใช้ไฮไลต์ของ LS
    pub fn typing_sector(&self) -> Option<Dir8> {
        if self.fast {
            self.fast_sector()
        } else {
            self.typing.sector()
        }
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
            dpad_up: pad.buttons.dpad_up && !self.prev_dpad_up,
            dpad_down: pad.buttons.dpad_down && !self.prev_dpad_down,
            dpad_right: pad.buttons.dpad_right && !self.prev_dpad_right,
            a: pad.buttons.a && !self.prev_a,
            b: pad.buttons.b && !self.prev_b,
            x: pad.buttons.x && !self.prev_x,
            y: pad.buttons.y && !self.prev_y,
            lt: pad.buttons.lt && !self.prev_lt,
            rb: pad.buttons.rb && !self.prev_rb,
            rt: pad.buttons.rt && !self.prev_rt,
            rs_click: pad.buttons.rs_click && !self.prev_rs_click,
            ls_click: pad.buttons.ls_click && !self.prev_ls_click,
            view: pad.buttons.view && !self.prev_view,
        };

        let action = self.step(pad, dt_ms, edges);

        self.prev_dpad_left = pad.buttons.dpad_left;
        self.prev_dpad_up = pad.buttons.dpad_up;
        self.prev_dpad_down = pad.buttons.dpad_down;
        self.prev_dpad_right = pad.buttons.dpad_right;
        self.prev_a = pad.buttons.a;
        self.prev_b = pad.buttons.b;
        self.prev_x = pad.buttons.x;
        self.prev_y = pad.buttons.y;
        self.prev_lt = pad.buttons.lt;
        self.prev_rb = pad.buttons.rb;
        self.prev_rt = pad.buttons.rt;
        self.prev_rs_click = pad.buttons.rs_click;
        self.prev_ls_click = pad.buttons.ls_click;
        self.prev_view = pad.buttons.view;
        action
    }

    fn step(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        // ตัวจับเวลา double-tap ของ A เดินตลอดเวลา (แม้อยู่ในวง/Disabled)
        self.a_window_ms = self.a_window_ms.saturating_sub(dt_ms);
        // หน้าต่าง R3 สองครั้ง (เปิดโหมดเลือกข้อความ)
        self.r3_window_ms = self.r3_window_ms.saturating_sub(dt_ms);
        if self.mode != Mode::Type {
            self.r3_window_ms = 0;
        }

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

        // 3) D-pad ← : Fast mode จัดการใน `tick_fast` (ซ่อน = วงคำสั่ง · เมนูเปิด = ย้อน)
        // 3) D-pad ← เปิด/ปิดวงคำสั่ง — พฤติกรรมเดิมของ Mode ปกติ (owner 2026-11-17)
        if edges.dpad_left {
            self.toggle_cmd_wheel(pad);
            return None;
        }

        // 4) quick bar: ปล่อย LB = ปิด · ดัน LS = กลับไปพิมพ์แบบ Ctrl+ตัวอักษร
        if self.wheel == Some(WheelKind::QuickBar) {
            let ls_armed = pad.ls.magnitude() >= self.cfg.gesture.ls_activate;
            if !pad.buttons.lb || ls_armed {
                self.close_wheel();
            }
        }

        // Fast mode: ปุ่ม View ก็เปิดวงคำสั่งได้ (ทางเลือกนอกจาก D-pad ←)
        if self.fast && self.mode == Mode::Type && self.wheel.is_none() && edges.view {
            self.toggle_cmd_wheel(pad);
            return None;
        }

        // 5) วงเปิดอยู่ = A/B + RS เท่านั้น (LS ไม่มีผล · X/Y ไม่มีผล — วงชนะ)
        if self.wheel.is_some() {
            return self.tick_wheel(pad, dt_ms, edges);
        }

        // 6) Modifier: LT แตะ = สลับ Shift lock (ADR-13) — เฉพาะ Type
        if edges.lt && self.mode == Mode::Type {
            self.shift_lock = !self.shift_lock;
        }

        // 7) ชั้น 2 (RB/RT) เฉพาะ Type: แตะ = ล็อก · ค้าง = ชั่วคราว
        if self.mode == Mode::Type {
            // เช็คก่อนประมวลผลปล่อยปุ่ม: ถ้ามี gesture ค้างอยู่ = ค้างเพื่อพิมพ์ ไม่ใช่แตะ
            if self.layer2_held && self.typing.state() != GestureState::Idle {
                self.layer2_used = true;
            }
            self.tick_layer2(pad, dt_ms, edges.rb, edges.rt);
            // เปลี่ยนชั้นกลาง gesture = ยกเลิก (กันพิมพ์ผิดชุด)
            let layer = self.layer();
            if layer != self.prev_layer {
                if self.typing.state() != GestureState::Idle {
                    self.typing.cancel();
                }
                // Fast mode: ไม่ล้างช่องที่ไฮไลต์ตอนเปลี่ยนชั้น — A อ่านคีย์ตามชั้นปัจจุบันตอนกดอยู่แล้ว
                // (เดิมล้างช่อง → ปล่อย LS แล้วแตะ RB/RT → A ไม่พิมพ์จนกว่าจะดัน LS ใหม่)
            }
            self.prev_layer = layer;
        } else {
            self.layer2_held = false;
            self.prev_layer = 1;
        }

        // 8) Fast mode: LS = เลือกกลุ่ม→ตัวอักษร · A = พิมพ์ · RS = ลูกศร
        //    (ปุ่มหน้า A/B/X/Y ถูกจัดการใน `tick_fast` — A ห้ามกลายเป็น Space)
        if self.fast && self.mode == Mode::Type {
            return self.tick_fast(pad, dt_ms, edges);
        }

        // 8) ปุ่มหน้า A/B/X/Y + double-tap / hold (spec §4)
        //    caret mode (ADR-05): A = ยืนยัน · B = ยกเลิก — ห้ามส่ง Space/Backspace
        //    ทับข้อความที่เลือก; X/Y ยังทำงานปกติ
        if self.caret && (edges.a || edges.b) {
            self.caret = false;
            self.caret_select = false;
            self.caret_dir = None;
            self.caret_ms = 0;
            self.caret_fired = false;
            self.caret_sector.reset();
            self.a_window_ms = 0;
            if edges.b {
                // กัน hold-B ยิง Backspace/Ctrl+Backspace ต่อหลังออกจากโหมด
                self.b_hold_ms = 0;
                self.b_hold_fired = true;
                self.b_repeat_ok = false;
            }
            return None;
        }
        if let Some(action) = self.tick_face(pad, dt_ms, edges) {
            return Some(action);
        }
        if self.face_busy(pad) {
            return None;
        }

        // 9) gesture บนแกนตาม mode
        match self.mode {
            Mode::Type => {
                // R3: เปิด/ปิดโหมดลูกศร · แตะ 2 ครั้ง = เลือกข้อความ
                if edges.rs_click {
                    self.toggle_caret();
                    return None;
                }
                // LB ค้าง 150 ms (ขณะยังไม่พิมพ์) = quick bar
                self.tick_quickbar_hold(pad, dt_ms);
                if self.wheel.is_some() {
                    return None;
                }
                if self.caret {
                    return self.tick_caret(pad, dt_ms);
                }
                self.typing.set_layer(self.layer());
                self.typing
                    .tick(pad, dt_ms)
                    .map(|action| self.with_shift_lock(action))
            }
            Mode::Mouse => self.tick_mouse(pad, dt_ms, edges),
            Mode::Nav => self.tick_nav(pad, dt_ms, edges),
            Mode::Disabled => None,
        }
    }

    /* ---------------- ชั้น 2 (RB/RT) ---------------- */

    fn tick_layer2(&mut self, pad: &PadState, dt_ms: u32, rb_edge: bool, rt_edge: bool) {
        if rb_edge {
            self.rb_hold_ms = 0;
            self.layer2_used = false;
        }
        if rt_edge {
            self.rt_hold_ms = 0;
            self.layer2_used = false;
        }
        if pad.buttons.rb {
            self.rb_hold_ms = self.rb_hold_ms.saturating_add(dt_ms);
        }
        if pad.buttons.rt {
            self.rt_hold_ms = self.rt_hold_ms.saturating_add(dt_ms);
        }
        self.layer2_held = pad.buttons.rb || pad.buttons.rt;

        // แตะเร็วโดยไม่ได้พิมพ์ระหว่างกด = สลับล็อก
        if self.prev_rb && !pad.buttons.rb && self.rb_hold_ms < self.cfg.double_tap_ms {
            if !self.layer2_used {
                self.layer2_lock = !self.layer2_lock;
            }
            self.rb_hold_ms = 0;
        }
        if self.prev_rt && !pad.buttons.rt && self.rt_hold_ms < self.cfg.double_tap_ms {
            if !self.layer2_used {
                self.layer2_lock = !self.layer2_lock;
            }
            self.rt_hold_ms = 0;
        }
    }

    /* ---------------- ปุ่มหน้า (double-tap / hold) ---------------- */

    fn face_busy(&self, pad: &PadState) -> bool {
        pad.buttons.b || (pad.buttons.y && self.y_pending)
    }

    /// จัดการปุ่มหน้า — คืน action เดียวต่อเฟรม (ท่อ injector รับทีละ action)
    ///
    /// - A: Space · แตะเร็ว 2 ครั้ง = Enter (Space แรกออกไปแล้ว — ดู ADR-21)
    ///   **ค้าง 400 ms = ส่ง Space ซ้ำทุก 90 ms** (owner 2026-11-17)
    /// - B: Backspace · **ค้าง 400 ms = ส่ง Backspace ซ้ำทุก 80 ms**
    /// - X: Enter
    /// - Y: Tab on release · ค้าง 400 ms = Esc
    fn tick_face(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        let typing_active = self.typing.state() != GestureState::Idle;
        if edges.a || edges.b || edges.x || edges.y {
            if typing_active {
                self.typing.cancel();
            }
            self.layer2_used = true;
        }

        // B hold (ตัวจับเวลาเดินแม้ไม่มี edge)
        if edges.b {
            self.b_hold_ms = 0;
            self.b_hold_fired = false;
            self.b_repeat_ms = 0;
        } else if pad.buttons.b {
            self.b_hold_ms = self.b_hold_ms.saturating_add(dt_ms);
        } else {
            self.b_hold_ms = 0;
            self.b_hold_fired = false;
            self.b_repeat_ms = 0;
        }

        // Y hold — Tab ตอนปล่อย, Esc เมื่อค้างครบ 400 ms
        if edges.y {
            self.y_pending = true;
            self.y_hold_ms = 0;
            self.y_hold_fired = false;
        } else if pad.buttons.y && self.y_pending {
            self.y_hold_ms = self.y_hold_ms.saturating_add(dt_ms);
        }

        // A double-tap ก่อน (สำคัญกว่าปุ่มอื่นในเฟรมเดียวกัน)
        if edges.a {
            if self.a_window_ms > 0 {
                self.a_window_ms = 0;
                self.a_repeat = None;
                return Some(self.named_key(NamedKey::Enter, false));
            }
            self.a_window_ms = self.cfg.double_tap_ms;
            let press = KeyPress {
                key: Key::Named(NamedKey::Space),
                shift: false,
                ctrl: false,
                alt: false,
            };
            self.a_repeat = Some(press);
            self.a_hold_ms = 0;
            self.a_repeat_ms = 0;
            return Some(Action::Key(press));
        }

        // A ค้าง = Space ซ้ำ (เหมือนกด Spacebar ค้าง)
        if pad.buttons.a {
            if let Some(press) = self.a_repeat {
                self.a_hold_ms = self.a_hold_ms.saturating_add(dt_ms);
                if self.a_hold_ms >= self.cfg.face_hold_ms {
                    self.a_repeat_ms = self.a_repeat_ms.saturating_add(dt_ms);
                    if self.a_repeat_ms >= self.cfg.gesture.repeat_rate_ms {
                        self.a_repeat_ms = 0;
                        return Some(Action::Key(press));
                    }
                }
            }
        } else {
            self.a_repeat = None;
            self.a_hold_ms = 0;
            self.a_repeat_ms = 0;
        }

        if edges.x {
            return Some(self.named_key(NamedKey::Enter, false));
        }

        if edges.b {
            // B ขณะมี gesture = ยกเลิกอย่างเดียว ไม่ลบ (spec §2)
            if typing_active {
                return None;
            }
            self.b_repeat_ok = true;
            return Some(self.named_key(NamedKey::Backspace, pad.buttons.lb));
        }

        // B ค้าง = Backspace ซ้ำ (ไม่ใช่ Ctrl+Backspace แล้ว — owner 2026-11-17)
        if pad.buttons.b && !self.b_hold_fired && self.b_hold_ms >= self.cfg.face_hold_ms {
            self.b_hold_fired = true;
            self.b_repeat_ms = 0;
            return Some(self.named_key(NamedKey::Backspace, false));
        }
        if pad.buttons.b && self.b_hold_fired && self.b_repeat_ok {
            self.b_repeat_ms = self.b_repeat_ms.saturating_add(dt_ms);
            if self.b_repeat_ms >= self.cfg.gesture.repeat_rate_ms {
                self.b_repeat_ms = 0;
                return Some(self.named_key(NamedKey::Backspace, false));
            }
        } else if !pad.buttons.b {
            self.b_repeat_ok = false;
        }

        if self.y_pending {
            if pad.buttons.y {
                if !self.y_hold_fired && self.y_hold_ms >= self.cfg.face_hold_ms {
                    self.y_hold_fired = true;
                    self.y_pending = false;
                    return Some(self.named_key(NamedKey::Escape, false));
                }
            } else {
                self.y_pending = false;
                return Some(self.named_key(NamedKey::Tab, pad.buttons.lb));
            }
        }

        None
    }

    fn named_key(&self, key: NamedKey, ctrl: bool) -> Action {
        Action::Key(KeyPress {
            key: Key::Named(key),
            // ADR-13: LT เป็น toggle ไม่ใช่ hold → ไม่ส่ง Shift ของ OS ออกไป
            shift: false,
            ctrl,
            alt: false,
        })
    }

    /* ---------------- caret nav (R3) ---------------- */

    /// R3 แตะ = เปิด/ปิดลูกศร · แตะ 2 ครั้งภายในหน้าต่าง = เลือกข้อความ
    ///
    /// ครั้งแรกเปิดลูกศรเสมอ (interaction model §3) — ถ้าแตะครั้งที่สองเร็ว ๆ
    /// จะเปลี่ยนเป็นโหมดเลือกแทนการปิด (กดครั้งเดียวหลังพ้นหน้าต่างจึงปิด)
    fn toggle_caret(&mut self) {
        self.typing.cancel();
        if self.r3_window_ms > 0 {
            self.r3_window_ms = 0;
            self.caret = true;
            self.caret_select = true;
        } else {
            self.r3_window_ms = self.cfg.double_tap_ms;
            if self.caret {
                self.caret = false;
            } else {
                self.caret = true;
            }
            self.caret_select = false;
        }
        self.caret_sector.reset();
        self.caret_dir = None;
        self.caret_ms = 0;
        self.caret_fired = false;
    }

    /// RS = ลูกศร · ดันครั้งแรกยิงทันที · ค้าง repeat ตาม 350/90 ms
    fn tick_caret(&mut self, pad: &PadState, dt_ms: u32) -> Option<Action> {
        self.tick_arrows(pad, dt_ms, self.caret_select)
    }

    /// Fast mode: RS = ลูกศร (มี repeat) — R3 สองครั้ง = ลูกศร + Shift
    fn tick_fast_arrows(&mut self, pad: &PadState, dt_ms: u32) -> Option<Action> {
        self.tick_arrows(pad, dt_ms, self.fast_select)
    }

    fn tick_arrows(&mut self, pad: &PadState, dt_ms: u32, select: bool) -> Option<Action> {
        let cfg = self.cfg.gesture;
        let rm = pad.rs.magnitude();

        if rm >= cfg.rs_activate {
            let dir = self.caret_sector.update(pad.rs, cfg.rs_activate);
            if dir != self.caret_dir {
                self.caret_dir = dir;
                self.caret_ms = 0;
                self.caret_fired = false;
                return dir.map(|_| self.arrow(pad.rs, select));
            }
            self.caret_ms = self.caret_ms.saturating_add(dt_ms);
            let interval = if self.caret_fired {
                cfg.repeat_rate_ms
            } else {
                cfg.repeat_delay_ms
            };
            if self.caret_ms >= interval {
                self.caret_ms = 0;
                self.caret_fired = true;
                return self.caret_dir.map(|_| self.arrow(pad.rs, select));
            }
            return None;
        }

        if rm < cfg.rs_release {
            self.caret_sector.reset();
            self.caret_dir = None;
            self.caret_ms = 0;
            self.caret_fired = false;
        }
        None
    }

    /// ทิศ RS → ลูกศร (แกนที่เบี่ยงมากกว่าเป็นตัวตัดสิน · จอย y ขึ้น = Up)
    fn arrow(&self, rs: Vec2, select: bool) -> Action {
        let key = if rs.x.abs() >= rs.y.abs() {
            if rs.x >= 0.0 {
                NamedKey::Right
            } else {
                NamedKey::Left
            }
        } else if rs.y >= 0.0 {
            NamedKey::Up
        } else {
            NamedKey::Down
        };
        Action::Key(KeyPress {
            key: Key::Named(key),
            shift: select,
            ctrl: false,
            alt: false,
        })
    }

    /* ---------------- Fast mode (ADR-32 / ADR-33) ---------------- */

    /// 1 เฟรมของ Fast mode
    ///
    /// - **push-through**: LS เอียง = หมวด · ดันสุด = โซนตัวอักษร · A = พิมพ์
    /// - **D-pad ↑/↓** = ±1 · **→** = เข้าลึก · **←** = ย้อน (ตอนซ่อน = เมนูโหมด)
    /// - **B** = ย้อน 1 ระดับ · ไม่มีเมนู + LS ศูนย์ = Backspace
    /// - L3 = ล้างช่อง · X = Enter · Y = Tab/Esc
    /// - **RS = ลูกศร** · R3 สองครั้ง = highlight (Shift+ลูกศร)
    fn tick_fast(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        let queued = self.fast_queued.take();

        // R3: แตะ 2 ครั้ง = สลับโหมดเลือกข้อความ
        if edges.rs_click {
            if self.r3_window_ms > 0 {
                self.r3_window_ms = 0;
                self.fast_select = !self.fast_select;
            } else {
                self.r3_window_ms = self.cfg.double_tap_ms;
            }
            self.caret_sector.reset();
            self.caret_dir = None;
            self.caret_ms = 0;
            self.caret_fired = false;
        }

        // L3 = ล้างช่องที่เลือก
        if edges.ls_click {
            self.typing.fast_unlock();
        }

        // ห้าม return กลางทางในฟังก์ชันนี้: ทุกเฟรมต้องไปถึง `typing.tick_fast` เสมอ
        // ไม่งั้น A edge (เกิดเฟรมเดียว) หายถ้าชนกับลูกศร RS / B / X / Y (v0.3.5)
        // action อื่นเก็บไว้ใน `others` แล้วส่งตามลำดับ (A ก่อน · ที่เหลือเข้าคิวเฟรมถัดไป)
        let mut others: [Option<Action>; 3] = [None; 3];

        // B = ย้อน 1 ระดับ (ไม่ลบ) · ไม่มีเมนู = Backspace (+ ค้าง = ซ้ำ)
        if edges.b {
            self.b_hold_ms = 0;
            self.b_repeat_ms = 0;
            self.b_hold_fired = true;
            if !self.typing.fast_ui_hidden() && self.typing.fast_cancel() {
                self.b_repeat_ok = false;
            } else {
                self.b_repeat_ok = true;
                others[0] = Some(self.named_key(NamedKey::Backspace, pad.buttons.lb));
            }
        } else if pad.buttons.b {
            self.b_hold_ms = self.b_hold_ms.saturating_add(dt_ms);
            if self.b_repeat_ok
                && (self.typing.fast_zone() == FastZone::Hidden || self.typing.fast_ui_hidden())
                && self.b_hold_ms >= self.cfg.face_hold_ms
            {
                self.b_repeat_ms = self.b_repeat_ms.saturating_add(dt_ms);
                if self.b_repeat_ms >= self.cfg.gesture.repeat_rate_ms {
                    self.b_repeat_ms = 0;
                    others[0] = Some(self.named_key(NamedKey::Backspace, false));
                }
            }
        } else {
            self.b_hold_ms = 0;
            self.b_repeat_ms = 0;
            self.b_repeat_ok = false;
        }

        // X/Y = Enter / Tab-Esc (ใช้ machinery เดิม โดยไม่ให้ A/B หลุดเข้าไป)
        if edges.x || edges.y {
            let face = Edges {
                x: edges.x,
                y: edges.y,
                ..Edges::default()
            };
            others[1] = self.tick_face(pad, dt_ms, face);
        }

        // RS = ลูกศร (รับ Shift เมื่ออยู่ในโหมดเลือกข้อความ)
        others[2] = self.tick_fast_arrows(pad, dt_ms);

        // Fast mode ใช้ LS อย่างเดียว (owner 2026-11-17) · D-pad = เมนูโหมด (วงคำสั่ง)
        // ทางหนี `letterPick = dpad` (0.2.5): ↑ = ถอย · ↓/→ = ช่องถัดไป
        let classic = !self.typing.fast_push();
        let nav = if !classic {
            None
        } else if edges.dpad_up {
            Some(FastNav::Prev)
        } else if edges.dpad_down || edges.dpad_right {
            Some(FastNav::Next)
        } else {
            None
        };

        self.typing.set_layer(self.layer());
        let typed = self
            .typing
            .tick_fast(
                pad,
                dt_ms,
                FastInput {
                    accept: edges.a,
                    nav,
                    enter: false,
                },
            )
            .map(|action| self.with_shift_lock(action));

        // A ตอนไม่มีช่องค้างเลือก (ปล่อย LS แล้ว / พิมพ์ไปแล้ว) = Space · ค้าง = Space ซ้ำ
        let idle_a = edges.a
            && typed.is_none()
            && self.typing.fast_sector().is_none()
            && self.typing.fast_radius() < FAST_CENTER_OUT;
        let typed = if idle_a {
            Some(self.named_key(NamedKey::Space, false))
        } else {
            typed
        };

        // A ค้าง = พิมพ์ตัวเดิมซ้ำ (owner 2026-11-17 — เหมือนกดคีย์ค้าง)
        // A ที่ถูกเก็บไว้ (lockout/latch) แล้วพิมพ์เฟรมหลัง ๆ ก็เริ่ม repeat ได้ถ้ายังกดค้างอยู่
        let mut a_action = typed;
        if let Some(Action::Key(press)) = typed {
            if pad.buttons.a {
                self.a_repeat = Some(press);
                self.a_hold_ms = 0;
                self.a_repeat_ms = 0;
            }
        } else if pad.buttons.a {
            if let Some(press) = self.a_repeat {
                self.a_hold_ms = self.a_hold_ms.saturating_add(dt_ms);
                if self.a_hold_ms >= self.cfg.face_hold_ms {
                    self.a_repeat_ms = self.a_repeat_ms.saturating_add(dt_ms);
                    if self.a_repeat_ms >= self.cfg.gesture.repeat_rate_ms {
                        self.a_repeat_ms = 0;
                        a_action = Some(self.with_shift_lock(Action::Key(press)));
                    }
                }
            }
        } else {
            self.a_repeat = None;
            self.a_hold_ms = 0;
            self.a_repeat_ms = 0;
        }

        // ส่ง 1 action ต่อเฟรม: A → ที่ค้างคิว → B/X/Y → ลูกศร · ที่เหลือเข้าคิวเฟรมถัดไป
        let mut out = a_action;
        let mut next = None;
        for action in std::iter::once(queued).chain(others).flatten() {
            if out.is_none() {
                out = Some(action);
            } else if next.is_none() {
                next = Some(action);
            }
        }
        self.fast_queued = next;
        out
    }

    /* ---------------- quick bar (LB) ---------------- */
    /// LB ค้าง 150 ms = เปิด quick bar (แตะสั้นไม่เปิด — ยังใช้เป็น Ctrl ได้)
    ///
    /// เปิดครั้งเดียวต่อการกด · ปล่อย LB แล้วเริ่มนับใหม่ · ดัน LS ก่อนครบเวลา = ยกเลิกการเปิด
    fn tick_quickbar_hold(&mut self, pad: &PadState, dt_ms: u32) {
        if !pad.buttons.lb {
            self.lb_hold_ms = 0;
            self.lb_hold_armed = true;
            return;
        }
        if !self.lb_hold_armed {
            return;
        }
        let dragging_ls = pad.ls.magnitude() >= self.cfg.gesture.ls_activate;
        if dragging_ls || self.caret || self.typing.state() != GestureState::Idle {
            self.lb_hold_ms = 0;
            self.lb_hold_armed = false;
            return;
        }
        self.lb_hold_ms = self.lb_hold_ms.saturating_add(dt_ms);
        if self.lb_hold_ms >= self.cfg.quickbar_hold_ms {
            self.open_quickbar(pad);
            self.lb_hold_armed = false;
        }
    }

    fn open_quickbar(&mut self, pad: &PadState) {
        self.close_wheel();
        self.wheel = Some(WheelKind::QuickBar);
        self.typing.fast_reset();
        self.wheel_armed = pad.rs.magnitude() < self.cfg.gesture.rs_release;
    }

    /// ยืนยันช่อง quick bar → คีย์ลัดจริง (Ctrl+ตัวอักษร / Alt+Tab)
    fn commit_quickbar(&mut self, index: u8) -> Option<Action> {
        let dir = Dir8::from_index(index as usize);
        self.close_wheel();
        self.typing.cancel();

        let (key, ctrl, alt) = match Self::quickbar_slot(dir) {
            QuickAction::Copy => (Key::Char('c'), true, false),
            QuickAction::Paste => (Key::Char('v'), true, false),
            QuickAction::Cut => (Key::Char('x'), true, false),
            QuickAction::Undo => (Key::Char('z'), true, false),
            QuickAction::Redo => (Key::Char('y'), true, false),
            QuickAction::SelectAll => (Key::Char('a'), true, false),
            QuickAction::Save => (Key::Char('s'), true, false),
            QuickAction::SwitchWindow => (Key::Named(NamedKey::Tab), false, true),
        };
        Some(Action::Key(KeyPress {
            key,
            shift: false,
            ctrl,
            alt,
        }))
    }

    /* ---------------- Mouse mode ---------------- */

    fn tick_mouse(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        self.rb_tap_window_ms = self.rb_tap_window_ms.saturating_sub(dt_ms);

        // RB = คลิกซ้าย (กดค้าง = ลาก) · แตะ 2 ครั้ง = ล็อกปุ่มซ้ายค้าง (sticky drag)
        // · RT = คลิกขวา
        if edges.rb {
            if self.sticky_drag {
                // แตะ RB อีกครั้ง = ปล่อยปุ่มซ้าย (จบการลาก)
                self.sticky_drag = false;
                self.rb_tap_window_ms = 0;
                self.rb_sticky_released = true;
                return Some(Action::MouseButton {
                    button: MouseButton::Left,
                    down: false,
                });
            }
            return Some(Action::MouseButton {
                button: MouseButton::Left,
                down: true,
            });
        }
        if self.prev_rb && !pad.buttons.rb {
            if self.rb_sticky_released {
                // การกดครั้งนี้เพิ่งปล่อย sticky drag ไปแล้ว — ไม่ปล่อยซ้ำ
                self.rb_sticky_released = false;
                return None;
            }
            if self.rb_tap_window_ms > 0 {
                // แตะครั้งที่ 2 เร็ว ๆ = ล็อกปุ่มซ้ายค้างไว้ (ไม่ปล่อย)
                self.sticky_drag = true;
                self.rb_tap_window_ms = 0;
                return None;
            }
            self.rb_tap_window_ms = self.cfg.double_tap_ms;
            return Some(Action::MouseButton {
                button: MouseButton::Left,
                down: false,
            });
        }
        if edges.rt {
            return Some(Action::MouseButton {
                button: MouseButton::Right,
                down: true,
            });
        }
        if self.prev_rt && !pad.buttons.rt {
            return Some(Action::MouseButton {
                button: MouseButton::Right,
                down: false,
            });
        }

        // กด RS = pointer กลับกลางจอหลัก
        if edges.rs_click {
            self.pointer.reset();
            self.scroll.reset();
            return Some(Action::RecenterPointer);
        }

        // RS → pointer (LT = precision · LB = turbo)
        let (dx, dy) = self
            .pointer
            .tick(pad.rs, dt_ms, pad.buttons.lt, pad.buttons.lb);
        if dx != 0 || dy != 0 {
            return Some(Action::MouseMove { dx, dy });
        }

        // LS → scroll (↖ = Ctrl+Home · ↘ = Ctrl+End)
        if pad.ls.x.abs() > 0.45 && pad.ls.y.abs() > 0.45 {
            return self.scroll_diagonal(pad.ls);
        }
        self.scroll_diag_fired = false;
        self.scroll
            .tick(pad.ls, dt_ms, pad.buttons.lt)
            .map(|(axis, delta)| match axis {
                ScrollAxis::Vertical => Action::MouseWheel { delta },
                ScrollAxis::Horizontal => Action::MouseHWheel { delta },
            })
    }

    /// LS ↖ = ไปบนสุด · ↘ = ไปล่างสุด (ครั้งเดียวต่อการดัน)
    fn scroll_diagonal(&mut self, ls: Vec2) -> Option<Action> {
        let diagonal = ls.x.abs() > 0.45 && ls.y.abs() > 0.45;
        if !diagonal {
            self.scroll_diag_fired = false;
            return None;
        }
        if self.scroll_diag_fired {
            return None;
        }
        self.scroll_diag_fired = true;
        let key = if ls.x < 0.0 && ls.y > 0.0 {
            NamedKey::Home
        } else if ls.x > 0.0 && ls.y < 0.0 {
            NamedKey::End
        } else {
            return None;
        };
        Some(self.named_key(key, true))
    }

    /* ---------------- Nav mode (pivot) ---------------- */

    fn tick_nav(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        if edges.rs_click {
            return Some(Action::ResetPivot);
        }
        let (dx, dy) = self.pivot.tick(pad.ls, pad.rs, dt_ms, pad.buttons.lt);
        if dx.abs() < f32::EPSILON && dy.abs() < f32::EPSILON {
            return None;
        }
        Some(Action::MovePivot { dx, dy })
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
            alt: press.alt,
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
        self.reset_mode_state();
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
        self.reset_mode_state();
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
        self.caret = false;
        self.caret_select = false;
        self.wheel = Some(WheelKind::Command);
        // เปิดวงขณะ RS ยังดันค้าง = ยังไม่ armed (spec §3.1 บรรทัดสุดท้าย)
        self.wheel_armed = pad.rs.magnitude() < self.cfg.gesture.rs_release;
        self.typing.cancel();
        // Fast mode หยุดเดินขณะวงเปิด → ล้างโซน/ช่อง ไม่ให้ UI เก่าโผล่กลับตอนปิดวง
        self.typing.fast_reset();
    }

    fn open_lang_wheel(&mut self, pad: &PadState) {
        self.close_wheel();
        if self.languages.is_empty() {
            return;
        }
        self.wheel = Some(WheelKind::Language);
        self.typing.fast_reset();
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

    fn tick_wheel(&mut self, pad: &PadState, dt_ms: u32, edges: Edges) -> Option<Action> {
        let cfg = self.cfg.gesture;
        let kind = self.wheel?;
        let (a, b) = (edges.a, edges.b);

        // B = ยกเลิกทุกวง (spec §3.1/§3.2)
        if b {
            self.close_wheel();
            return None;
        }

        // D-pad ← = ย้อน/ปิดวง (ทางเลือกนอกจาก B)
        if edges.dpad_left {
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
                WheelKind::QuickBar => sector.map(|dir| dir.index() as u8),
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
            WheelKind::QuickBar => self.commit_quickbar(index),
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
                    let slot = self.languages[0].clone();
                    self.apply_mode(Mode::Type);
                    self.typing
                        .set_profile(slot.lang, &slot.code, &slot.label, slot.keymap);
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
        let slot = self.languages.get(index as usize)?.clone();
        self.close_wheel();
        self.typing.cancel();
        self.apply_mode(Mode::Type);
        self.typing
            .set_profile(slot.lang, &slot.code, &slot.label, slot.keymap);
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
        self.reset_mode_state();
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
                alt: false,
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

        // Mouse mode: LS/RS ไม่พิมพ์ (ขยับ pointer/wheel ได้)
        for _ in 0..30 {
            let action = e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
            assert!(!matches!(action, Some(Action::Key(_))), "ต้องไม่พิมพ์");
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
            let action = e.tick(&p((0.0, 0.9), (0.0, 0.9)), D);
            assert!(!matches!(action, Some(Action::Key(_))));
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
                alt: false,
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
        // Mouse mode: RB = left click → ระหว่างทางมีคลิกก่อนถึง 1.5 s
        let mut disabled = false;
        for _ in 0..20 {
            if e.tick(&both, 100) == Some(Action::SetMode(Mode::Disabled)) {
                disabled = true;
                break;
            }
        }
        assert!(disabled, "LB+RB 1.5 s ต้องเข้า Disabled");
        assert_eq!(e.mode(), Mode::Disabled);

        let action = hold(&mut e, p((0.0, 0.0), (0.0, 0.0)), 3000, 100).expect("ต้องกลับมา");
        assert_eq!(action, Action::SetMode(Mode::Mouse));
        assert_eq!(e.mode(), Mode::Mouse);
    }

    /* ---------------- slice 2: Shift toggle (LT, ADR-13) ---------------- */

    /// แตะปุ่ม 1 ครั้ง (rising edge) แล้วปล่อย — คืน action จากเฟรมกดหรือเฟรมปล่อย
    ///
    /// ปุ่ม Y ส่ง Tab ตอนปล่อย (เพื่อแยก hold = Esc) ปุ่มอื่นส่งตอนกด
    fn tap(e: &mut RuntimeEngine, f: fn(&mut Buttons)) -> Option<Action> {
        let pressed = with(p((0.0, 0.0), (0.0, 0.0)), f);
        let down = e.tick(&pressed, D);
        let up = e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        down.or(up)
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
            alt: false,
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
                alt: false,
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

    /* ---------------- slice 3: Mouse mode ---------------- */

    /// เปิดวงคำสั่งแล้วเลือกโหมดด้วย RS (ทิศ `dir`) + ปล่อย RS
    fn switch_mode(e: &mut RuntimeEngine, dir: (f32, f32)) {
        open_cmd(e, (0.0, 0.0));
        e.tick(&p((0.0, 0.0), dir), D);
        let action = release_rs(e).expect("ต้องยืนยันโหมด");
        assert!(matches!(action, Action::Command(_)));
    }

    fn to_mouse(e: &mut RuntimeEngine) {
        switch_mode(e, (0.9, 0.0));
        assert_eq!(e.mode(), Mode::Mouse);
    }

    fn to_nav(e: &mut RuntimeEngine) {
        switch_mode(e, (0.9, -0.9));
        assert_eq!(e.mode(), Mode::Nav);
    }

    fn button_press(button: fn(&mut Buttons)) -> PadState {
        with(p((0.0, 0.0), (0.0, 0.0)), button)
    }

    #[test]
    fn mouse_mode_moves_the_pointer_with_rs() {
        let mut e = engine();
        to_mouse(&mut e);
        let action = e.tick(&p((0.0, 0.0), (1.0, 0.0)), 100).expect("ต้องขยับ");
        match action {
            Action::MouseMove { dx, dy } => {
                assert!(dx > 0, "ได้ dx={dx}");
                assert_eq!(dy, 0);
            }
            other => panic!("ต้องเป็น MouseMove ไม่ใช่ {other:?}"),
        }
        // ดันขึ้น → dy ติดลบ (หน้าจอ y ลง)
        let action = e.tick(&p((0.0, 0.0), (0.0, 1.0)), 100).expect("ต้องขยับ");
        assert!(matches!(action, Action::MouseMove { dy, .. } if dy < 0));
    }

    #[test]
    fn mouse_rb_and_rt_click_down_and_up() {
        let mut e = engine();
        to_mouse(&mut e);

        // RB = คลิกซ้าย
        assert_eq!(
            e.tick(&button_press(|b| b.rb = true), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: true
            })
        );
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: false
            })
        );

        // RT = คลิกขวา
        assert_eq!(
            e.tick(&button_press(|b| b.rt = true), D),
            Some(Action::MouseButton {
                button: MouseButton::Right,
                down: true
            })
        );
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D),
            Some(Action::MouseButton {
                button: MouseButton::Right,
                down: false
            })
        );
    }

    #[test]
    fn mouse_rs_click_asks_the_app_to_recenter() {
        let mut e = engine();
        to_mouse(&mut e);
        assert_eq!(
            e.tick(&button_press(|b| b.rs_click = true), D),
            Some(Action::RecenterPointer)
        );
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
    }

    #[test]
    fn mouse_ls_scrolls_vertically_horizontally_and_to_the_ends() {
        let mut e = engine();
        to_mouse(&mut e);

        assert_eq!(
            e.tick(&p((0.0, 0.9), (0.0, 0.0)), 40),
            Some(Action::MouseWheel { delta: 120 })
        );
        assert_eq!(
            e.tick(&p((0.9, 0.0), (0.0, 0.0)), 40),
            Some(Action::MouseHWheel { delta: 120 })
        );
        // ↖ = Ctrl+Home
        assert_eq!(
            e.tick(&p((-0.9, 0.9), (0.0, 0.0)), 40),
            Some(Action::Key(KeyPress {
                key: Key::Named(NamedKey::Home),
                shift: false,
                ctrl: true,
                alt: false,
            }))
        );
        // ค้างทิศเดิม = ไม่ยิงซ้ำ
        assert_eq!(e.tick(&p((-0.9, 0.9), (0.0, 0.0)), 40), None);
    }

    /* ---------------- slice 3: Nav mode ---------------- */

    #[test]
    fn nav_mode_moves_the_pivot_with_ls_and_rs() {
        let mut e = engine();
        to_nav(&mut e);

        let action = e.tick(&p((0.0, 1.0), (0.0, 0.0)), 100).expect("ต้องขยับ");
        assert!(
            matches!(action, Action::MovePivot { dx, dy } if dx == 0.0 && dy <= -80.0),
            "ได้ {action:?}"
        );

        let action = e.tick(&p((0.0, 0.0), (1.0, 0.0)), 100).expect("ต้องขยับ");
        assert!(
            matches!(action, Action::MovePivot { dx, dy } if dx >= 20.0 && dy == 0.0),
            "ได้ {action:?}"
        );
    }

    #[test]
    fn nav_rs_click_resets_the_pivot() {
        let mut e = engine();
        to_nav(&mut e);
        assert_eq!(
            e.tick(&button_press(|b| b.rs_click = true), D),
            Some(Action::ResetPivot)
        );
    }

    /* ---------------- slice 3: ชั้น 2 (RB/RT) ---------------- */

    fn tap_rb(e: &mut RuntimeEngine) {
        e.tick(&button_press(|b| b.rb = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
    }

    #[test]
    fn tapping_rb_locks_layer_two_and_tapping_again_unlocks_it() {
        let mut e = engine();
        assert_eq!(e.layer(), 1);
        tap_rb(&mut e);
        assert_eq!(e.layer(), 2);
        assert!(e.layer2_locked());
        // ล็อกอยู่ = ปล่อย RB แล้วยังพิมพ์ชั้น 2 ได้
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('A')));
        tap_rb(&mut e);
        assert_eq!(e.layer(), 1);
        assert!(!e.layer2_locked());
        // กลับมาพิมพ์ตัวเล็กตามเดิม
        assert_eq!(type_dir(&mut e, LS_N, LS_N), key(Key::Char('a')));
    }

    #[test]
    fn holding_rb_is_temporary_and_does_not_lock() {
        let mut e = engine();
        let rb = |ls: (f32, f32), rs: (f32, f32)| with(p(ls, rs), |b| b.rb = true);

        e.tick(&rb((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&rb((0.0, 0.0), (0.0, 0.0)), 300);
        assert_eq!(e.layer(), 2);
        assert!(!e.layer2_locked());

        // พิมพ์ระหว่างกดค้าง (gesture เกิดขึ้น = ไม่นับเป็นการแตะ)
        e.tick(&rb(LS_N, (0.0, 0.0)), D);
        e.tick(&rb(LS_N, LS_N), D);
        e.tick(&rb(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&rb(LS_N, (0.0, 0.0)), 64), key(Key::Char('A')));

        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.layer(), 1);
        assert!(!e.layer2_locked(), "ค้างเพื่อพิมพ์ต้องไม่ล็อกชั้น 2");
    }

    #[test]
    fn quick_layer_two_gesture_does_not_lock_the_layer() {
        let mut e = engine();
        let rb = |ls: (f32, f32), rs: (f32, f32)| with(p(ls, rs), |b| b.rb = true);
        e.tick(&rb((0.0, 0.0), (0.0, 0.0)), D); // กด RB
        e.tick(&rb(LS_N, (0.0, 0.0)), D); // arm
        e.tick(&rb(LS_N, LS_N), D); // select ภายใน < 280 ms
        e.tick(&rb(LS_N, (0.0, 0.0)), D); // committing
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D); // ปล่อย RB
        assert!(!e.layer2_locked(), "การพิมพ์ระหว่างกดต้องไม่ล็อกชั้น 2");
        assert_eq!(e.layer(), 1);
    }

    #[test]
    fn changing_layer_mid_gesture_cancels_the_selection() {
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        e.tick(&p(LS_N, LS_N), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        e.tick(&button_press(|b| b.rb = true), D);
        assert_eq!(e.gesture_state(), GestureState::Idle);
        assert_eq!(e.layer(), 2);
        // ปล่อย RS แล้วรอ dwell → ต้องไม่พิมพ์
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 100), None);
    }

    /* ---------------- slice 3: double-tap / hold ---------------- */

    #[test]
    fn a_double_tap_sends_enter_instead_of_a_second_space() {
        let mut e = engine();
        let press = button_press(|b| b.a = true);

        assert_eq!(e.tick(&press, D), key(Key::Named(NamedKey::Space)));
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.tick(&press, D), key(Key::Named(NamedKey::Enter)));

        // ครั้งที่ 3 หลังจากหน้าต่างหมด = Space อีกครั้ง
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), 300); // หน้าต่าง double-tap หมด
        assert_eq!(e.tick(&press, D), key(Key::Named(NamedKey::Space)));
    }

    #[test]
    fn holding_a_repeats_space() {
        let mut e = engine();
        let held = button_press(|b| b.a = true);
        assert_eq!(e.tick(&held, D), key(Key::Named(NamedKey::Space)));
        assert_eq!(e.tick(&held, 300), None, "ยังไม่ครบ 400 ms");
        assert_eq!(
            e.tick(&held, 100),
            key(Key::Named(NamedKey::Space)),
            "ค้าง A = Space ซ้ำ"
        );
        assert_eq!(e.tick(&held, 90), key(Key::Named(NamedKey::Space)));
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
    }

    #[test]
    fn b_hold_repeats_backspace() {
        let mut e = engine();
        let held = button_press(|b| b.b = true);
        assert_eq!(e.tick(&held, D), key(Key::Named(NamedKey::Backspace)));
        assert_eq!(e.tick(&held, 300), None, "ยังไม่ครบ 400 ms");
        // ครบ 400 ms = เริ่มลบซ้ำ (ไม่ใช่ Ctrl+Backspace)
        assert_eq!(
            e.tick(&held, 100),
            key(Key::Named(NamedKey::Backspace)),
            "ค้าง B = Backspace ซ้ำ"
        );
        // แล้วทุก 90 ms
        assert_eq!(e.tick(&held, 90), key(Key::Named(NamedKey::Backspace)));
        assert_eq!(e.tick(&held, 90), key(Key::Named(NamedKey::Backspace)));
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
    }

    #[test]
    fn y_is_tab_on_tap_and_escape_on_hold() {
        // แตะ = Tab (ส่งตอนปล่อย)
        let mut e = engine();
        let held = button_press(|b| b.y = true);
        assert_eq!(e.tick(&held, D), None);
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D),
            key(Key::Named(NamedKey::Tab))
        );

        // ค้าง 400 ms = Esc (ไม่ส่ง Tab ตอนปล่อย)
        let mut e = engine();
        assert_eq!(e.tick(&held, D), None);
        assert_eq!(e.tick(&held, 390), None);
        assert_eq!(e.tick(&held, 10), key(Key::Named(NamedKey::Escape)));
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
    }

    /* ---------------- slice 4: quick bar (LB) ---------------- */

    fn lb_pad(rs: (f32, f32)) -> PadState {
        with(p((0.0, 0.0), rs), |b| b.lb = true)
    }

    /// กด LB แล้วค้าง 150 ms (ADR-23) — quick bar จะเปิด
    fn open_quickbar(e: &mut RuntimeEngine, rs: (f32, f32)) {
        assert_eq!(e.tick(&lb_pad(rs), D), None);
        assert_eq!(e.tick(&lb_pad(rs), 150), None);
        assert_eq!(e.wheel(), Some(WheelKind::QuickBar));
    }

    fn shortcut(ch: char) -> Option<Action> {
        Some(Action::Key(KeyPress {
            key: Key::Char(ch),
            shift: false,
            ctrl: true,
            alt: false,
        }))
    }

    #[test]
    fn holding_lb_opens_quickbar_and_releasing_rs_runs_the_shortcut() {
        let mut e = engine();
        open_quickbar(&mut e, (0.0, 0.0));
        assert!(e.wheel_armed());

        // RS ↑ = Copy
        e.tick(&lb_pad((0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
        assert_eq!(e.wheel_highlight_dir(), Some(Dir8::N));

        // ปล่อย RS (ยังถือ LB) → ยืนยันหลัง dwell
        assert_eq!(e.tick(&lb_pad((0.0, 0.0)), 30), None);
        assert_eq!(e.tick(&lb_pad((0.0, 0.0)), 40), shortcut('c'));
        assert_eq!(e.wheel(), None);

        // LB ยังค้าง = ไม่เปิดซ้ำ (edge เท่านั้น)
        for _ in 0..10 {
            assert_eq!(e.tick(&lb_pad((0.0, 0.0)), D), None);
        }
        assert_eq!(e.wheel(), None);
    }

    #[test]
    fn quickbar_slot_map_matches_the_interaction_model() {
        let cases = [
            (Dir8::N, 'c', false, QuickAction::Copy),
            (Dir8::NE, 'v', false, QuickAction::Paste),
            (Dir8::E, 'x', false, QuickAction::Cut),
            (Dir8::SE, 'z', false, QuickAction::Undo),
            (Dir8::S, 'y', false, QuickAction::Redo),
            (Dir8::SW, 'a', false, QuickAction::SelectAll),
            (Dir8::W, 's', false, QuickAction::Save),
            (Dir8::NW, '\t', true, QuickAction::SwitchWindow),
        ];
        for (dir, ch, alt, quick) in cases {
            assert_eq!(RuntimeEngine::quickbar_slot(dir), quick, "{dir:?}");
            let mut e = engine();
            open_quickbar(&mut e, (0.0, 0.0));
            let angle = dir.center_deg().to_radians();
            let rs = Vec2::new(angle.sin(), angle.cos());
            e.tick(&lb_pad((rs.x, rs.y)), D);
            assert_eq!(e.wheel_index(), Some(dir.index() as u8), "{dir:?}");
            e.tick(&lb_pad((0.0, 0.0)), 30);
            let action = e.tick(&lb_pad((0.0, 0.0)), 40).expect("ต้องทำช่อง");
            let expected_key = if alt {
                Key::Named(NamedKey::Tab)
            } else {
                Key::Char(ch)
            };
            assert_eq!(
                action,
                Action::Key(KeyPress {
                    key: expected_key,
                    shift: false,
                    ctrl: !alt,
                    alt,
                }),
                "{dir:?}"
            );
        }
    }

    #[test]
    fn releasing_lb_closes_quickbar_without_running_anything() {
        let mut e = engine();
        open_quickbar(&mut e, (0.0, 0.0));
        e.tick(&lb_pad((0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), D), None);
        assert_eq!(e.wheel(), None);
        // ปล่อย RS ต่อ = ไม่มีคีย์หลุด
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), 100), None);
    }

    #[test]
    fn pushing_ls_in_quickbar_switches_to_ctrl_typing() {
        let mut e = engine();
        open_quickbar(&mut e, (0.0, 0.0));
        assert_eq!(e.wheel(), Some(WheelKind::QuickBar));

        // ดัน LS ขณะ LB ค้าง = ปิด quickbar แล้วพิมพ์แบบ Ctrl+ตัวอักษร
        let lb_ls = |ls: (f32, f32), rs: (f32, f32)| with(p(ls, rs), |b| b.lb = true);
        assert_eq!(e.tick(&lb_ls(LS_N, (0.0, 0.0)), D), None);
        assert_eq!(e.wheel(), None);
        e.tick(&lb_ls(LS_N, LS_N), D);
        e.tick(&lb_ls(LS_N, (0.0, 0.0)), D);
        assert_eq!(
            e.tick(&lb_ls(LS_N, (0.0, 0.0)), 64),
            Some(Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: true,
                alt: false,
            }))
        );
    }

    #[test]
    fn quickbar_opened_while_rs_is_held_waits_for_center() {
        let mut e = engine();
        e.tick(&lb_pad((0.0, 0.0)), D);
        e.tick(&lb_pad((0.0, 0.9)), 150);
        assert_eq!(e.wheel(), Some(WheelKind::QuickBar));
        assert!(!e.wheel_armed());
        for _ in 0..20 {
            assert_eq!(e.tick(&lb_pad((0.0, 0.9)), D), None);
        }
        assert_eq!(e.wheel_index(), None);
        e.tick(&lb_pad((0.0, 0.0)), D);
        assert!(e.wheel_armed());
        e.tick(&lb_pad((0.0, 0.9)), D);
        assert_eq!(e.wheel_index(), Some(Dir8::N.index() as u8));
    }

    #[test]
    fn tapping_lb_shorter_than_150ms_never_opens_quickbar() {
        let mut e = engine();
        for _ in 0..20 {
            assert_eq!(e.tick(&lb_pad((0.0, 0.0)), 5), None);
        }
        assert_eq!(e.wheel(), None);
        // ปล่อย LB แล้วเริ่มนับใหม่
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        open_quickbar(&mut e, (0.0, 0.0));
    }

    /* ---------------- slice 4: caret nav (R3) ---------------- */

    fn tap_r3(e: &mut RuntimeEngine) {
        e.tick(&button_press(|b| b.rs_click = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
    }

    fn arrow(key: NamedKey, shift: bool) -> Option<Action> {
        Some(Action::Key(KeyPress {
            key: Key::Named(key),
            shift,
            ctrl: false,
            alt: false,
        }))
    }

    #[test]
    fn r3_toggles_caret_mode_and_rs_sends_arrows_with_repeat() {
        let mut e = engine();
        assert!(!e.caret_active());
        tap_r3(&mut e);
        assert!(e.caret_active());

        // ดันครั้งแรก = ยิงทันที
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.9)), D),
            arrow(NamedKey::Up, false)
        );
        // ยังไม่ครบ 350 ms
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), 300), None);
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.9)), 50),
            arrow(NamedKey::Up, false)
        );
        // 90 ms ต่อครั้ง
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), 89), None);
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.9)), 1),
            arrow(NamedKey::Up, false)
        );

        // เปลี่ยนทิศ = ยิงทันที
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.9, 0.0)), D),
            arrow(NamedKey::Right, false)
        );
        // ปล่อย RS = หยุด
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), 4), None);
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), 500), None);

        // กด R3 อีกครั้ง (พ้นหน้าต่าง double tap) = ปิด
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), 300);
        tap_r3(&mut e);
        assert!(!e.caret_active());
        // ปิดแล้ว RS ไม่ส่งลูกศร
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.9)), D), None);
    }

    #[test]
    fn r3_double_tap_enables_selection_and_arrows_carry_shift() {
        let mut e = engine();
        tap_r3(&mut e); // เปิดลูกศร
        assert!(e.caret_active());
        assert!(!e.caret_selecting());

        // แตะครั้งที่ 2 ภายในหน้าต่าง = เลือกข้อความ (ยังเปิดลูกศร)
        tap_r3(&mut e);
        assert!(e.caret_active());
        assert!(e.caret_selecting());

        assert_eq!(
            e.tick(&p((0.0, 0.0), (-0.9, 0.0)), D),
            arrow(NamedKey::Left, true)
        );

        // แตะเดี่ยวหลังพ้นหน้าต่าง = ปิดทั้งลูกศรและการเลือก
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), 300);
        tap_r3(&mut e);
        assert!(!e.caret_active());
        assert!(!e.caret_selecting());
    }

    #[test]
    fn r3_while_selecting_a_character_cancels_typing() {
        let mut e = engine();
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        e.tick(&p(LS_N, LS_N), D);
        assert_eq!(e.gesture_state(), GestureState::CharSelected);

        tap_r3(&mut e);
        assert!(e.caret_active());
        assert_eq!(e.gesture_state(), GestureState::Idle);

        // ปล่อย RS + รอ dwell → ต้องไม่พิมพ์ตัวอักษร
        e.tick(&p(LS_N, (0.0, 0.0)), D);
        assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), 200), None);
    }

    #[test]
    fn caret_mode_ignores_ls_and_uses_only_rs_for_arrows() {
        let mut e = engine();
        tap_r3(&mut e);
        // ดัน LS อย่างเดียว = ไม่มีอะไร (ไม่พิมพ์)
        for _ in 0..20 {
            assert_eq!(e.tick(&p(LS_N, (0.0, 0.0)), D), None);
        }
        assert_eq!(e.gesture_state(), GestureState::Idle);
        // LS + RS = ลูกศรตาม RS
        assert_eq!(
            e.tick(&p(LS_N, (0.0, -0.9)), D),
            arrow(NamedKey::Down, false)
        );
    }

    #[test]
    fn a_and_b_leave_caret_mode_without_typing_over_the_selection() {
        // A = ยืนยัน (ออกจากโหมด) ไม่ส่ง Space
        let mut e = engine();
        tap_r3(&mut e);
        assert!(e.caret_active());
        assert_eq!(e.tick(&button_press(|b| b.a = true), D), None);
        assert!(!e.caret_active());
        // หลังจากนั้น A กลับเป็น Space ตามปกติ
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(
            tap(&mut e, |b| b.a = true),
            key(Key::Named(NamedKey::Space))
        );

        // B = ยกเลิก (ออกจากโหมด) ไม่ส่ง Backspace
        let mut e = engine();
        tap_r3(&mut e);
        assert_eq!(e.tick(&button_press(|b| b.b = true), D), None);
        assert!(!e.caret_active());
        // ค้าง B ต่อ = ต้องไม่ยิง Ctrl+Backspace หลังออกจากโหมด
        assert_eq!(e.tick(&button_press(|b| b.b = true), 500), None);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(
            tap(&mut e, |b| b.b = true),
            key(Key::Named(NamedKey::Backspace))
        );
    }

    /* ---------------- slice 4: sticky drag (Mouse) ---------------- */

    #[test]
    fn double_tapping_rb_locks_the_left_button_until_the_next_tap() {
        let mut e = engine();
        to_mouse(&mut e);

        // แตะครั้งที่ 1 = คลิกปกติ
        assert_eq!(
            e.tick(&button_press(|b| b.rb = true), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: true
            })
        );
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: false
            })
        );
        assert!(!e.sticky_drag());

        // แตะครั้งที่ 2 เร็ว ๆ = กดค้างไว้ (ไม่ปล่อย)
        assert_eq!(
            e.tick(&button_press(|b| b.rb = true), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: true
            })
        );
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
        assert!(e.sticky_drag());

        // ลากได้: RS ยังขยับ pointer ระหว่างล็อก
        assert!(matches!(
            e.tick(&p((0.0, 0.0), (1.0, 0.0)), 100),
            Some(Action::MouseMove { .. })
        ));

        // แตะอีกครั้ง = ปล่อยปุ่มซ้าย
        assert_eq!(
            e.tick(&button_press(|b| b.rb = true), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: false
            })
        );
        assert!(!e.sticky_drag());
        // ยกนิ้วแล้วต้องไม่ปล่อยซ้ำ
        assert_eq!(e.tick(&p((0.0, 0.0), (0.0, 0.0)), D), None);
    }

    #[test]
    fn slow_second_rb_tap_is_a_normal_click() {
        let mut e = engine();
        to_mouse(&mut e);
        e.tick(&button_press(|b| b.rb = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        // ปล่อยทิ้งไว้จนหน้าต่าง double tap หมด
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), 300);
        e.tick(&button_press(|b| b.rb = true), D);
        assert_eq!(
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D),
            Some(Action::MouseButton {
                button: MouseButton::Left,
                down: false
            })
        );
        assert!(!e.sticky_drag());
    }

    #[test]
    fn sticky_drag_is_cleared_when_leaving_mouse_mode() {
        let mut e = engine();
        to_mouse(&mut e);
        e.tick(&button_press(|b| b.rb = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&button_press(|b| b.rb = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(e.sticky_drag());

        e.set_mode(Mode::Type);
        assert!(!e.sticky_drag());
    }

    /* ---------------- Fast mode (ADR-33 push-through) ---------------- */

    /// ดัน LS ต่อเนื่อง (คืน action สุดท้ายที่เกิด — ปกติ None)
    fn push(e: &mut RuntimeEngine, ls: (f32, f32), ms: u32) -> Option<Action> {
        let mut out = None;
        let mut t = 0;
        while t < ms {
            if let Some(action) = e.tick(&p(ls, (0.0, 0.0)), D) {
                out = Some(action);
            }
            t += D;
        }
        out
    }

    #[test]
    fn fast_mode_push_through_latches_then_a_types() {
        let mut e = engine();
        e.set_fast(true);
        assert!(e.fast());

        // โยก = ทิศนั้นเป็นหมวด → โซนตัวอักษรทันที
        push(&mut e, (0.0, 0.5), 80);
        assert_eq!(e.fast_zone(), FastZone::Letter);
        assert_eq!(e.fast_group(), Some(Dir8::N));

        push(&mut e, (0.0, 0.9), 240);
        assert_eq!(e.fast_zone(), FastZone::Letter);
        assert_eq!(e.fast_sector(), Some(Dir8::N));

        let mut press = p((0.0, 0.9), (0.0, 0.0));
        press.buttons.a = true;
        let action = e.tick(&press, D);
        assert!(
            matches!(
                action,
                Some(Action::Key(KeyPress {
                    key: Key::Char(_),
                    ..
                }))
            ),
            "A ในโซนตัวอักษรต้องพิมพ์: {action:?}"
        );

        // ปล่อย LS = ล้างหมวด แล้วโยกทิศใหม่ = หมวดใหม่
        push(&mut e, (0.0, 0.0), 60);
        assert_eq!(e.fast_zone(), FastZone::Hidden);
        assert_eq!(e.fast_group(), None);
        push(&mut e, (0.9, 0.0), 120);
        assert_eq!(e.fast_group(), Some(Dir8::E));
    }

    #[test]
    fn fast_mode_b_walks_back_a_level_before_backspace() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        assert_eq!(e.fast_zone(), FastZone::Letter);

        let mut hold_b = p((0.0, 0.9), (0.0, 0.0));
        hold_b.buttons.b = true;

        // B ครั้งแรก = กลับไปโซนหมวด (ไม่ลบ)
        assert_eq!(e.tick(&hold_b, D), None);
        assert_eq!(e.fast_zone(), FastZone::Category);
        assert_eq!(e.fast_sector(), None);

        // B อีกครั้ง = ซ่อน UI
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        assert_eq!(e.tick(&hold_b, D), None);
        assert_eq!(e.fast_zone(), FastZone::Hidden);

        // ไม่มีเมนูแล้ว (และ LS ศูนย์) = Backspace
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        let action = e.tick(&button_press(|b| b.b = true), D);
        assert!(
            matches!(
                action,
                Some(Action::Key(KeyPress {
                    key: Key::Named(NamedKey::Backspace),
                    ..
                }))
            ),
            "B ที่ไม่มีเมนูต้องเป็น Backspace: {action:?}"
        );
    }

    #[test]
    fn fast_mode_a_while_pushing_then_a_after_release_is_space() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        assert!(is_char(press_a(&mut e, (0.0, 0.9))), "A ตอนดันอยู่ = ตัวอักษร");
        release_ls(&mut e);
        assert_eq!(
            press_a(&mut e, (0.0, 0.0)),
            key(Key::Named(NamedKey::Space)),
            "ปล่อย LS แล้วกด A ห้ามซ้ำตัวเดิม"
        );
    }

    #[test]
    fn fast_mode_holding_a_at_rest_repeats_space() {
        let mut e = engine();
        e.set_fast(true);
        let space = key(Key::Named(NamedKey::Space));
        let mut held = p((0.0, 0.0), (0.0, 0.0));
        held.buttons.a = true;
        assert_eq!(e.tick(&held, D), space);
        let mut repeats = 0;
        for _ in 0..8 {
            if e.tick(&held, 90) == space {
                repeats += 1;
            }
        }
        assert!(repeats >= 3, "ค้าง A ต้อง Space ซ้ำ: {repeats}");
    }

    #[test]
    fn fast_mode_holding_a_repeats_the_character() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        assert_eq!(e.fast_zone(), FastZone::Letter);

        let mut held = p((0.0, 0.9), (0.0, 0.0));
        held.buttons.a = true;
        let first = e.tick(&held, D);
        assert!(
            matches!(first, Some(Action::Key(KeyPress { key: Key::Char(_), .. }))),
            "A ต้องพิมพ์ตัวอักษร: {first:?}"
        );

        let mut repeats = 0;
        for _ in 0..8 {
            if matches!(
                e.tick(&held, 90),
                Some(Action::Key(KeyPress { key: Key::Char(_), .. }))
            ) {
                repeats += 1;
            }
        }
        assert!(repeats >= 2, "ค้าง A ต้องพิมพ์ซ้ำ (ได้ {repeats})");
    }

    #[test]
    fn fast_mode_dpad_behaves_like_the_normal_mode() {
        let mut e = engine();
        e.set_fast(true);

        // D-pad ← = วงคำสั่ง (เหมือน Mode ปกติ)
        e.tick(&button_press(|b| b.dpad_left = true), D);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&button_press(|b| b.dpad_left = true), D);
        assert_eq!(e.wheel(), None);

        // เปิดเมนู Fast ด้วย LS — D-pad ↑/↓/→ ต้องไม่เปลี่ยนโซน/ช่อง
        push(&mut e, (0.0, 0.5), 80);
        assert_eq!(e.fast_zone(), FastZone::Letter);
        let group = e.fast_group();
        e.tick(&button_press(|b| b.dpad_down = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&button_press(|b| b.dpad_right = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.fast_zone(), FastZone::Letter);
        assert_eq!(e.fast_group(), group, "D-pad ต้องไม่เลือกเมนูแทน LS");

        // ← ยังเปิดวงคำสั่งได้แม้เมนู Fast เปิดอยู่ (พฤติกรรม Mode ปกติ)
        e.tick(&button_press(|b| b.dpad_left = true), D);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
    }

    #[test]
    fn fast_mode_dpad_fallback_uses_right_arrow_to_step() {
        let mut e = engine();
        e.set_fast(true);
        e.set_fast_push(false);

        e.tick(&p((0.9, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.fast_group(), Some(Dir8::E));
        assert_eq!(e.fast_sector(), Some(Dir8::E));

        // โหมด 0.2.5: D-pad → = ช่องถัดไป
        e.tick(&button_press(|b| b.dpad_right = true), D);
        assert_eq!(e.fast_sector(), Some(Dir8::SE));
    }

    #[test]
    fn fast_mode_release_keeps_a_printing_and_b_backspaces() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        assert_eq!(e.fast_zone(), FastZone::Letter);

        // ปล่อย LS 300 ms → UI ซ่อน แต่ A ยังพิมพ์ตัวที่เลือก
        for _ in 0..75 {
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        }
        assert!(e.fast_ui_hidden());
        assert!(
            matches!(
                e.tick(&button_press(|b| b.a = true), D),
                Some(Action::Key(KeyPress {
                    key: Key::Char(_),
                    ..
                }))
            ),
            "A ต้องพิมพ์ได้แม้ปล่อยจอยแล้ว"
        );

        // ปล่อย A แล้วกด B → เมนูซ่อนอยู่ = Backspace ทันที
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(
            matches!(
                e.tick(&button_press(|b| b.b = true), D),
                Some(Action::Key(KeyPress {
                    key: Key::Named(NamedKey::Backspace),
                    ..
                }))
            ),
            "B ตอนเมนูซ่อนต้องเป็น Backspace"
        );
    }

    /// Analog จริงเด้งกลับศูนย์ผ่านช่วงกลางหลายเฟรม (ไม่ใช่ 0.9 → 0 ในเฟรมเดียว)
    fn release_ls(e: &mut RuntimeEngine) {
        for ls in [
            (0.0, 0.8),
            (0.1, 0.6),
            (0.2, 0.4),
            (0.2, 0.15),
            (0.1, 0.05),
            (0.0, 0.0),
        ] {
            e.tick(&p(ls, (0.0, 0.0)), D);
        }
    }

    fn is_char(action: Option<Action>) -> bool {
        matches!(
            action,
            Some(Action::Key(KeyPress {
                key: Key::Char(_),
                ..
            }))
        )
    }

    fn press_a(e: &mut RuntimeEngine, ls: (f32, f32)) -> Option<Action> {
        let action = e.tick(&with(p(ls, (0.0, 0.0)), |b| b.a = true), D);
        e.tick(&p(ls, (0.0, 0.0)), D); // ปล่อย A
        action
    }

    fn select_mode_on(e: &mut RuntimeEngine) {
        e.tick(&button_press(|b| b.rs_click = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        e.tick(&button_press(|b| b.rs_click = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(e.fast_selecting());
    }

    #[test]
    fn fast_mode_a_types_right_after_releasing_ls_and_after_120ms() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        release_ls(&mut e);
        assert_eq!(e.fast_zone(), FastZone::Letter, "เด้งกลับห้ามตกโซน");
        assert_eq!(e.fast_sector(), Some(Dir8::N), "เด้งกลับห้ามล้างช่อง");

        // ทันทีหลังปล่อย (UI ยังไม่ซ่อน)
        assert!(is_char(press_a(&mut e, (0.0, 0.0))), "A ทันทีหลังปล่อย LS ต้องพิมพ์");

        // หลัง 120 ms (UI ซ่อนแล้ว) — A = Space ทุกครั้ง (ไม่พิมพ์ตัวเดิมซ้ำ)
        push(&mut e, (0.0, 0.0), 200);
        assert!(e.fast_ui_hidden());
        for _ in 0..3 {
            assert_eq!(
                press_a(&mut e, (0.0, 0.0)),
                key(Key::Named(NamedKey::Space)),
                "A หลัง UI ซ่อนต้องเป็น Space"
            );
            push(&mut e, (0.0, 0.0), 200);
        }
    }

    #[test]
    fn fast_mode_first_a_right_after_latch_is_not_swallowed() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.5), 80);
        let mut guard = 0;
        while e.fast_zone() != FastZone::Letter && guard < 100 {
            e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
            guard += 1;
        }
        assert_eq!(e.fast_zone(), FastZone::Letter);

        // แตะ A ทันทีที่ latch (ติด lockout 100 ms) แล้วปล่อย — ต้องพิมพ์ 1 ครั้งหลัง lockout
        assert!(press_a(&mut e, (0.0, 0.9)).is_none());
        let typed = (0..40)
            .filter(|_| is_char(e.tick(&p((0.0, 0.9), (0.0, 0.0)), D)))
            .count();
        assert_eq!(typed, 1, "A ตอน latch ต้องพิมพ์ครั้งเดียว ไม่หาย ไม่ซ้ำ");
    }

    #[test]
    fn fast_mode_a_pressed_while_latching_is_not_swallowed() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.5), 80);
        e.tick(&p((0.0, 0.9), (0.0, 0.0)), D);
        // กด A ระหว่างรอ latch 60 ms
        assert!(press_a(&mut e, (0.0, 0.9)).is_none());
        let typed = (0..60)
            .filter(|_| is_char(e.tick(&p((0.0, 0.9), (0.0, 0.0)), D)))
            .count();
        assert_eq!(e.fast_zone(), FastZone::Letter);
        assert_eq!(typed, 1);
    }

    #[test]
    fn fast_mode_a_edge_reaches_typing_even_when_a_right_stick_arrow_fires() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);

        // เฟรมเดียวกัน: RS ดันครั้งแรก (ลูกศรยิง) + A edge → A ต้องพิมพ์ ลูกศรเข้าคิวเฟรมถัดไป
        let mut both = p((0.0, 0.9), (0.9, 0.0));
        both.buttons.a = true;
        assert!(is_char(e.tick(&both, D)), "A ห้ามถูกลูกศร RS กลืน");
        let queued = e.tick(&p((0.0, 0.9), (0.9, 0.0)), D);
        assert!(
            matches!(
                queued,
                Some(Action::Key(KeyPress {
                    key: Key::Named(NamedKey::Right),
                    ..
                }))
            ),
            "ลูกศรที่ชนกับ A ต้องไม่หาย: {queued:?}"
        );
    }

    #[test]
    fn fast_mode_a_types_in_select_mode_before_and_after_releasing_ls() {
        let mut e = engine();
        e.set_fast(true);
        select_mode_on(&mut e);

        push(&mut e, (0.0, 0.9), 300);
        assert!(is_char(press_a(&mut e, (0.0, 0.9))), "Select mode ต้องไม่บล็อก A");
        release_ls(&mut e);
        assert_eq!(press_a(&mut e, (0.0, 0.0)), key(Key::Named(NamedKey::Space)));
        push(&mut e, (0.0, 0.0), 200);
        assert!(e.fast_selecting(), "ปล่อย LS ไม่ปิด Select mode");
        assert_eq!(press_a(&mut e, (0.0, 0.0)), key(Key::Named(NamedKey::Space)));
    }

    #[test]
    fn fast_mode_a_survives_a_layer_change_after_releasing_ls() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        release_ls(&mut e);
        e.tick(&button_press(|b| b.rb = true), D); // แตะ RB = ชั้น 2
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert_eq!(e.fast_sector(), Some(Dir8::N), "เปลี่ยนชั้นห้ามล้างช่อง");
        assert!(
            press_a(&mut e, (0.0, 0.0)).is_some(),
            "A หลังเปลี่ยนชั้นต้องพิมพ์ (คีย์ตามชั้นปัจจุบัน)"
        );
    }

    #[test]
    fn fast_mode_ui_state_returns_to_idle_after_releasing_ls_and_rs() {
        for select in [false, true] {
            let mut e = engine();
            e.set_fast(true);
            if select {
                select_mode_on(&mut e);
            }

            // LS: เอียงเต็ม = มี UI → ปล่อย 120 ms = UI ซ่อน (A ยังพิมพ์ได้)
            push(&mut e, (0.0, 0.9), 300);
            assert!(!e.fast_ui_hidden());
            release_ls(&mut e);
            push(&mut e, (0.0, 0.0), 120);
            assert!(e.fast_ui_hidden(), "select={select}: ปล่อย LS 120 ms ต้องซ่อน UI");

            // RS: ดัน = ลูกศรทำงาน (วงโชว์) → ปล่อย = หายทันทีเฟรมถัดไป
            e.tick(&p((0.0, 0.0), (0.9, 0.0)), D);
            assert!(e.fast_arrows_active());
            e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
            assert!(!e.fast_arrows_active(), "select={select}: ปล่อย RS ต้องหาย");
        }
    }

    #[test]
    fn fast_mode_ls_resting_in_the_old_dead_band_still_hides_the_ui() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        // หยุดค้างที่ 0.25 (เดิมอยู่ใน dead band 0.20–0.30 → ไม่เคยนับว่าปล่อย)
        push(&mut e, (0.0, 0.25), 200);
        assert!(e.fast_ui_hidden());
    }

    #[test]
    fn fast_mode_opening_the_command_wheel_clears_the_fast_ui() {
        let mut e = engine();
        e.set_fast(true);
        push(&mut e, (0.0, 0.9), 300);
        assert_eq!(e.fast_zone(), FastZone::Letter);
        e.tick(&button_press(|b| b.dpad_left = true), D);
        assert_eq!(e.wheel(), Some(WheelKind::Command));
        assert_eq!(e.fast_zone(), FastZone::Hidden);
        assert_eq!(e.fast_sector(), None);
    }

    #[test]
    fn fast_mode_r3_twice_turns_the_right_stick_into_shift_arrows() {
        let mut e = engine();
        e.set_fast(true);

        // R3 สองครั้งติดกัน = โหมดเลือกข้อความ
        e.tick(&button_press(|b| b.rs_click = true), D);
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);
        assert!(!e.fast_selecting());
        e.tick(&button_press(|b| b.rs_click = true), D);
        assert!(e.fast_selecting());
        e.tick(&p((0.0, 0.0), (0.0, 0.0)), D);

        // RS → = Shift+Right (highlight)
        let action = e.tick(&p((0.0, 0.0), (0.9, 0.0)), D);
        match action {
            Some(Action::Key(press)) => {
                assert_eq!(press.key, Key::Named(NamedKey::Right));
                assert!(press.shift, "โหมดเลือกข้อความต้องกด Shift ค้าง");
            }
            other => panic!("ต้องได้ Shift+Right: {other:?}"),
        }
    }
}
