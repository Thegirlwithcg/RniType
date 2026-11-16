//! Windows output layer
//!
//! P1: สัญญา `Injector` + `NullInjector` สำหรับ test (ไม่มี `unsafe`)
//! P3 slice 1: [`plan`] (pure) + [`SendInputInjector`] (Unicode/mouse บน Windows)
//! P5a: ตรวจ layout + สร้าง keymap จาก QWERTY
//!
//! `unsafe` ทั้งหมดอยู่ใน `sendinput.rs` (FFI 2 จุด) — ที่เหลือ test ได้โดยไม่ต้องมี Windows

use rnitype_core::Action;

pub mod plan;

#[cfg(windows)]
mod sendinput;

#[cfg(windows)]
pub use sendinput::{SendInputInjector, cursor_position};

/// ตำแหน่ง pointer ปัจจุบัน (พิกัดหน้าจอ) — ใช้ตั้งจุดเริ่มของ HUD ในโหมด Mouse
///
/// ฝั่ง Windows อยู่ที่ [`sendinput`]; ที่อื่นไม่มี pointer ของระบบ → `None`
#[cfg(not(windows))]
pub fn cursor_position() -> Option<(f32, f32)> {
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InjectError(pub String);

impl std::fmt::Display for InjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InjectError {}

impl InjectError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

/// ชั้นเดียวที่ core/HUD รู้จัก — แปลง `Action` เป็นผลข้างเคียงของ OS
///
/// สัญญา:
/// - `apply` ต้องไม่ panic และคืน error แทน
/// - `release_all` ต้องปล่อย modifier/ปุ่มที่ค้างทั้งหมด (เรียกเมื่อจอยหลุด/ปิดแอป)
pub trait Injector {
    fn apply(&mut self, action: Action) -> Result<(), InjectError>;

    fn release_all(&mut self) -> Result<(), InjectError> {
        Ok(())
    }
}

/// ใช้ใน unit test และตอนที่ยังไม่มีชั้น Windows
#[derive(Default, Debug)]
pub struct NullInjector {
    pub sent: Vec<Action>,
}

impl Injector for NullInjector {
    fn apply(&mut self, action: Action) -> Result<(), InjectError> {
        self.sent.push(action);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnitype_core::{Key, KeyPress};

    #[test]
    fn null_injector_records_actions_in_order() {
        let mut out = NullInjector::default();
        out.apply(Action::Key(KeyPress {
            key: Key::Char('a'),
            shift: false,
            ctrl: false,
            alt: false,
        }))
        .unwrap();
        out.apply(Action::MouseMove { dx: 3, dy: -1 }).unwrap();

        assert_eq!(out.sent.len(), 2);
        assert!(matches!(out.sent[0], Action::Key(_)));
        assert!(matches!(out.sent[1], Action::MouseMove { dx: 3, dy: -1 }));
    }
}
