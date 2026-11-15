//! Gesture engine — state machine ตาม `Design/02-design/04-state-machine-spec.md`
//!
//! รอบนี้ทำครบเฉพาะเส้นทางการพิมพ์ (IDLE → GROUP_ARMED → CHAR_SELECTED → COMMITTING)
//! ส่วน QUICKBAR / MODE_MENU / REPEATING / MOUSE ยังไม่ทำ (M2–M3)

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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureState {
    Idle,
    GroupArmed,
    CharSelected,
    Committing,
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

    /// เรียกทุก 4 ms (250 Hz) พร้อม `dt_ms` ที่ผ่านจริง
    /// คืน `Some(Commit)` เฉพาะเฟรมที่ยืนยันตัวอักษร
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32) -> Option<Commit> {
        let lm = pad.ls.magnitude();

        match self.state {
            GestureState::Idle => {
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
                    if self.selected_ms > self.cfg.cancel_dwell_ms {
                        // ค้างนานเกิน = ยกเลิก ไม่พิมพ์
                        self.cancel();
                    }
                } else if rm < self.cfg.rs_release {
                    self.state = GestureState::Committing;
                    self.commit_ms = 0;
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
                    return commit;
                }
            }
        }

        None
    }

    /// ยกเลิกกลาง gesture → ต้องปล่อย LS ก่อนจึงเริ่มใหม่ (กันพิมพ์ค้าง)
    fn cancel(&mut self) {
        self.reset(true);
    }

    fn reset(&mut self, lock: bool) {
        self.state = GestureState::Idle;
        self.group = None;
        self.sector = None;
        self.selected_ms = 0;
        self.commit_ms = 0;
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
}
