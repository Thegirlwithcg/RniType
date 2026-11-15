//! Mouse / Nav math — สูตรอยู่ใน `Design/02-design/03-mouse-and-navigation.md`
//!
//! - [`PointerMover`] — RS → pointer (gamma 1.7 · deadzone 0.15 · smoothing 0.5)
//! - [`PivotMover`] — Nav mode: LS หยาบ 900 px/s · RS ละเอียด 320 px/s
//! - [`ScrollEngine`] — Mouse mode: LS → wheel notch (40 ms, เร่ง 3 notch หลัง 500 ms)
//!
//! ทุกตัวคืนค่าเป็นพิกัดหน้าจอ (x ขวา, y ลง) เพื่อให้ชั้นบนเอาไปใช้ต่อได้ตรง ๆ
//! (จอยใช้ y ขึ้น จึงกลับเครื่องหมายในโมดูลนี้ที่เดียว)

use crate::pad::Vec2;

/// ค่าคงที่ของ pointer ตาม §1 ของเอกสารเมาส์
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerConfig {
    pub deadzone: f32,
    pub gamma: f32,
    pub vmax: f32,
    pub smoothing: f32,
    pub precision: f32,
    pub turbo: f32,
}

impl Default for PointerConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            gamma: 1.7,
            vmax: 1600.0,
            smoothing: 0.5,
            precision: 0.3,
            turbo: 1.8,
        }
    }
}

/// แปลง RS เป็นระยะขยับ pointer (px) ต่อหนึ่ง tick
///
/// `precision` = LT (×0.3) · `turbo` = LB (×1.8) · smoothing ตามเอกสาร
#[derive(Clone, Debug)]
pub struct PointerMover {
    cfg: PointerConfig,
    /// ระยะขยับของ tick ก่อน (ใช้ lerp กันสั่น)
    prev: Vec2,
    /// เศษพิกเซลที่ยังไม่ส่ง — กัน pointer ขยับช้ากว่าค่าจริงตอน dt สั้น
    residual: Vec2,
}

impl PointerMover {
    pub fn new(cfg: PointerConfig) -> Self {
        Self {
            cfg,
            prev: Vec2::ZERO,
            residual: Vec2::ZERO,
        }
    }

    pub fn config(&self) -> PointerConfig {
        self.cfg
    }

    /// เริ่มใหม่หลังสลับ mode (กัน pointer กระโดดจากค่า prev/residual เก่า)
    pub fn reset(&mut self) {
        self.prev = Vec2::ZERO;
        self.residual = Vec2::ZERO;
    }

    /// คืน `(dx, dy)` เป็น px ของหน้าจอ (y บวก = ลง)
    pub fn tick(&mut self, rs: Vec2, dt_ms: u32, precision: bool, turbo: bool) -> (i32, i32) {
        let cfg = self.cfg;
        let m = rs.magnitude();

        let mut speed_scale = 1.0;
        if precision {
            speed_scale *= cfg.precision;
        }
        if turbo {
            speed_scale *= cfg.turbo;
        }

        let target = if m < cfg.deadzone || m <= f32::EPSILON {
            Vec2::ZERO
        } else {
            let clamped = m.min(1.0);
            let s = (clamped - cfg.deadzone) / (1.0 - cfg.deadzone);
            let speed = cfg.vmax * s.powf(cfg.gamma) * speed_scale;
            let distance = speed * (dt_ms as f32 / 1000.0);
            Vec2::new(rs.x / m * distance, rs.y / m * distance)
        };

        // lerp(d_prev, d, smoothing) — 250 Hz
        let smooth = cfg.smoothing.clamp(0.0, 1.0);
        let smoothed = Vec2::new(
            self.prev.x + (target.x - self.prev.x) * smooth,
            self.prev.y + (target.y - self.prev.y) * smooth,
        );
        self.prev = smoothed;

        // จอย y ขึ้น → หน้าจอ y ลง
        self.residual.x += smoothed.x;
        self.residual.y -= smoothed.y;

        let dx = self.residual.x.trunc();
        let dy = self.residual.y.trunc();
        self.residual.x -= dx;
        self.residual.y -= dy;
        (dx as i32, dy as i32)
    }
}

