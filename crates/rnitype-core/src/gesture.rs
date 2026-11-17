//! Gesture engine — state machine ตาม `Design/02-design/04-state-machine-spec.md`
//!
//! รอบนี้ทำครบเส้นทางการพิมพ์ (IDLE → GROUP_ARMED → CHAR_SELECTED → COMMITTING)
//! และ REPEATING (auto-repeat หลัง commit)
//! QUICKBAR / CARET_NAV ไม่ใช่ state ของ gesture engine — runtime จัดการเอง (ADR-23)

use crate::normalizer::{Dir8, SectorTracker};
use crate::pad::PadState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureConfig {
    pub ls_activate: f32,
    pub ls_release: f32,
    pub rs_activate: f32,
    pub rs_release: f32,
    pub sector_hysteresis_deg: f32,
    pub commit_dwell_ms: u32,
    pub cancel_dwell_ms: u32,
    /// auto-repeat: เริ่มซ้ำหลังค้างทิศเดิมนานเท่านี้ (spec: 350 ms)
    pub repeat_delay_ms: u32,
    /// auto-repeat: ระยะระหว่างครั้งถัดไป (spec: 90 ms)
    pub repeat_rate_ms: u32,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            ls_activate: 0.35,
            ls_release: 0.25,
            rs_activate: 0.40,
            rs_release: 0.25,
            sector_hysteresis_deg: 9.0,
            commit_dwell_ms: 60,
            cancel_dwell_ms: 600,
            repeat_delay_ms: 350,
            repeat_rate_ms: 90,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureState {
    Idle,
    GroupArmed,
    CharSelected,
    Committing,
    /// ค้างทิศเดิมหลัง commit → พิมพ์ซ้ำ (REPEATING)
    Repeating,
}

/// ผลลัพธ์เมื่อยืนยันตัวอักษรแล้ว (mapper แปลงเป็นคีย์ต่อ)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    pub group: Dir8,
    pub sector: Dir8,
}

#[derive(Clone, Debug)]
pub struct GestureEngine {
    cfg: GestureConfig,
    state: GestureState,
    group: Option<Dir8>,
    sector: Option<Dir8>,
    ls: SectorTracker,
    rs: SectorTracker,
    selected_ms: u32,
    commit_ms: u32,
    /// ช่องที่เพิ่ง commit — ถ้า LS ยังค้างและกลับไปค้างทิศเดิม = auto-repeat
    repeat_pending: Option<(Dir8, Dir8)>,
    repeat_ms: u32,
    repeat_count: u32,
    /// หลัง "ยกเลิก" (ไม่ใช่หลัง commit) ต้องปล่อย LS กลับก่อนจึงเริ่ม gesture ใหม่ได้
    locked: bool,
}

impl GestureEngine {
    pub fn new(cfg: GestureConfig) -> Self {
        Self {
            state: GestureState::Idle,
            group: None,
            sector: None,
            ls: SectorTracker::new(cfg.sector_hysteresis_deg),
            rs: SectorTracker::new(cfg.sector_hysteresis_deg),
            selected_ms: 0,
            commit_ms: 0,
            repeat_pending: None,
            repeat_ms: 0,
            repeat_count: 0,
            locked: false,
            cfg,
        }
    }

    pub fn config(&self) -> GestureConfig {
        self.cfg
    }

    pub fn state(&self) -> GestureState {
        self.state
    }

    pub fn group(&self) -> Option<Dir8> {
        self.group
    }

    pub fn sector(&self) -> Option<Dir8> {
        self.sector
    }

    /// จำนวนครั้งที่พิมพ์ซ้ำในรอบ REPEATING ปัจจุบัน (`0` = ไม่ได้ repeat)
    pub fn repeat_count(&self) -> u32 {
        self.repeat_count
    }

