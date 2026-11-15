//! ค่าจอยที่ชั้นอื่นใช้ร่วมกัน — ชั้นนี้ไม่รู้จัก gilrs/SDL3

/// แกน 2 มิติ (x = ขวา, y = ขึ้น)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn magnitude(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// มุมเป็นองศา: 0 = ขึ้น, 90 = ขวา, 180 = ลง, 270 = ซ้าย
    pub fn angle_deg(self) -> f32 {
        if self.x == 0.0 && self.y == 0.0 {
            return 0.0;
        }
        self.x.atan2(self.y).to_degrees().rem_euclid(360.0)
    }
}

/// ปุ่มแบบเปิด/ปิด (analog ของ trigger อยู่ใน [`PadState`])
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub lb: bool,
    pub rb: bool,
    pub lt: bool,
    pub rt: bool,
    pub ls_click: bool,
    pub rs_click: bool,
    pub view: bool,
    pub menu: bool,
}

impl Buttons {
    pub fn any_pressed(self) -> bool {
        self.names().next().is_some()
    }

    /// ชื่อปุ่มที่กดอยู่ (ใช้ใน log/probe)
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        const TABLE: [(bool, &str); 16] = [
            (false, "DpadUp"),
            (false, "DpadDown"),
            (false, "DpadLeft"),
            (false, "DpadRight"),
            (false, "A"),
            (false, "B"),
            (false, "X"),
            (false, "Y"),
            (false, "LB"),
            (false, "RB"),
            (false, "LT"),
            (false, "RT"),
            (false, "LS"),
            (false, "RS"),
            (false, "View"),
            (false, "Menu"),
        ];
        let me = self;
        let flags = [
            me.dpad_up,
            me.dpad_down,
            me.dpad_left,
            me.dpad_right,
            me.a,
            me.b,
            me.x,
            me.y,
            me.lb,
            me.rb,
            me.lt,
            me.rt,
            me.ls_click,
            me.rs_click,
            me.view,
            me.menu,
        ];
        TABLE
            .iter()
            .map(|(_, name)| *name)
            .zip(flags)
            .filter_map(|(name, on)| on.then_some(name))
    }
}

/// สถานะจอย 1 เฟรม (หลัง normalize แล้ว)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadState {
    pub ls: Vec2,
    pub rs: Vec2,
    pub lt_analog: f32,
    pub rt_analog: f32,
    pub buttons: Buttons,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_is_compass_degrees() {
        assert!((Vec2::new(0.0, 1.0).angle_deg() - 0.0).abs() < 1e-4);
        assert!((Vec2::new(1.0, 0.0).angle_deg() - 90.0).abs() < 1e-4);
        assert!((Vec2::new(0.0, -1.0).angle_deg() - 180.0).abs() < 1e-4);
        assert!((Vec2::new(-1.0, 0.0).angle_deg() - 270.0).abs() < 1e-4);
    }

    #[test]
    fn button_names_follow_pressed_flags() {
        let b = Buttons { a: true, rb: true, ..Buttons::default() };
        let names: Vec<_> = b.names().collect();
        assert_eq!(names, vec!["A", "RB"]);
        assert!(Buttons::default().names().next().is_none());
    }
}