impl Default for PointerMover {
    fn default() -> Self {
        Self::new(PointerConfig::default())
    }
}

/// Nav mode — pivot ของ HUD (พิกัดหน้าจอ)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PivotConfig {
    pub deadzone: f32,
    /// LS = หยาบ (px/s)
    pub coarse: f32,
    /// RS = ละเอียด (px/s)
    pub fine: f32,
    /// LT ค้าง = precision (×)
    pub precision: f32,
}

impl Default for PivotConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            coarse: 900.0,
            fine: 320.0,
            precision: 0.35,
        }
    }
}

/// แปลง LS/RS เป็นระยะขยับ pivot (px) — ไม่มี gamma เพราะเป็นการจัดตำแหน่ง UI
#[derive(Clone, Debug)]
pub struct PivotMover {
    cfg: PivotConfig,
}

impl PivotMover {
    pub fn new(cfg: PivotConfig) -> Self {
        Self { cfg }
    }

    pub fn config(&self) -> PivotConfig {
        self.cfg
    }

    /// คืน `(dx, dy)` เป็น px ของหน้าจอ (y บวก = ลง)
    pub fn tick(&mut self, ls: Vec2, rs: Vec2, dt_ms: u32, precision: bool) -> (f32, f32) {
        let cfg = self.cfg;
        let scale = if precision { cfg.precision } else { 1.0 };
        let dt = dt_ms as f32 / 1000.0;

        let axis = |value: f32| {
            if value.abs() < cfg.deadzone {
                0.0
            } else {
                value
            }
        };

        let dx = (axis(ls.x) * cfg.coarse + axis(rs.x) * cfg.fine) * scale * dt;
        let dy = (axis(ls.y) * cfg.coarse + axis(rs.y) * cfg.fine) * scale * dt;
        (dx, -dy)
    }
}

impl Default for PivotMover {
    fn default() -> Self {
        Self::new(PivotConfig::default())
    }
}

/// แกนที่ scroll (ใช้เลือกค่าคงที่ของ wheel)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

impl ScrollAxis {
    /// `WHEEL_DELTA` ของ Windows
    pub const NOTCH: i32 = 120;
}

/// ค่าคงที่ scroll ตาม §5
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollConfig {
    pub deadzone: f32,
    /// 1 notch ต่อ 40 ms
    pub notch_ms: u32,
    /// เริ่มเร่งหลังค้าง 500 ms
    pub fast_after_ms: u32,
    /// จำนวน notch ต่อรอบเมื่อเร่งแล้ว
    pub fast_notches: i32,
    /// LT + LS = 1 notch ต่อ 120 ms
    pub fine_notch_ms: u32,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            notch_ms: 40,
            fast_after_ms: 500,
            fast_notches: 3,
            fine_notch_ms: 120,
        }
    }
}

/// LS → wheel (แนวนอน/แนวตั้ง) — ใช้เฉพาะ Mouse mode
#[derive(Clone, Debug)]
pub struct ScrollEngine {
    cfg: ScrollConfig,
    held_ms: u32,
    accum_ms: u32,
    last: Option<(ScrollAxis, i8)>,
}

impl ScrollEngine {
    pub fn new(cfg: ScrollConfig) -> Self {
        Self {
            cfg,
            held_ms: 0,
            accum_ms: 0,
            last: None,
        }
    }

    pub fn config(&self) -> ScrollConfig {
        self.cfg
    }

    pub fn reset(&mut self) {
        self.held_ms = 0;
        self.accum_ms = 0;
        self.last = None;
    }