    /// เรียกทุก 4 ms (250 Hz) พร้อม `dt_ms` ที่ผ่านจริง
    /// คืน `Some(Commit)` เฉพาะเฟรมที่ยืนยันตัวอักษร
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32) -> Option<Commit> {
        let lm = pad.ls.magnitude();

        match self.state {
            GestureState::Idle => {
                if lm < self.cfg.ls_release {
                    // ปล่อย LS = จบรอบ auto-repeat
                    self.repeat_pending = None;
                }
                if self.locked {
                    // หลังยกเลิก ต้องปล่อย LS กลับก่อน กัน "พิมพ์ค้าง" จากการกดค้าง
                    if lm < self.cfg.ls_release {
                        self.locked = false;
                    } else {
                        return None;
                    }
                }
                if lm >= self.cfg.ls_activate
                    && let Some(group) = self.ls.update(pad.ls, self.cfg.ls_activate)
                {
                    if self.repeat_pending.map(|(pending, _)| pending) != Some(group) {
                        self.repeat_pending = None;
                    }
                    self.group = Some(group);
                    self.sector = None;
                    self.rs.reset();
                    self.selected_ms = 0;
                    self.commit_ms = 0;
                    self.state = GestureState::GroupArmed;
                }
            }

            GestureState::GroupArmed => {
                if lm < self.cfg.ls_release {
                    self.cancel();
                    return None;
                }
                // ยังเปลี่ยนกลุ่มได้ก่อนเลือกตัวอักษร
                if let Some(group) = self.ls.update(pad.ls, self.cfg.ls_release) {
                    if self.repeat_pending.map(|(pending, _)| pending) != Some(group) {
                        self.repeat_pending = None;
                    }
                    self.group = Some(group);
                }
                if let Some(sector) = self.rs.update(pad.rs, self.cfg.rs_activate) {
                    self.sector = Some(sector);
                    self.selected_ms = 0;
                    self.state = GestureState::CharSelected;
                }
            }

            GestureState::CharSelected => {
                if lm < self.cfg.ls_release {
                    self.cancel();
                    return None;
                }
                let rm = pad.rs.magnitude();
                if rm >= self.cfg.rs_activate {
                    if let Some(sector) = self.rs.update(pad.rs, self.cfg.rs_activate)
                        && Some(sector) != self.sector
                    {
                        self.sector = Some(sector);
                        self.selected_ms = 0;
                    }
                    self.selected_ms = self.selected_ms.saturating_add(dt_ms);

                    // auto-repeat: ค้างทิศเดิมหลัง commit ครบ repeat_delay → พิมพ์ซ้ำ
                    if let Some((pending_group, pending_sector)) = self.repeat_pending {
                        if self.group == Some(pending_group) && self.sector == Some(pending_sector)
                        {
                            if self.selected_ms >= self.cfg.repeat_delay_ms {
                                self.state = GestureState::Repeating;
                                self.repeat_ms = 0;
                                self.repeat_count = 1;
                                return Some(Commit {
                                    group: pending_group,
                                    sector: pending_sector,
                                });
                            }
                        } else {
                            self.repeat_pending = None;
                        }
                    }

                    if self.selected_ms > self.cfg.cancel_dwell_ms {
                        // ค้างนานเกิน = ยกเลิก ไม่พิมพ์
                        self.cancel();
                    }
                } else if rm < self.cfg.rs_release {
                    self.state = GestureState::Committing;
                    self.commit_ms = 0;
                }
            }

            GestureState::Repeating => {
                if lm < self.cfg.ls_release {
                    self.cancel();
                    return None;
                }
                // เปลี่ยนกลุ่ม = เริ่มเลือกใหม่ (ไม่ repeat ต่อ)
                if let Some(group) = self.ls.update(pad.ls, self.cfg.ls_release)
                    && Some(group) != self.group
                {
                    self.group = Some(group);
                    self.sector = None;
                    self.selected_ms = 0;
                    self.repeat_pending = None;
                    self.state = GestureState::GroupArmed;
                    return None;
                }

                let rm = pad.rs.magnitude();
                if rm < self.cfg.rs_release {
                    // ปล่อย RS = หยุด repeat (ยังจำช่องไว้ให้ดันซ้ำได้)
                    self.state = GestureState::Idle;
                    self.sector = None;
                    self.selected_ms = 0;
                    self.repeat_ms = 0;
                    self.repeat_count = 0;
                    return None;
                }
                if rm >= self.cfg.rs_activate
                    && let Some(sector) = self.rs.update(pad.rs, self.cfg.rs_activate)
                    && Some(sector) != self.sector
                {
                    // เปลี่ยนช่องระหว่างค้าง = เลือกตัวใหม่ตามปกติ
                    self.sector = Some(sector);
                    self.selected_ms = 0;
                    self.repeat_pending = None;
                    self.state = GestureState::CharSelected;
                    return None;
                }

                self.repeat_ms = self.repeat_ms.saturating_add(dt_ms);
                if self.repeat_ms >= self.cfg.repeat_rate_ms {
                    self.repeat_ms = 0;
                    self.repeat_count = self.repeat_count.saturating_add(1);
                    return self
                        .group
                        .zip(self.sector)
                        .map(|(group, sector)| Commit { group, sector });
                }
            }

            GestureState::Committing => {
                if lm < self.cfg.ls_release {
                    self.cancel();
                    return None;
                }
                if pad.rs.magnitude() > self.cfg.rs_activate {
                    // เปลี่ยนใจ = กลับไปเลือก
                    self.state = GestureState::CharSelected;
                    self.selected_ms = 0;
                    return None;
                }
                self.commit_ms = self.commit_ms.saturating_add(dt_ms);
                if self.commit_ms >= self.cfg.commit_dwell_ms {
                    let commit = self
                        .group
                        .zip(self.sector)
                        .map(|(group, sector)| Commit { group, sector });
                    self.reset(false);
                    // จำช่องไว้: ถ้า LS ยังค้างและดัน RS ทิศเดิมต่อ = auto-repeat
                    self.repeat_pending = commit.map(|commit| (commit.group, commit.sector));
                    return commit;
                }
            }
        }

        None
    }

    /// ยกเลิกกลาง gesture → ต้องปล่อย LS ก่อนจึงเริ่มใหม่ (กันพิมพ์ค้าง)
    /// เรียกจาก [`crate::runtime::RuntimeEngine`] เมื่อมี event ที่สำคัญกว่า
    /// (เปิดวงคำสั่ง / สลับ mode / ปิดฉุกเฉิน) ตาม spec §3
    pub fn cancel(&mut self) {
        self.reset(true);
    }

    /// `true` = ถูกยกเลิกและยังรอ LS กลับเข้าใกล้ศูนย์ก่อนเริ่ม gesture ใหม่
    pub fn locked(&self) -> bool {
        self.locked
    }

    fn reset(&mut self, lock: bool) {
        self.state = GestureState::Idle;
        self.group = None;
        self.sector = None;
        self.selected_ms = 0;
        self.commit_ms = 0;
        self.repeat_pending = None;
        self.repeat_ms = 0;
        self.repeat_count = 0;
        self.locked = lock;
        self.rs.reset();
    }
}

/* ---------------- Fast mode (ADR-32 / ADR-33) ---------------- */
//
// Fast mode v2 = "push-through" (ADR-33)
//   HIDDEN   : ยังไม่มี UI · A ไม่ทำอะไร · B = Backspace (runtime)
//   CATEGORY : เอียง LS กลาง ๆ (0.30–0.75) = เลือกหมวด · วงนอกยังไม่ไฮไลต์
//   LETTER   : ดันสุด (≥ 0.85 ค้าง 60 ms) = ล็อกหมวด แล้ว LS เลือกตัวอักษร · A = พิมพ์
//
// - ดึงกลับ < 0.70 = กลับไปเลือกหมวดโดยไม่ต้องผ่านศูนย์
// - LS ศูนย์ค้าง 250 ms = ซ่อน UI + ล้างหมวด (กัน spring-back สร้างทิศผี)
// - D-pad ↑/↓ = ±1 · → = เข้าลึก/หมวดถัดไป · ← หรือ B = ย้อน 1 ระดับ (runtime)
// - เจ้าของเมนู = ตัวที่ขยับล่าสุด · LS แย่งกลับเมื่อ r > 0.5 และข้าม sector
// - `letterPick: "dpad"` = พฤติกรรม 0.2.5: LS เปลี่ยนหมวดได้ตลอด · D-pad เลือกตัวอักษร

/// โซนศูนย์: ต่ำกว่านี้ = อยู่ศูนย์
pub const FAST_CENTER_IN: f32 = 0.20;
/// โซนหมวด: เริ่มที่ค่านี้
pub const FAST_CENTER_OUT: f32 = 0.30;
/// โซนตัวอักษร: ต้องดันถึงค่านี้
pub const FAST_LETTER_IN: f32 = 0.85;
/// ออกจากโซนตัวอักษรเมื่อต่ำกว่านี้
pub const FAST_LETTER_OUT: f32 = 0.70;
/// หมวดต้องนิ่งกี่ ms ก่อน latch
pub const FAST_LATCH_MS: u32 = 60;
/// อยู่ศูนย์ต่อเนื่องกี่ ms แล้วซ่อน UI
pub const FAST_CENTER_RESET_MS: u32 = 40;
/// อยู่ศูนย์ต่อเนื่องกี่ ms แล้วล้างการเลือกจริง (ยังกด A พิมพ์ตัวที่เลือกไว้ได้ก่อนหน้านี้)
pub const FAST_RESET_MS: u32 = 1500;
/// ละเว้นทิศหลังเข้าศูนย์ (กัน spring-back)
pub const FAST_DIR_IGNORE_MS: u32 = 80;
/// กันกด A รัวหลังเปลี่ยนโซน — A ที่กดในช่วงนี้ถูก "เก็บไว้" พิมพ์ทีหลัง ไม่ถูกทิ้ง (v0.3.5)
pub const FAST_LOCKOUT_MS: u32 = 100;
/// อยู่ระหว่าง 0.20–0.70 นานเท่านี้ถึงนับว่า "ดึงกลับ" (ต่ำกว่า = Analog เด้งกลับศูนย์ ไม่ล้างช่อง)
pub const FAST_PULLBACK_MS: u32 = 100;
/// A ใช้ช่องที่ไฮไลต์ก่อนกดภายในเวลานี้ (anti-flick)
pub const FAST_LOOKBACK_MS: u32 = 40;
/// เมนูที่ขับด้วย D-pad และไม่มี input = ซ่อน
pub const FAST_DPAD_IDLE_MS: u32 = 2500;

