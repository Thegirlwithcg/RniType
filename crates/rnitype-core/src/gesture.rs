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
}