    /// คืน `(axis, delta)` เมื่อถึงรอบ — `None` = ยังไม่ scroll
    pub fn tick(&mut self, ls: Vec2, dt_ms: u32, precision: bool) -> Option<(ScrollAxis, i32)> {
        let m = ls.magnitude();
        if m < self.cfg.deadzone {
            self.reset();
            return None;
        }

        let axis = if ls.y.abs() >= ls.x.abs() {
            ScrollAxis::Vertical
        } else {
            ScrollAxis::Horizontal
        };
        let direction: i8 = match axis {
            // ↑ = ขึ้น (+WHEEL_DELTA) · → = ขวา (+)
            ScrollAxis::Vertical => {
                if ls.y > 0.0 {
                    1
                } else {
                    -1
                }
            }
            ScrollAxis::Horizontal => {
                if ls.x > 0.0 {
                    1
                } else {
                    -1
                }
            }
        };

        if self.last != Some((axis, direction)) {
            self.held_ms = 0;
            self.accum_ms = 0;
            self.last = Some((axis, direction));
        }
        self.held_ms = self.held_ms.saturating_add(dt_ms);
        self.accum_ms = self.accum_ms.saturating_add(dt_ms);

        let interval = if precision {
            self.cfg.fine_notch_ms
        } else {
            self.cfg.notch_ms
        };
        if self.accum_ms < interval {
            return None;
        }
        self.accum_ms -= interval;

        let notches = if !precision && self.held_ms >= self.cfg.fast_after_ms {
            self.cfg.fast_notches
        } else {
            1
        };
        Some((axis, i32::from(direction) * notches * ScrollAxis::NOTCH))
    }
}