/// จังหวะของ Fast mode (HUD ใช้)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastPhase {
    /// ยังไม่ได้เลือกช่องตัวอักษร
    Group,
    /// ไฮไลต์ช่องตัวอักษรอยู่ (A = พิมพ์)
    Char,
}

/// โซนของ Fast mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastZone {
    /// ไม่มี UI
    Hidden,
    /// เลือกหมวด (วงใน)
    Category,
    /// เลือกตัวอักษร (วงนอก)
    Letter,
}

/// ใครเป็นเจ้าของเมนูตอนนี้
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastOwner {
    Stick,
    Dpad,
}

/// ทิศที่ D-pad เพิ่งกด
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastNav {
    Prev,
    Next,
}

impl FastNav {
    fn delta(self) -> i32 {
        match self {
            FastNav::Prev => -1,
            FastNav::Next => 1,
        }
    }
}

/// input ของ 1 เฟรม (rising edge ทั้งหมด)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FastInput {
    /// A
    pub accept: bool,
    /// D-pad ↑/↓
    pub nav: Option<FastNav>,
    /// D-pad →
    pub enter: bool,
}

/// ผลเมื่อกด A ยืนยันตัวอักษร
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FastCommit {
    pub group: Dir8,
    pub sector: Dir8,
}

/// State machine ของ Fast mode (ไม่ผูกกับ keymap — pipeline เป็นคนแปลงเป็นคีย์)
#[derive(Clone, Debug)]
pub struct FastGesture {
    /// true = push-through (ADR-33) · false = D-pad เลือกตัวอักษร (0.2.5)
    push: bool,
    zone: FastZone,
    owner: FastOwner,
    group: Option<Dir8>,
    last_group: Option<Dir8>,
    sector: Option<Dir8>,
    prev_sector: Option<Dir8>,
    /// ช่องที่ไฮไลต์ล่าสุด (ยังไม่พิมพ์) — ให้ A พิมพ์ได้แม้ปล่อยจอย/UI ซ่อนไปแล้ว
    last_pick: Option<(Dir8, Dir8)>,
    ls: SectorTracker,
    hold_ms: u32,
    center_ms: u32,
    ignore_ms: u32,
    stable_ms: u32,
    sector_age_ms: u32,
    phase_ms: u32,
    quiet_ms: u32,
    /// เวลาที่ LS อยู่ช่วงกลาง (CENTER_IN..LETTER_OUT) ขณะอยู่โซนตัวอักษร
    band_ms: u32,
    /// A ที่กดตอนยังติด lockout / ตอนกำลัง latch — พิมพ์ทันทีเมื่อพร้อม (ไม่ถูกทิ้ง)
    pending_accept: bool,
    /// ช่องที่ไฮไลต์ถูกพิมพ์ไปแล้ว (ยังดัน LS ค้างเลยคงไฮไลต์ไว้) — พอปล่อย LS A ต้องเป็น Space
    consumed: bool,
    radius: f32,
    /// ซ่อน UI แล้ว (อยู่ศูนย์ ≥ 120 ms) แต่ยังพิมพ์ตัวที่เลือกไว้ได้
    ui_hidden: bool,
}

impl FastGesture {
    pub fn new(cfg: GestureConfig) -> Self {
        Self {
            push: true,
            zone: FastZone::Hidden,
            owner: FastOwner::Stick,
            group: None,
            last_group: None,
            sector: None,
            prev_sector: None,
            last_pick: None,
            ls: SectorTracker::new(cfg.sector_hysteresis_deg),
            hold_ms: 0,
            center_ms: 0,
            ignore_ms: 0,
            stable_ms: 0,
            sector_age_ms: u32::MAX,
            phase_ms: 0,
            quiet_ms: 0,
            band_ms: 0,
            pending_accept: false,
            consumed: false,
            radius: 0.0,
            ui_hidden: false,
        }
    }

    /// สลับโหมดเลือกตัวอักษร (config `defaults.typing.letterPick`)
    pub fn set_push(&mut self, push: bool) {
        if self.push != push {
            self.push = push;
            self.reset();
        }
    }

    pub fn push(&self) -> bool {
        self.push
    }

    pub fn zone(&self) -> FastZone {
        self.zone
    }

    pub fn owner(&self) -> FastOwner {
        self.owner
    }

    pub fn phase(&self) -> FastPhase {
        if self.zone == FastZone::Letter {
            FastPhase::Char
        } else {
            FastPhase::Group
        }
    }

    pub fn group(&self) -> Option<Dir8> {
        self.group
    }

