//! Normalizer: deadzone แบบวงกลม + trigger hysteresis + sector 8 ทิศ (มี hysteresis 9°)

use crate::pad::{PadState, Vec2};

/// 8 ทิศเรียงแบบเข็มทิศ (index = ทิศ, ใช้เป็น index ของ keymap ด้วย)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dir8 {
    N = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
}

impl Dir8 {
    pub const ALL: [Dir8; 8] = [
        Dir8::N,
        Dir8::NE,
        Dir8::E,
        Dir8::SE,
        Dir8::S,
        Dir8::SW,
        Dir8::W,
        Dir8::NW,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Dir8 {
        Dir8::ALL[index % 8]
    }

    pub fn center_deg(self) -> f32 {
        self.index() as f32 * 45.0
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Dir8::N => "↑",
            Dir8::NE => "↗",
            Dir8::E => "→",
            Dir8::SE => "↘",
            Dir8::S => "↓",
            Dir8::SW => "↙",
            Dir8::W => "←",
            Dir8::NW => "↖",
        }
    }
}

/// ระยะเชิงมุมที่สั้นที่สุดระหว่างสองมุม (0–180)
pub fn angle_delta(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { 360.0 - d } else { d }
}

/// แปลงเวกเตอร์เป็นทิศ 8 ทิศ โดยคงทิศเดิมไว้จนกว่าจะเบี่ยงเกิน 22.5° + hysteresis
#[derive(Clone, Debug)]
pub struct SectorTracker {
    current: Option<Dir8>,
    hysteresis_deg: f32,
}

impl SectorTracker {
    pub fn new(hysteresis_deg: f32) -> Self {
        Self { current: None, hysteresis_deg }
    }

    pub fn current(&self) -> Option<Dir8> {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = None;
    }

    /// คืนทิศเมื่อขนาดเกิน `activate` (ไม่เกิน = คืน `None` และล้างทิศเดิม)
    pub fn update(&mut self, value: Vec2, activate: f32) -> Option<Dir8> {
        if value.magnitude() < activate {
            self.current = None;
            return None;
        }

        let angle = value.angle_deg();
        let target = Dir8::from_index(((angle + 22.5) / 45.0).floor() as usize);

        if let Some(cur) = self.current
            && cur != target
            && angle_delta(angle, cur.center_deg()) <= 22.5 + self.hysteresis_deg
        {
            // ยังอยู่ในโซนของทิศเดิม = ไม่เปลี่ยน (กันสั่นตรงเส้นแบ่ง)
            return self.current;
        }

        self.current = Some(target);
        self.current
    }
}

/// ตัด deadzone แบบวงกลม แล้วขยายค่าที่เหลือกลับเป็น 0–1 เพื่อไม่ให้การควบคุมด้านนอกอ่อนลง
pub fn apply_deadzone(value: Vec2, deadzone: f32) -> Vec2 {
    let m = value.magnitude();
    let dz = deadzone.clamp(0.0, 0.99);
    if m <= dz {
        return Vec2::ZERO;
    }
    let scale = ((m - dz) / (1.0 - dz)).min(1.0) / m;
    Vec2::new(value.x * scale, value.y * scale)
}

/// ค่าเริ่มต้นตาม `Design/02-design/04-state-machine-spec.md`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizerConfig {
    pub deadzone: f32,
    pub trigger_down: f32,
    pub trigger_up: f32,
}

impl Default for NormalizerConfig {
    fn default() -> Self {
        Self { deadzone: 0.15, trigger_down: 0.40, trigger_up: 0.20 }
    }
}

/// Normalizer มีสถานะเฉพาะ trigger hysteresis
#[derive(Clone, Debug, Default)]
pub struct Normalizer {
    cfg: NormalizerConfig,
    lt_down: bool,
    rt_down: bool,
}

impl Normalizer {
    pub fn new(cfg: NormalizerConfig) -> Self {
        Self { cfg, lt_down: false, rt_down: false }
    }

    pub fn config(&self) -> NormalizerConfig {
        self.cfg
    }

    pub fn apply(&mut self, raw: &PadState) -> PadState {
        self.lt_down = hysteresis(raw.lt_analog, self.lt_down, self.cfg.trigger_down, self.cfg.trigger_up);
        self.rt_down = hysteresis(raw.rt_analog, self.rt_down, self.cfg.trigger_down, self.cfg.trigger_up);

        PadState {
            ls: apply_deadzone(raw.ls, self.cfg.deadzone),
            rs: apply_deadzone(raw.rs, self.cfg.deadzone),
            lt_analog: raw.lt_analog.clamp(0.0, 1.0),
            rt_analog: raw.rt_analog.clamp(0.0, 1.0),
            buttons: crate::pad::Buttons {
                lt: self.lt_down,
                rt: self.rt_down,
                ..raw.buttons
            },
        }
    }
}

fn hysteresis(value: f32, down: bool, on: f32, off: f32) -> bool {
    if down { value >= off } else { value >= on }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_zeroes_center_and_rescales_edge() {
        assert_eq!(apply_deadzone(Vec2::new(0.1, 0.05), 0.15), Vec2::ZERO);
        let out = apply_deadzone(Vec2::new(1.0, 0.0), 0.15);
        assert!((out.magnitude() - 1.0).abs() < 1e-5);

        // กลางระหว่าง deadzone กับขอบ → ถูกขยายขึ้น ไม่ใช่ค่าดิบ
        let half = apply_deadzone(Vec2::new(0.575, 0.0), 0.15);
        assert!((half.magnitude() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn sector_picks_nearest_direction() {
        let mut t = SectorTracker::new(9.0);
        assert_eq!(t.update(Vec2::new(0.0, 0.9), 0.35), Some(Dir8::N));
        assert_eq!(t.update(Vec2::new(0.9, 0.9), 0.35), Some(Dir8::NE));
        assert_eq!(t.update(Vec2::new(0.9, 0.0), 0.35), Some(Dir8::E));
        assert_eq!(t.update(Vec2::new(0.0, 0.0), 0.35), None);
    }

    #[test]
    fn sector_hysteresis_holds_near_boundary() {
        let mut t = SectorTracker::new(9.0);
        assert_eq!(t.update(Vec2::new(0.0, 0.9), 0.35), Some(Dir8::N));

        // 30° ยังอยู่ในโซน N (22.5 + 9 = 31.5)
        let wobble = Vec2::new(30f32.to_radians().sin(), 30f32.to_radians().cos());
        assert_eq!(t.update(wobble, 0.35), Some(Dir8::N));

        // 40° เบี่ยงเกินโซนแล้ว → เปลี่ยนเป็น NE
        let clear = Vec2::new(40f32.to_radians().sin(), 40f32.to_radians().cos());
        assert_eq!(t.update(clear, 0.35), Some(Dir8::NE));
    }

    #[test]
    fn trigger_hysteresis_prevents_chatter() {
        let mut n = Normalizer::new(NormalizerConfig::default());
        let mut raw = PadState { lt_analog: 0.45, ..PadState::default() };
        assert!(n.apply(&raw).buttons.lt);

        raw.lt_analog = 0.30; // ระหว่าง up กับ down → ต้องยังกดอยู่
        assert!(n.apply(&raw).buttons.lt);

        raw.lt_analog = 0.10;
        assert!(!n.apply(&raw).buttons.lt);

        raw.lt_analog = 0.30; // ยังไม่ถึง down → ต้องไม่กด
        assert!(!n.apply(&raw).buttons.lt);
    }
}
