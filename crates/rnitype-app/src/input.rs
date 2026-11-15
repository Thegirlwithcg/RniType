//! แหล่งค่าจอย (PadSource)
//!
//! - [`GilrsSource`] = จอยจริงผ่าน gilrs (XInput บน Windows — ดู ADR-15)
//! - [`ScriptSource`] = สคริปต์จอยสำหรับ `--fake-pad` (ตรวจ runtime โดยไม่ต้องมีจอย)
//!
//! trait นี้ทำให้ loop ของแอปไม่ผูกกับ gilrs → ทดสอบ/log ได้เท่ากันทั้งสองทาง

use gilrs::{Axis, Button, Gamepad, GamepadId, Gilrs};
use rnitype_core::pad::{Buttons, PadState, Vec2};

pub trait PadSource: Send {
    /// ค่าจอย ณ เวลานั้น — `None` = ไม่มีจอยต่ออยู่
    fn sample(&mut self, now_ms: u64) -> Option<PadState>;
    /// ชื่อจอย (ว่าง = ไม่มี)
    fn name(&self) -> String;
}

/* ============================================================
   จอยจริง
   ============================================================ */

pub struct GilrsSource {
    gilrs: Gilrs,
    active: Option<GamepadId>,
    name: String,
}

impl GilrsSource {
    pub fn open() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|err| format!("เปิด gilrs ไม่ได้: {err}"))?;
        let mut source = Self {
            gilrs,
            active: None,
            name: String::new(),
        };
        source.choose_first();
        Ok(source)
    }

    fn choose_first(&mut self) {
        if self.active.is_some() {
            return;
        }
        let first = self
            .gilrs
            .gamepads()
            .map(|(id, pad)| (id, pad.name().to_string()))
            .next();
        if let Some((id, name)) = first {
            self.active = Some(id);
            self.name = name;
        }
    }

    /// เก็บ event ค้าง (กันบัฟเฟอร์บวม) + อัปเดตการเชื่อมต่อ
    fn pump(&mut self) {
        use gilrs::EventType;
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    if self.active.is_none() {
                        self.active = Some(event.id);
                        self.name = self.gilrs.gamepad(event.id).name().to_string();
                    }
                }
                EventType::Disconnected => {
                    if self.active == Some(event.id) {
                        self.active = None;
                        self.name.clear();
                    }
                }
                _ => {}
            }
        }

        // จอยที่ต่ออยู่ก่อนเปิดแอป / ตัวที่หายไปโดยไม่มี event
        if let Some(id) = self.active
            && !self.gilrs.gamepad(id).is_connected()
        {
            self.active = None;
            self.name.clear();
        }
        self.choose_first();
    }
}

impl PadSource for GilrsSource {
    fn sample(&mut self, _now_ms: u64) -> Option<PadState> {
        self.pump();
        let id = self.active?;
        Some(read_pad(&self.gilrs.gamepad(id)))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// แปลงค่าจอย gilrs เป็น [`PadState`] (คู่กับ `rnitype-probe` — probe คือแหล่งอ้างอิง)
fn read_pad(pad: &Gamepad) -> PadState {
    PadState {
        ls: Vec2::new(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY)),
        rs: Vec2::new(pad.value(Axis::RightStickX), pad.value(Axis::RightStickY)),
        lt_analog: pad.value(Axis::LeftZ).clamp(0.0, 1.0),
        rt_analog: pad.value(Axis::RightZ).clamp(0.0, 1.0),
        buttons: Buttons {
            a: pad.is_pressed(Button::South),
            b: pad.is_pressed(Button::East),
            x: pad.is_pressed(Button::West),
            y: pad.is_pressed(Button::North),
            lb: pad.is_pressed(Button::LeftTrigger),
            rb: pad.is_pressed(Button::RightTrigger),
            lt: pad.is_pressed(Button::LeftTrigger2),
            rt: pad.is_pressed(Button::RightTrigger2),
            ls_click: pad.is_pressed(Button::LeftThumb),
            rs_click: pad.is_pressed(Button::RightThumb),
            dpad_up: pad.is_pressed(Button::DPadUp),
            dpad_down: pad.is_pressed(Button::DPadDown),
            dpad_left: pad.is_pressed(Button::DPadLeft),
            dpad_right: pad.is_pressed(Button::DPadRight),
            view: pad.is_pressed(Button::Select),
            menu: pad.is_pressed(Button::Start),
        },
    }
}

/* ============================================================
   สคริปต์ (--fake-pad / test)
   ============================================================ */

/// จอยปลอมที่เล่นตามสคริปต์ (วินาที → PadState) วนซ้ำ
pub struct ScriptSource {
    steps: Vec<(u64, PadState)>,
    loop_ms: u64,
    name: String,
}

impl ScriptSource {
    pub fn new(loop_ms: u64, steps: Vec<(u64, PadState)>) -> Self {
        let mut steps = steps;
        steps.sort_by_key(|(at, _)| *at);
        Self {
            steps,
            loop_ms: loop_ms.max(1),
            name: "script".to_string(),
        }
    }

    /// ท่าพิมพ์: LS ไปกลุ่ม → RS เลือกช่อง → ปล่อย RS (commit) → ปล่อย LS
    ///
    /// 1 ตัวอักษร = 600 ms · ใช้กับ `--fake-pad` เพื่อดูทั้งท่อโดยไม่ต้องมีจอย
    pub fn typing_script(chars: &[(f32, f32)]) -> Self {
        const STEP_MS: u64 = 600;
        let mut steps = Vec::new();
        for (index, (x, y)) in chars.iter().enumerate() {
            let base = index as u64 * STEP_MS;
            let group = PadState {
                ls: Vec2::new(*x, *y),
                ..PadState::default()
            };
            let both = PadState {
                ls: Vec2::new(*x, *y),
                rs: Vec2::new(*x, *y),
                ..PadState::default()
            };
            steps.push((base, PadState::default()));
            steps.push((base + 100, group));
            steps.push((base + 200, both));
            steps.push((base + 320, group));
            steps.push((base + 460, PadState::default()));
        }
        Self::new(chars.len() as u64 * STEP_MS + 400, steps)
    }
}

impl PadSource for ScriptSource {
    fn sample(&mut self, now_ms: u64) -> Option<PadState> {
        let t = now_ms % self.loop_ms;
        let pad = self
            .steps
            .iter()
            .rev()
            .find(|(at, _)| *at <= t)
            .map(|(_, pad)| *pad)
            .unwrap_or_default();
        Some(pad)
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_replays_steps_and_loops() {
        let mut source = ScriptSource::typing_script(&[(0.0, 0.9)]);
        // 0 ms = พัก
        assert_eq!(source.sample(0).unwrap(), PadState::default());
        // 200 ms = ดัน LS+RS ขึ้น → กลุ่ม/ช่องแรก
        let at_pick = source.sample(250).unwrap();
        assert!(at_pick.ls.y > 0.5 && at_pick.rs.y > 0.5);
        // 460 ms = ปล่อยหมด
        assert_eq!(source.sample(500).unwrap(), PadState::default());
        // วนซ้ำ: 1000 ms = loop ใหม่ (1000 % 1000 = 0)
        assert_eq!(source.sample(1000).unwrap(), PadState::default());
    }

    #[test]
    fn script_has_a_stable_name() {
        let source = ScriptSource::typing_script(&[(0.0, 0.9)]);
        assert_eq!(source.name(), "script");
    }
}