    pub fn sector(&self) -> Option<Dir8> {
        self.sector
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// UI ถูกซ่อนเพราะปล่อย Analog แล้ว (แต่ยังพิมพ์ตัวที่เลือกไว้ได้)
    pub fn ui_hidden(&self) -> bool {
        self.ui_hidden
    }

    /// auto-repeat ไม่มีใน Fast mode (A ซ้ำ = พิมพ์ตัวเดิม)
    pub fn repeat_count(&self) -> u32 {
        0
    }

    /// B / D-pad ← = ย้อน 1 ระดับ
    ///
    /// LETTER → CATEGORY → HIDDEN · คืน `false` เมื่อไม่มีอะไรให้ย้อน (runtime ใช้ตัดสิน Backspace)
    pub fn cancel(&mut self) -> bool {
        match self.zone {
            FastZone::Letter => {
                self.zone = FastZone::Category;
                self.sector = None;
                self.prev_sector = None;
                self.stable_ms = 0;
                self.phase_ms = 0;
                self.pending_accept = false;
                true
            }
            FastZone::Category => {
                self.hide();
                true
            }
            FastZone::Hidden => false,
        }
    }

    /// L3 = ล้างช่องที่เลือก (หมวดเดิมยังอยู่) → กลับไปจังหวะหมวด
    pub fn unlock(&mut self) {
        if self.zone == FastZone::Letter {
            self.zone = FastZone::Category;
            self.phase_ms = 0;
        }
        self.sector = None;
        self.prev_sector = None;
        self.stable_ms = 0;
        self.pending_accept = false;
    }

    /// ล้างไฮไลต์ตัวอักษร (คืน `true` เมื่อมีอะไรให้ล้าง)
    pub fn clear_sector(&mut self) -> bool {
        let had = self.sector.is_some();
        self.sector = None;
        self.prev_sector = None;
        had
    }

    /// ออกจาก Fast mode / สลับโหมด — จำหมวดล่าสุดไว้ให้ D-pad เริ่มที่เดิม
    pub fn reset(&mut self) {
        self.zone = FastZone::Hidden;
        self.owner = FastOwner::Stick;
        self.group = None;
        self.sector = None;
        self.prev_sector = None;
        self.last_pick = None;
        self.ls.reset();
        self.hold_ms = 0;
        self.center_ms = 0;
        self.ignore_ms = 0;
        self.stable_ms = 0;
        self.sector_age_ms = u32::MAX;
        self.phase_ms = 0;
        self.quiet_ms = 0;
        self.band_ms = 0;
        self.pending_accept = false;
        self.radius = 0.0;
        self.ui_hidden = false;
    }

    fn hide(&mut self) {
        self.zone = FastZone::Hidden;
        self.group = None;
        self.sector = None;
        self.prev_sector = None;
        self.hold_ms = 0;
        self.center_ms = 0;
        self.stable_ms = 0;
        self.quiet_ms = 0;
        self.band_ms = 0;
        self.pending_accept = false;
        self.ui_hidden = false;
        self.ls.reset();
    }

    fn set_sector(&mut self, dir: Dir8) {
        if Some(dir) != self.sector {
            self.prev_sector = self.sector;
            self.sector = Some(dir);
            self.consumed = false;
            self.sector_age_ms = 0;
            if let Some(group) = self.group {
                self.last_pick = Some((group, dir));
            }
        }
    }

    /// 1 เฟรมของ Fast mode
    ///
    /// - `accept` = rising edge ของ A
    /// - `nav` = D-pad ↑/↓ · `enter` = D-pad →
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32, input: FastInput) -> Option<FastCommit> {
        let r = pad.ls.magnitude();
        self.radius = r;
        self.phase_ms = self.phase_ms.saturating_add(dt_ms);
        self.sector_age_ms = self.sector_age_ms.saturating_add(dt_ms);

        // "ศูนย์" = ต่ำกว่าเกณฑ์เปิดโซนหมวด (0.30) — ไม่เว้น dead band 0.20–0.30 อีกแล้ว
        // (Analog ที่หยุดค้างแถว 0.2–0.3 เคยทำให้ UI ไม่หายตลอดไป)
        if r < FAST_CENTER_OUT {
            self.center_ms = self.center_ms.saturating_add(dt_ms);
        } else {
            self.center_ms = 0;
            self.ignore_ms = self.ignore_ms.saturating_sub(dt_ms);
        }
        if r < FAST_CENTER_IN {
            self.hold_ms = 0;
            self.ignore_ms = FAST_DIR_IGNORE_MS;
        }
        // ปล่อย Analog แล้วซ่อน UI เร็ว ๆ แต่ยังไม่ล้างตัวที่เลือก (ให้ A ยังพิมพ์ได้)
        self.ui_hidden = self.center_ms >= FAST_CENTER_RESET_MS;

        if !self.push {
            return self.tick_classic(pad, input, r);
        }

        // เจ้าของเมนู = ตัวที่ขยับล่าสุด: LS แย่งกลับเมื่อเอียงแรงและข้าม sector
        if self.zone != FastZone::Hidden
            && self.owner == FastOwner::Dpad
            && self.ignore_ms == 0
            && r > 0.5
            && let Some(dir) = self.ls.update(pad.ls, 0.5)
            && Some(dir) != self.group
        {
            self.owner = FastOwner::Stick;
            self.group = Some(dir);
            self.last_group = Some(dir);
            self.stable_ms = 0;
            self.phase_ms = 0;
        }

        let dpad_input = input.nav.is_some() || input.enter;

        match self.zone {
            FastZone::Hidden => {
                // UI ซ่อนแล้ว แต่ A ยังต้องพิมพ์ตัวที่เลือกไว้ล่าสุดได้ (owner 2026-11-17)
                if input.accept
                    && let Some((group, sector)) = self.last_pick.take()
                {
                    return Some(FastCommit { group, sector });
                }
                if input.nav.is_some() || input.enter {
                    self.zone = FastZone::Category;
                    self.owner = FastOwner::Dpad;
                    self.group = self.last_group.or(Some(Dir8::N));
                    self.last_group = self.group;
                    self.sector = None;
                    self.phase_ms = 0;
                    self.quiet_ms = 0;
                    if let Some(nav) = input.nav {
                        self.group = Some(step_sector(self.group.unwrap_or(Dir8::N), nav));
                        self.last_group = self.group;
                    }
                    return None;
                }
                if r > FAST_CENTER_OUT {
                    self.hold_ms = self.hold_ms.saturating_add(dt_ms);
                    if self.hold_ms >= FAST_LATCH_MS {
                        self.zone = FastZone::Category;
                        self.owner = FastOwner::Stick;
                        self.phase_ms = 0;
                        self.stable_ms = 0;
                    }
                }
            }

            FastZone::Category => match self.owner {
                FastOwner::Stick => {
                    if self.ignore_ms == 0
                        && r >= FAST_CENTER_OUT
                        && let Some(dir) = self.ls.update(pad.ls, FAST_CENTER_OUT)
                        && Some(dir) != self.group
                    {
                        self.group = Some(dir);
                        self.last_group = Some(dir);
                        self.stable_ms = 0;
                        self.last_pick = None;
                    }
                    if r >= FAST_LETTER_IN && self.group.is_some() {
                        // A ที่กดระหว่างรอ latch (60 ms) ไม่ถูกทิ้ง — พิมพ์ทันทีหลังเข้าโซนตัวอักษร
                        if input.accept && self.stable_ms > 0 {
                            self.pending_accept = true;
                        }
                        self.stable_ms = self.stable_ms.saturating_add(dt_ms);
                        if self.stable_ms >= FAST_LATCH_MS {
                            self.zone = FastZone::Letter;
                            self.phase_ms = 0;
                            self.band_ms = 0;
                            if let Some(group) = self.group {
                                self.set_sector(group);
                            }
                        }
                    } else {
                        self.stable_ms = 0;
                        self.pending_accept = false;
                    }
                    if self.center_ms >= FAST_RESET_MS {
                        self.hide();
                    }
                }
                FastOwner::Dpad => {
                    if dpad_input {
                        self.quiet_ms = 0;
                    } else {
                        self.quiet_ms = self.quiet_ms.saturating_add(dt_ms);
                    }
                    if let Some(nav) = input.nav {
                        self.group = Some(step_sector(self.group.unwrap_or(Dir8::N), nav));
                        self.last_group = self.group;
                    }
                    if input.enter || input.accept {
                        if self.group.is_some() {
                            self.zone = FastZone::Letter;
                            self.phase_ms = 0;
                            if let Some(group) = self.group {
                                self.set_sector(group);
                            }
                        }
                        return None;
                    }
                    if self.quiet_ms >= FAST_DPAD_IDLE_MS {
                        self.hide();
                    }
                }
            },

            FastZone::Letter => {
                match self.owner {
                    FastOwner::Stick => {
                        if r >= FAST_LETTER_OUT {
                            // เปลี่ยนช่องได้เฉพาะตอนดันสุดจริง — ระหว่าง Analog เด้งกลับศูนย์ (r ลดผ่าน
                            // 0.7→0.2) ทิศสั่นไม่มีผล จึงไม่เปลี่ยนตัวที่ไฮไลต์ไว้
                            self.band_ms = 0;
                            if self.ignore_ms == 0
                                && let Some(dir) = self.ls.update(pad.ls, FAST_CENTER_OUT)
                            {
                                self.set_sector(dir);
                            }
                        } else if r < FAST_CENTER_IN {
                            // ปล่อย Analog = ซ่อน UI แต่ยังไม่ล้างช่องที่เลือก → A ยังพิมพ์ได้
                            self.band_ms = 0;
                            if self.center_ms >= FAST_RESET_MS {
                                self.hide();
                                return None;
                            }
                        } else {
                            // ช่วงกลาง: ผ่านเร็ว ๆ = เด้งกลับ (คงช่อง) · ค้างอยู่ = ดึงกลับเลือกหมวดใหม่
                            self.band_ms = self.band_ms.saturating_add(dt_ms);
                            if self.band_ms >= FAST_PULLBACK_MS {
                                self.zone = FastZone::Category;
                                self.sector = None;
                                self.prev_sector = None;
                                self.stable_ms = 0;
                                self.phase_ms = 0;
                                self.band_ms = 0;
                                self.pending_accept = false;
                                return None;
                            }
                        }
                    }
                    FastOwner::Dpad => {
                        if dpad_input || input.accept {
                            self.quiet_ms = 0;
                        } else {
                            self.quiet_ms = self.quiet_ms.saturating_add(dt_ms);
                        }
                        if let Some(nav) = input.nav {
                            self.set_sector(step_sector(
                                self.sector.or(self.group).unwrap_or(Dir8::N),
                                nav,
                            ));
                        }
                        if self.quiet_ms >= FAST_DPAD_IDLE_MS {
                            self.hide();
                            return None;
                        }
                    }
                }

                // A = พิมพ์ (lookback กัน stick สะเทือน) · A ที่มาในช่วง lockout ถูกเก็บไว้
                // พิมพ์เมื่อ lockout หมด — ห้ามทิ้ง edge (A เป็น rising edge ครั้งเดียว)
                if input.accept || self.pending_accept {
                    if let (Some(group), Some(sector)) = (self.group, self.sector) {
                        if self.phase_ms < FAST_LOCKOUT_MS {
                            self.pending_accept = true;
                            return None;
                        }
                        self.pending_accept = false;
                        // พิมพ์ช่องนี้ไปแล้วและปล่อย LS แล้ว → ไม่พิมพ์ซ้ำ (runtime ส่ง Space)
                        if self.consumed && r < FAST_CENTER_OUT {
                            self.sector = None;
                            self.prev_sector = None;
                            self.sector_age_ms = u32::MAX;
                            self.consumed = false;
                            return None;
                        }
                        let sector = if self.sector_age_ms < FAST_LOOKBACK_MS {
                            self.prev_sector.unwrap_or(sector)
                        } else {
                            sector
                        };
                        // กด A ตอนไม่ได้โยกจอย (stick อยู่ศูนย์) = ล้างไฮไลต์ ไม่ให้ค้างที่ตัวเดิม
                        // ถ้ายังดันอยู่ = คงไฮไลต์เหมือน 0.3.5 (เลือกต่อ/ข้ามหมวดได้)
                        if r < FAST_CENTER_OUT {
                            self.sector = None;
                            self.prev_sector = None;
                            self.sector_age_ms = u32::MAX;
                        }
                        // พิมพ์แล้ว = ใช้ช่องหมดแล้ว → A ครั้งต่อไปตอนปล่อยจอยต้องเป็น Space (ไม่พิมพ์ซ้ำ)
                        self.last_pick = None;
                        self.consumed = true;
                        return Some(FastCommit { group, sector });
                    }
                    self.pending_accept = false;
                }
            }
        }
        None
    }