impl Default for ScrollEngine {
    fn default() -> Self {
        Self::new(ScrollConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact() -> PointerConfig {
        PointerConfig {
            // 1.0 = ใช้ค่าเป้าหมายทันที (ไม่ smoothing) — ค่าจริงใช้ 0.5
            smoothing: 1.0,
            ..PointerConfig::default()
        }
    }

    #[test]
    fn deadzone_and_release_do_not_move_the_pointer() {
        let mut mover = PointerMover::new(exact());
        for _ in 0..30 {
            assert_eq!(mover.tick(Vec2::new(0.14, 0.0), 4, false, false), (0, 0));
        }
        // ปล่อย sticks หลังขยับแล้ว → residual ไม่ทำให้เกิดการขยับค้าง
        mover.tick(Vec2::new(1.0, 0.0), 100, false, false);
        let _ = mover.tick(Vec2::ZERO, 4, false, false);
        for _ in 0..100 {
            assert_eq!(mover.tick(Vec2::ZERO, 4, false, false), (0, 0));
        }
    }

    #[test]
    fn full_stick_uses_vmax_and_flips_y_for_the_screen() {
        let mut mover = PointerMover::new(exact());
        // 1 วินาทีที่ความเร็วสูงสุด = 1600 px
        let (dx, _) = mover.tick(Vec2::new(1.0, 0.0), 1000, false, false);
        assert_eq!(dx, 1600);

        let mut mover = PointerMover::new(exact());
        let (_, dy) = mover.tick(Vec2::new(0.0, 1.0), 1000, false, false);
        assert_eq!(dy, -1600, "จอยขึ้น = pointer ขึ้น (y ลด)");
    }

    #[test]
    fn gamma_makes_mid_stick_slower_than_linear() {
        let mut mover = PointerMover::new(exact());
        // (0.575 - 0.15) / 0.85 = 0.5 → 1600 * 0.5^1.7 ≈ 492 px/s
        let (dx, _) = mover.tick(Vec2::new(0.575, 0.0), 1000, false, false);
        assert!((480..=505).contains(&dx), "ได้ {dx}");
    }

    #[test]
    fn precision_and_turbo_scale_speed() {
        let cfg = exact();
        let mut fine = PointerMover::new(cfg);
        let (fine_dx, _) = fine.tick(Vec2::new(1.0, 0.0), 1000, true, false);
        assert_eq!(fine_dx, (cfg.vmax * cfg.precision) as i32);

        let mut turbo = PointerMover::new(cfg);
        let (turbo_dx, _) = turbo.tick(Vec2::new(1.0, 0.0), 1000, false, true);
        assert_eq!(turbo_dx, (cfg.vmax * cfg.turbo) as i32);
    }

    #[test]
    fn smoothing_ramps_instead_of_jumping() {
        let mut mover = PointerMover::new(PointerConfig::default());
        let first = mover.tick(Vec2::new(1.0, 0.0), 100, false, false).0;
        let second = mover.tick(Vec2::new(1.0, 0.0), 100, false, false).0;
        let third = mover.tick(Vec2::new(1.0, 0.0), 100, false, false).0;
        assert!(first > 0 && second > first && third > second);
        // steady state ≈ vmax * 0.1 s
        let settle = mover.tick(Vec2::new(1.0, 0.0), 100, false, false).0;
        assert!((140..=160).contains(&settle), "ได้ {settle}");
    }

    #[test]
    fn tiny_frames_accumulate_instead_of_losing_pixels() {
        let mut mover = PointerMover::new(exact());
        let mut total = 0;
        for _ in 0..250 {
            total += mover.tick(Vec2::new(0.3, 0.0), 4, false, false).0;
        }
        // 0.3 ที่ gamma 1.7 ≈ 0.176^1.7 * 1600 ≈ 93 px/s → ~93 px ใน 1 s
        assert!((80..=100).contains(&total), "ได้ {total}");
    }

    #[test]
    fn pivot_uses_coarse_and_fine_speeds() {
        let mut pivot = PivotMover::default();
        // LS เต็ม 1 s = 900 px ลงจอเมื่อดันขึ้น → y ติดลบ
        assert_eq!(
            pivot.tick(Vec2::new(0.0, 1.0), Vec2::ZERO, 1000, false),
            (0.0, -900.0)
        );
        assert_eq!(
            pivot.tick(Vec2::ZERO, Vec2::new(1.0, 0.0), 1000, false),
            (320.0, 0.0)
        );
        // LT precision
        let (dx, _) = pivot.tick(Vec2::ZERO, Vec2::new(1.0, 0.0), 1000, true);
        assert!((111.0..=113.0).contains(&dx), "ได้ {dx}");
    }

    #[test]
    fn pivot_deadzone_ignores_small_deflection() {
        let mut pivot = PivotMover::default();
        assert_eq!(
            pivot.tick(Vec2::new(0.1, 0.1), Vec2::new(0.1, -0.1), 16, false),
            (0.0, 0.0)
        );
    }

    #[test]
    fn scroll_waits_for_the_interval_and_speeds_up_after_500ms() {
        let mut scroll = ScrollEngine::default();
        assert_eq!(scroll.tick(Vec2::new(0.0, 0.9), 39, false), None);
        assert_eq!(
            scroll.tick(Vec2::new(0.0, 0.9), 1, false),
            Some((ScrollAxis::Vertical, 120))
        );
        // ค้างต่อ: ยัง 1 notch ก่อน 500 ms
        assert_eq!(
            scroll.tick(Vec2::new(0.0, 0.9), 40, false),
            Some((ScrollAxis::Vertical, 120))
        );
        // ดันค้างจนผ่าน 500 ms → รอบถัดไปเป็น 3 notch
        let mut last = 0;
        for _ in 0..13 {
            if let Some((_, delta)) = scroll.tick(Vec2::new(0.0, 0.9), 40, false) {
                last = delta;
            }
        }
        assert_eq!(last, 3 * 120);
    }

    #[test]
    fn scroll_horizontal_and_precision_are_supported() {
        let mut scroll = ScrollEngine::default();
        assert_eq!(
            scroll.tick(Vec2::new(0.9, 0.0), 40, false),
            Some((ScrollAxis::Horizontal, 120))
        );
        let mut scroll = ScrollEngine::default();
        // LT: 120 ms ต่อ notch
        assert_eq!(scroll.tick(Vec2::new(0.0, -0.9), 119, true), None);
        assert_eq!(
            scroll.tick(Vec2::new(0.0, -0.9), 1, true),
            Some((ScrollAxis::Vertical, -120))
        );
    }

    #[test]
    fn releasing_the_stick_resets_the_hold_timer() {
        let mut scroll = ScrollEngine::default();
        for _ in 0..20 {
            scroll.tick(Vec2::new(0.0, 0.9), 40, false);
        }
        scroll.tick(Vec2::ZERO, 40, false);
        // กลับมาดันใหม่ = เริ่มที่ 1 notch
        assert_eq!(
            scroll.tick(Vec2::new(0.0, 0.9), 40, false),
            Some((ScrollAxis::Vertical, 120))
        );
    }
}