    /// พฤติกรรม 0.2.5 (fallback `letterPick: "dpad"`)
    fn tick_classic(&mut self, pad: &PadState, input: FastInput, r: f32) -> Option<FastCommit> {
        if self.ignore_ms == 0
            && r >= FAST_CENTER_OUT
            && let Some(dir) = self.ls.update(pad.ls, FAST_CENTER_OUT)
            && Some(dir) != self.group
        {
            self.group = Some(dir);
            self.last_group = Some(dir);
            self.set_sector(dir);
            self.phase_ms = 0;
        }

        self.zone = if self.sector.is_some() {
            FastZone::Letter
        } else if self.group.is_some() {
            FastZone::Category
        } else {
            FastZone::Hidden
        };

        if let Some(nav) = input.nav
            && self.group.is_some()
        {
            self.set_sector(step_sector(self.sector.unwrap_or(Dir8::N), nav));
            self.zone = FastZone::Letter;
        }

        if input.accept
            && let (Some(group), Some(sector)) = (self.group, self.sector)
        {
            return Some(FastCommit { group, sector });
        }
        None
    }
}

fn wrap_index(value: i32) -> usize {
    value.rem_euclid(8) as usize
}

fn step_sector(current: Dir8, nav: FastNav) -> Dir8 {
    Dir8::from_index(wrap_index(current.index() as i32 + nav.delta()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pad::Vec2;

    fn pad(ls: (f32, f32), rs: (f32, f32)) -> PadState {
        PadState {
            ls: Vec2::new(ls.0, ls.1),
            rs: Vec2::new(rs.0, rs.1),
            ..PadState::default()
        }
    }

    fn engine() -> GestureEngine {
        GestureEngine::new(GestureConfig::default())
    }

    #[test]
    fn ls_alone_never_types() {
        let mut e = engine();
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4).is_none());
        for _ in 0..20 {
            e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        }
        assert!(e.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4).is_none());
        assert_eq!(e.state(), GestureState::Idle);
    }

    #[test]
    fn commit_needs_release_and_dwell() {
        let mut e = engine();
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        assert_eq!(e.state(), GestureState::GroupArmed);

        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4).is_none());
        assert_eq!(e.state(), GestureState::CharSelected);

        // ปล่อย RS ลงศูนย์ → ยังไม่พิมพ์จนครบ dwell
        e.tick(&pad((0.0, 0.9), (0.0, 0.1)), 4);
        assert_eq!(e.state(), GestureState::Committing);

        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 40).is_none());
        let commit = e
            .tick(&pad((0.0, 0.9), (0.0, 0.0)), 24)
            .expect("ต้อง commit");
        assert_eq!(
            commit,
            Commit {
                group: Dir8::N,
                sector: Dir8::N
            }
        );

        // ไม่พิมพ์ซ้ำ
        for _ in 0..10 {
            assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4).is_none());
        }
    }

    #[test]
    fn releasing_ls_during_commit_cancels() {
        let mut e = engine();
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4); // arm
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4); // select
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        assert_eq!(e.state(), GestureState::Committing);
        assert!(e.tick(&pad((0.0, 0.1), (0.0, 0.0)), 40).is_none());
        assert!(e.tick(&pad((0.0, 0.0), (0.0, 0.0)), 40).is_none());
        assert_eq!(e.state(), GestureState::Idle);
    }

    #[test]
    fn holding_rs_too_long_cancels_without_typing() {
        let mut e = engine();
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4); // arm
        for _ in 0..200 {
            if e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4).is_some() {
                panic!("ค้าง RS เกิน cancel_dwell ต้องไม่พิมพ์");
            }
        }
        // ปล่อย RS แล้วรอ dwell — ต้องไม่มี commit หลุดออกมา
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4).is_none());
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 100).is_none());
    }

    #[test]
    fn changing_sector_before_release_changes_character() {
        let mut e = engine();
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.9, 0.0)), 4); // เลื่อนไปทางขวา
        assert_eq!(e.sector(), Some(Dir8::E));
        e.tick(&pad((0.0, 0.9), (0.0, 0.1)), 4);
        let commit = e
            .tick(&pad((0.0, 0.9), (0.0, 0.0)), 64)
            .expect("ต้อง commit");
        assert_eq!(
            commit,
            Commit {
                group: Dir8::N,
                sector: Dir8::E
            }
        );
    }

    #[test]
    fn changing_group_while_armed_uses_latest_group() {
        let mut e = engine();
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        e.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4);
        assert_eq!(e.group(), Some(Dir8::E));
        e.tick(&pad((0.9, 0.0), (0.0, 0.9)), 4);
        e.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4);
        let commit = e
            .tick(&pad((0.9, 0.0), (0.0, 0.0)), 64)
            .expect("ต้อง commit");
        assert_eq!(
            commit,
            Commit {
                group: Dir8::E,
                sector: Dir8::N
            }
        );
    }

    /* ---------------- auto-repeat (REPEATING) ---------------- */

    /// พิมพ์ 1 ตัวด้วยท่ามาตรฐาน แล้วคืน engine ที่ LS ยังค้างอยู่
    fn commit_one(e: &mut GestureEngine) -> Commit {
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 64)
            .expect("ต้อง commit ครั้งแรก")
    }

    #[test]
    fn holding_the_same_sector_after_commit_repeats_at_spec_timings() {
        let mut e = engine();
        let first = commit_one(&mut e);
        assert_eq!(e.state(), GestureState::Idle);

        // ดัน RS กลับทิศเดิม
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        assert_eq!(e.state(), GestureState::GroupArmed);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        assert_eq!(e.state(), GestureState::CharSelected);

        // ยังไม่ครบ 350 ms → ไม่ repeat
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 300).is_none());
        // ครบ 350 ms → repeat ครั้งแรก
        let repeat = e
            .tick(&pad((0.0, 0.9), (0.0, 0.9)), 50)
            .expect("ต้อง repeat");
        assert_eq!(repeat, first);
        assert_eq!(e.state(), GestureState::Repeating);
        assert_eq!(e.repeat_count(), 1);

        // 90 ms ต่อครั้ง
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 89).is_none());
        assert_eq!(
            e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 1),
            Some(first),
            "ต้อง repeat ครั้งที่ 2"
        );
        assert_eq!(e.repeat_count(), 2);
    }

    #[test]
    fn releasing_rs_stops_repeating_and_ls_release_clears_it() {
        let mut e = engine();
        commit_one(&mut e);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 350);
        assert_eq!(e.state(), GestureState::Repeating);

        // ปล่อย RS → หยุด (ไม่พิมพ์เพิ่ม)
        e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4);
        assert_eq!(e.state(), GestureState::Idle);
        for _ in 0..50 {
            assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4).is_none());
        }
        assert_eq!(e.repeat_count(), 0);

        // ปล่อย LS → ล้าง pending: ดัน RS ทิศเดิมใหม่จะไม่ repeat อีก
        e.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4);
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4).is_none());
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        assert!(e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 400).is_none());
    }

    #[test]
    fn different_sector_after_commit_does_not_repeat() {
        let mut e = engine();
        commit_one(&mut e);
        // ดัน RS ไป ↘ (ช่องอื่น) ค้าง 500 ms → ต้องไม่ repeat (แต่ยังไม่ครบ cancel 600)
        e.tick(&pad((0.0, 0.9), (0.9, -0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.9, -0.9)), 4);
        assert!(e.tick(&pad((0.0, 0.9), (0.9, -0.9)), 500).is_none());
        assert_eq!(e.state(), GestureState::CharSelected);
        assert_eq!(e.sector(), Some(Dir8::SE));
    }

    #[test]
    fn changing_group_while_repeating_starts_a_new_selection() {
        let mut e = engine();
        commit_one(&mut e);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 4);
        e.tick(&pad((0.0, 0.9), (0.0, 0.9)), 350);
        assert_eq!(e.state(), GestureState::Repeating);

        // เลื่อน LS ไปกลุ่ม → ใหม่ = กลับ GroupArmed
        e.tick(&pad((0.9, 0.0), (0.0, 0.9)), 4);
        assert_eq!(e.state(), GestureState::GroupArmed);
        assert_eq!(e.group(), Some(Dir8::E));
    }

    /* ---------------- Fast mode (ADR-33) ---------------- */

    fn fast() -> FastGesture {
        FastGesture::new(GestureConfig::default())
    }

    fn none() -> FastInput {
        FastInput::default()
    }

    fn accept() -> FastInput {
        FastInput {
            accept: true,
            ..FastInput::default()
        }
    }

    fn step(f: &mut FastGesture, ls: (f32, f32)) -> Option<FastCommit> {
        f.tick(&pad(ls, (0.0, 0.0)), 4, none())
    }

    /// ดัน LS กลาง ๆ 80 ms → CATEGORY
    fn category(f: &mut FastGesture, dir: (f32, f32)) {
        for _ in 0..20 {
            step(f, dir);
        }
    }

    /// ดันสุด 240 ms → LETTER (ล็อกหมวดตามทิศ)
    fn letter(f: &mut FastGesture, dir: (f32, f32)) {
        category(f, dir);
        for _ in 0..60 {
            step(f, dir);
        }
    }

    #[test]
    fn hidden_needs_60ms_and_a_does_nothing_in_category() {
        let mut f = fast();
        assert_eq!(f.zone(), FastZone::Hidden);

        // 40 ms ยังไม่พอ
        for _ in 0..10 {
            step(&mut f, (0.0, 0.5));
        }
        assert_eq!(f.zone(), FastZone::Hidden);

        category(&mut f, (0.0, 0.5));
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.group(), Some(Dir8::N));
        assert_eq!(f.sector(), None, "วงนอกยังไม่ไฮไลต์ช่อง");

        // A ในจังหวะหมวด (เจ้าของ = LS) ต้องไม่พิมพ์
        assert_eq!(f.tick(&pad((0.0, 0.5), (0.0, 0.0)), 4, accept()), None);
    }

    #[test]
    fn push_to_the_edge_latches_the_group_then_a_types() {
        let mut f = fast();
        letter(&mut f, (0.9, 0.0));
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(f.group(), Some(Dir8::E));
        assert_eq!(f.sector(), Some(Dir8::E));

        assert_eq!(
            f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::E,
                sector: Dir8::E
            })
        );
        // ยังอยู่โซนตัวอักษร — กด A ซ้ำได้
        assert!(f.zone() == FastZone::Letter);
        assert_eq!(
            f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::E,
                sector: Dir8::E
            })
        );
    }

    #[test]
    fn a_is_ignored_for_100ms_after_entering_letter() {
        let mut f = fast();
        category(&mut f, (0.0, 0.5));
        // latch ด้วย tick เดียว 70 ms → LETTER เฟรมแรก (phase_ms = 0)
        f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 70, none());
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4, accept()), None);

        // ผ่าน 100 ms → พิมพ์ได้
        for _ in 0..26 {
            step(&mut f, (0.0, 0.9));
        }
        assert!(
            f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4, accept()).is_some(),
            "หลัง lockout ต้องพิมพ์ได้"
        );
    }

    #[test]
    fn changing_direction_changes_the_letter_not_the_group() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.group(), Some(Dir8::N));

        step(&mut f, (0.9, 0.0));
        assert_eq!(f.sector(), Some(Dir8::E));
        assert_eq!(f.group(), Some(Dir8::N), "หมวดที่ latch แล้วห้ามเปลี่ยน");
    }

    #[test]
    fn a_uses_the_previous_cell_when_the_stick_just_moved() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.sector(), Some(Dir8::N));

        // เปลี่ยนทิศแล้วกด A ทันที (ภายใน 40 ms) → ใช้ช่องก่อนหน้า
        step(&mut f, (0.9, 0.0));
        assert_eq!(
            f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::N,
                sector: Dir8::N
            })
        );
    }

    #[test]
    fn pulling_back_returns_to_category_and_lets_the_group_change() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.zone(), FastZone::Letter);

        // ดึงกลับ < 0.70 แล้วค้างอยู่ช่วงกลาง ≥ 100 ms → CATEGORY, ไม่ต้องผ่านศูนย์
        for _ in 0..30 {
            step(&mut f, (0.0, 0.5));
        }
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.sector(), None);

        // หมุนไปทางขวา = เปลี่ยนหมวดได้ทันที
        for _ in 0..10 {
            step(&mut f, (0.6, 0.0));
        }
        assert_eq!(f.group(), Some(Dir8::E));
    }

    #[test]
    fn releasing_the_stick_hides_the_ui_but_a_still_types() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.sector(), Some(Dir8::N));

        // ปล่อย 200 ms → UI ซ่อน แต่ยังไม่ล้างช่องที่เลือก
        for _ in 0..50 {
            step(&mut f, (0.0, 0.0));
        }
        assert!(f.ui_hidden());
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(f.sector(), Some(Dir8::N));

        // A ยังพิมพ์ตัวที่เลือกไว้ได้ (bug "กด A ไม่ได้")
        assert_eq!(
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::N,
                sector: Dir8::N
            })
        );
    }

    /// Analog จริงไม่ได้ตกจาก 0.9 → 0 ในเฟรมเดียว: ไล่ผ่าน 0.8 → 0.6 → 0.4 → 0.25 → 0
    /// (ทิศแกว่งด้วย) — ต้องไม่ล้างช่องที่ไฮไลต์ไว้
    fn spring_back(f: &mut FastGesture) {
        for ls in [
            (0.0, 0.8),
            (0.1, 0.6),
            (0.2, 0.4),
            (0.2, 0.15),
            (0.1, 0.05),
            (0.0, 0.0),
        ] {
            step(f, ls);
        }
    }

    #[test]
    fn spring_back_transit_keeps_the_letter_so_a_types_immediately() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.sector(), Some(Dir8::N));

        spring_back(&mut f);
        assert_eq!(f.zone(), FastZone::Letter, "ผ่านช่วงกลางเร็ว ๆ ห้ามตกโซน");
        assert_eq!(f.sector(), Some(Dir8::N), "ทิศสั่นตอนเด้งกลับห้ามเปลี่ยนช่อง");

        // A ทันทีหลังปล่อย (UI ยังไม่ทันซ่อน) และหลัง 120 ms (UI ซ่อนแล้ว) ต้องพิมพ์ทั้งคู่
        let hit = FastCommit {
            group: Dir8::N,
            sector: Dir8::N,
        };
        assert_eq!(f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()), Some(hit));
        for _ in 0..40 {
            step(&mut f, (0.0, 0.0));
        }
        assert!(f.ui_hidden());
        // ตัวนั้นพิมพ์ไปแล้ว → A ต่อไปไม่พิมพ์ซ้ำ (runtime ส่ง Space)
        assert_eq!(f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()), None);
    }

    #[test]
    fn a_clears_the_highlight_only_when_the_stick_is_released() {
        let mut f = fast();
        letter(&mut f, (0.9, 0.0));
        assert_eq!(f.sector(), Some(Dir8::E));

        // ดันค้างอยู่ = คงไฮไลต์ (0.3.5) แล้วพิมพ์ได้
        assert!(f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()).is_some());
        assert_eq!(f.sector(), Some(Dir8::E), "ดันค้าง = ยังไฮไลต์เหมือนเดิม");

        // ปล่อยจอย (ไม่โยก) แล้วกด A → พิมพ์ได้ และไฮไลต์ต้องว่าง
        for _ in 0..10 {
            step(&mut f, (0.0, 0.0));
        }
        assert!(f.ui_hidden());
        assert_eq!(
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()),
            None,
            "พิมพ์ช่องนี้ไปแล้ว ปล่อยจอยแล้ว A ห้ามซ้ำ (= Space)"
        );
        assert_eq!(f.sector(), None, "ปล่อยจอยหลังพิมพ์ = ไฮไลต์ว่าง");

        // ขยับไปทิศอื่น = ได้ไฮไลต์ใหม่ (รอพ้น ignore หลังออกจากศูนย์)
        for _ in 0..25 {
            step(&mut f, (0.0, 0.9));
        }
        assert_eq!(f.sector(), Some(Dir8::N));
    }

    #[test]
    fn can_change_group_and_pick_another_character_after_printing() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9)); // หมวด N ช่อง N
        assert!(f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4, accept()).is_some());

        // ปล่อย → ดึงกลับช่วงกลาง = กลับไปเลือกหมวด
        for _ in 0..10 {
            step(&mut f, (0.0, 0.0));
        }
        for _ in 0..30 {
            step(&mut f, (0.5, 0.0));
        }
        assert_eq!(f.zone(), FastZone::Category);

        // ดันสุดทางขวา = latch หมวด E แล้วพิมพ์ได้ (รอพ้น lockout 100 ms)
        for _ in 0..40 {
            step(&mut f, (0.9, 0.0));
        }
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(f.group(), Some(Dir8::E));
        assert_eq!(f.sector(), Some(Dir8::E));
        assert_eq!(
            f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::E,
                sector: Dir8::E
            }),
            "ต้องเลือกอักษรข้ามหมวดได้"
        );
    }

    #[test]
    fn a_after_release_prints_a_selection_that_was_never_committed() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        assert_eq!(f.sector(), Some(Dir8::N));

        // ปล่อยจอยให้ UI ซ่อน แต่ยังไม่เคยกด A เลย
        for _ in 0..10 {
            step(&mut f, (0.0, 0.0));
        }
        assert!(f.ui_hidden());
        assert_eq!(
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()),
            Some(FastCommit {
                group: Dir8::N,
                sector: Dir8::N
            }),
            "A ต้องพิมพ์สิ่งที่เลือกไว้แม้ไม่เคย commit"
        );
    }

    #[test]
    fn hidden_a_types_the_unconsumed_pick_only_once() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        for _ in 0..400 {
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, none());
        }
        assert_eq!(f.zone(), FastZone::Hidden);
        assert!(f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()).is_some());
        assert_eq!(f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()), None);
    }

    #[test]
    fn a_after_release_types_once_then_yields_to_space() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        spring_back(&mut f);
        assert!(f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 40, accept()).is_some());
        for _ in 0..10 {
            assert!(
                f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 40, accept()).is_none(),
                "A ครั้งถัดไปห้ามพิมพ์ตัวเดิมซ้ำ"
            );
        }
    }

    #[test]
    fn resting_in_the_dead_band_still_hides_the_ui() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        // Analog หยุดค้างที่ 0.25 (ระหว่าง 0.20–0.30) — เดิมไม่นับเป็นศูนย์ UI เลยค้างตลอด
        for _ in 0..40 {
            step(&mut f, (0.0, 0.25));
        }
        assert!(f.ui_hidden(), "ต่ำกว่า 0.30 ครบ 120 ms ต้องซ่อน UI");
    }

    #[test]
    fn hovering_in_the_band_for_100ms_is_a_pull_back_not_a_release() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        for _ in 0..26 {
            step(&mut f, (0.0, 0.5));
        }
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.sector(), None);
    }

    #[test]
    fn a_pressed_during_the_lockout_is_queued_not_dropped() {
        let mut f = fast();
        category(&mut f, (0.0, 0.5));
        f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 70, none());
        assert_eq!(f.zone(), FastZone::Letter);

        // A ตอนเพิ่ง latch = ยังไม่พิมพ์ แต่ต้องไม่หาย → ออกมาเองเมื่อ lockout หมด (ครั้งเดียว)
        assert_eq!(f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4, accept()), None);
        let mut typed = 0;
        for _ in 0..40 {
            if step(&mut f, (0.0, 0.9)).is_some() {
                typed += 1;
            }
        }
        assert_eq!(typed, 1, "A ที่ถูกเก็บไว้ต้องพิมพ์ 1 ครั้งพอดี");
    }

    #[test]
    fn a_pressed_while_latching_is_not_lost() {
        let mut f = fast();
        category(&mut f, (0.0, 0.5));
        // เริ่มดันสุดแล้วกด A กลางทางก่อน latch ครบ 60 ms
        assert_eq!(step(&mut f, (0.0, 0.9)), None);
        assert_eq!(f.tick(&pad((0.0, 0.9), (0.0, 0.0)), 4, accept()), None);
        let mut typed = 0;
        for _ in 0..60 {
            if step(&mut f, (0.0, 0.9)).is_some() {
                typed += 1;
            }
        }
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(typed, 1, "A ระหว่าง latch ต้องพิมพ์หลังเข้าโซนตัวอักษร");
    }

    #[test]
    fn a_keeps_working_after_the_ui_hides() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        let first = f
            .tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept())
            .expect("A ครั้งแรกต้องพิมพ์");
        assert_eq!(f.zone(), FastZone::Letter);

        // ปล่อย Analog → UI ซ่อน (40 ms) แต่ A ยังพิมพ์ตัวเดิมได้
        for _ in 0..30 {
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, none());
        }
        assert!(f.ui_hidden());
        assert_eq!(
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()),
            None,
            "พิมพ์ไปแล้ว A ต้องไม่ซ้ำตัวเดิม"
        );
        let _ = first;

        // ค้างศูนย์จน reset ครบ (1.5 s) → zone/ไฮไลต์ล้าง แต่ A ยังพิมพ์ซ้ำได้
        for _ in 0..400 {
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, none());
        }
        assert_eq!(f.zone(), FastZone::Hidden);
        assert_eq!(
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, accept()),
            None,
            "A หลัง UI หายไปแล้วต้องไม่ซ้ำตัวเดิม"
        );
    }

    #[test]
    fn holding_the_centre_for_1_5s_resets_everything() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        for _ in 0..400 {
            step(&mut f, (0.0, 0.0));
        }
        assert_eq!(f.zone(), FastZone::Hidden);
        assert_eq!(f.group(), None);
        assert_eq!(f.sector(), None);
        assert!(f.ui_hidden(), "UI ซ่อนอยู่ (zone Hidden)");
    }

    #[test]
    fn spring_back_does_not_create_a_ghost_group() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        for _ in 0..50 {
            step(&mut f, (0.0, 0.0));
        }
        // เด้งไปฝั่งตรงข้าม (r = 0.4 แค่ 60 ms): ไม่ใช่การดึงกลับ → หมวด/ช่องต้องยังเป็น N
        for _ in 0..15 {
            step(&mut f, (0.0, -0.4));
        }
        assert_eq!(f.zone(), FastZone::Letter);
        assert_eq!(f.group(), Some(Dir8::N), "ช่วง ignore ต้องไม่เปลี่ยนหมวด");
        assert_eq!(f.sector(), Some(Dir8::N), "เด้งกลับต้องไม่เปลี่ยนช่อง");
    }

    #[test]
    fn dpad_opens_the_menu_and_ls_takes_it_back_when_pushed_hard() {
        let mut f = fast();
        let nav = FastInput {
            nav: Some(FastNav::Next),
            ..FastInput::default()
        };
        f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 4, nav);
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.owner(), FastOwner::Dpad);

        // → เข้าลึก
        f.tick(
            &pad((0.0, 0.0), (0.0, 0.0)),
            4,
            FastInput {
                enter: true,
                ..FastInput::default()
            },
        );
        assert_eq!(f.zone(), FastZone::Letter);

        // LS ดันแรงไปคนละทิศ → กลับมาเป็นเจ้าของ (ต้องพ้น ignore 80 ms ก่อน)
        for _ in 0..30 {
            step(&mut f, (0.9, 0.0));
        }
        assert_eq!(f.owner(), FastOwner::Stick);
        assert_eq!(f.group(), Some(Dir8::E));
    }

    #[test]
    fn b_walks_back_one_level_at_a_time() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));

        assert!(f.cancel());
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.group(), Some(Dir8::N));

        assert!(f.cancel());
        assert_eq!(f.zone(), FastZone::Hidden);
        assert!(!f.cancel(), "ไม่มีอะไรให้ย้อนแล้ว");
    }

    #[test]
    fn dpad_menu_hides_after_2_5s_idle() {
        let mut f = fast();
        f.tick(
            &pad((0.0, 0.0), (0.0, 0.0)),
            4,
            FastInput {
                nav: Some(FastNav::Prev),
                ..FastInput::default()
            },
        );
        assert_eq!(f.zone(), FastZone::Category);
        for _ in 0..3 {
            f.tick(&pad((0.0, 0.0), (0.0, 0.0)), 1000, none());
        }
        assert_eq!(f.zone(), FastZone::Hidden);
    }

    #[test]
    fn classic_mode_keeps_the_025_behaviour() {
        let mut f = fast();
        f.set_push(false);
        assert!(!f.push());

        // LS = หมวด + ไฮไลต์ช่องทิศเดียวกัน
        step(&mut f, (0.9, 0.0));
        assert_eq!(f.group(), Some(Dir8::E));
        assert_eq!(f.sector(), Some(Dir8::E));

        // D-pad → = ช่องถัดไป → A พิมพ์
        f.tick(
            &pad((0.9, 0.0), (0.0, 0.0)),
            4,
            FastInput {
                nav: Some(FastNav::Next),
                ..FastInput::default()
            },
        );
        assert_eq!(f.sector(), Some(Dir8::SE));
        assert!(f.tick(&pad((0.9, 0.0), (0.0, 0.0)), 4, accept()).is_some());
    }

    #[test]
    fn l3_clears_the_cell_and_resets_to_group_phase() {
        let mut f = fast();
        letter(&mut f, (0.0, 0.9));
        f.unlock();
        assert_eq!(f.zone(), FastZone::Category);
        assert_eq!(f.sector(), None);
        assert_eq!(f.group(), Some(Dir8::N));
    }
}
