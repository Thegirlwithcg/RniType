//! Windows sender: แปลง plan ([`crate::plan::Input`]) เป็น `SendInput`
//!
//! `unsafe` ทั้งหมดของ crate อยู่ในไฟล์นี้ — มีแค่ 2 จุด คือ `SendInput` กับ
//! `GetSystemMetrics` (ทั้งคู่เป็น FFI ล้วน ไม่มี state ที่เราต้องดูแล)
//!
//! ยังไม่ทำใน slice นี้ (ยกไป P3 slice 2): scancode/VK path ด้วย `VkKeyScanEx`
//! สำหรับ IME, exclusion list ต่อโปรเซส, foreground check, admin/UIPI detection

use std::mem::size_of;

use rnitype_core::{Action, MouseButton};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::plan::{self, HeldState, Input, ScreenRect};
use crate::{InjectError, Injector};

impl ScreenRect {
    /// กรอบ virtual desktop ทั้งหมด (หลายจอ = กรอบรวม) จาก `GetSystemMetrics`
    pub fn virtual_desktop() -> Self {
        // SAFETY: GetSystemMetrics อ่านค่าจาก OS เท่านั้น (ไม่มี pointer/ownership)
        let (x, y, width, height) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// injector จริงบน Windows
///
/// - `Char` → Unicode path (`KEYEVENTF_UNICODE`) ตาม Spike C
/// - `Named` → virtual-key
/// - modifier ที่ค้างถูกจำไว้ และ `release_all()` ปล่อยให้ครบ
/// - ถ้า `SendInput` รับไปแค่บางส่วน จะจำ **prefix ที่ส่งจริง** ไว้ด้วย
///   (กันคีย์ค้างตอน `release_all()`)
///
/// ยังต้องเรียก `release_all()` เองตอนจอยหลุด/ปิดแอป — `Drop` เป็นแค่ safety net
/// ที่พยายามปล่อยให้ (ไม่สน error) และ**รันบน thread ที่ drop injector**
#[derive(Debug, Default)]
pub struct SendInputInjector {
    held: HeldState,
}

impl SendInputInjector {
    pub fn new() -> Self {
        Self::default()
    }

    /// สถานะที่กดค้างอยู่ (ไว้ debug/HUD)
    pub fn held(&self) -> &HeldState {
        &self.held
    }
}

impl Drop for SendInputInjector {
    fn drop(&mut self) {
        // RAII safety net: best-effort, กลืน error (ปกติ runtime เรียก release_all เองก่อนแล้ว)
        let _ = self.release_all();
    }
}

impl Injector for SendInputInjector {
    fn apply(&mut self, action: Action) -> Result<(), InjectError> {
        // query ทุกครั้ง: จอเพิ่ม/ถอด/ย้าย ระหว่างรันแล้วยังถูก
        let inputs = plan::plan(action, ScreenRect::virtual_desktop());
        if inputs.is_empty() {
            // Command/SetMode/SetLang/SetLayer/MovePivot/Noop = แอปจัดการเอง
            return Ok(());
        }
        match send(&inputs) {
            Ok(sent) => {
                self.held.track(&inputs[..sent as usize]);
                Ok(())
            }
            Err((sent, err)) => {
                // partial: จำ prefix ที่ OS รับไปแล้ว เพื่อให้ release_all ปล่อยคีย์นั้นได้
                self.held.track(&inputs[..sent as usize]);
                Err(err)
            }
        }
    }

    fn release_all(&mut self) -> Result<(), InjectError> {
        let inputs = self.held.release_plan();
        if inputs.is_empty() {
            return Ok(());
        }
        // partial failure = เก็บ state ไว้ให้ caller ลองใหม่ (keyup ซ้ำไม่เป็นอันตราย)
        send(&inputs).map_err(|(_, err)| err)?;
        self.held.clear();
        Ok(())
    }
}

/// ส่ง plan ทั้งก้อนใน `SendInput` ครั้งเดียว (ลำดับภายในคงเดิม)
///
/// คืนจำนวน event ที่ OS รับจริงเมื่อสำเร็จ; ถ้าล้มเหลวคืน `(sent, error)`
/// เพื่อให้ caller จำ prefix ที่ถูก insert ไปแล้วได้
fn send(inputs: &[Input]) -> Result<u32, (u32, InjectError)> {
    let win_inputs: Vec<INPUT> = inputs.iter().map(to_win_input).collect();
    let count = win_inputs.len();
    let size = size_of::<INPUT>() as i32;

    // SAFETY: win_inputs มีสมาชิก count ตัวจริง และ size ตรงกับ INPUT ของ windows-sys;
    // SendInput อ่าน array ระหว่างการเรียกเท่านั้น (ไม่เก็บ pointer ไว้)
    let sent = unsafe { SendInput(count as u32, win_inputs.as_ptr(), size) };
    if sent != count as u32 {
        let err = std::io::Error::last_os_error();
        let code = err.raw_os_error().unwrap_or(0);
        let sent = sent.min(count as u32);
        return Err((
            sent,
            InjectError::new(format!(
                "SendInput ส่งได้ {sent}/{count} events (Win32 {code}: {err})"
            )),
        ));
    }
    Ok(sent)
}

fn to_win_input(input: &Input) -> INPUT {
    match *input {
        Input::Unicode { unit, up } => INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    // Unicode path: wVk = 0, wScan = code unit
                    wVk: 0,
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | keyup_flag(up),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        Input::VirtualKey { vk: code, up } => INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: code,
                    wScan: 0,
                    dwFlags: keyup_flag(up),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        Input::MouseMove { dx, dy } => mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE),
        Input::MouseMoveAbs { x, y } => mouse_input(
            x,
            y,
            0,
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        ),
        Input::MouseButton { button, down } => mouse_input(0, 0, 0, button_flags(button, down)),
        Input::Wheel { delta } => mouse_input(0, 0, delta as u32, MOUSEEVENTF_WHEEL),
        Input::HWheel { delta } => mouse_input(0, 0, delta as u32, MOUSEEVENTF_HWHEEL),
    }
}

fn keyup_flag(up: bool) -> u32 {
    if up { KEYEVENTF_KEYUP } else { 0 }
}

fn mouse_input(dx: i32, dy: i32, mouse_data: u32, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn button_flags(button: MouseButton, down: bool) -> u32 {
    match (button, down) {
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::vk;

    // helper: อ่าน union field ที่รู้ว่าถูกตั้งไว้ (ไม่เรียก OS)
    fn keyboard(input: &Input) -> KEYBDINPUT {
        // SAFETY: to_win_input สร้าง INPUT แบบ keyboard (r#type = INPUT_KEYBOARD) เท่านั้น
        unsafe { to_win_input(input).Anonymous.ki }
    }

    fn mouse(input: &Input) -> MOUSEINPUT {
        // SAFETY: เรียกเฉพาะ input ที่ to_win_input สร้างเป็น INPUT_MOUSE
        unsafe { to_win_input(input).Anonymous.mi }
    }

    #[test]
    fn unicode_char_uses_keyeventf_unicode() {
        let key_down = keyboard(&Input::Unicode {
            unit: 0x0E01,
            up: false,
        });
        assert_eq!(key_down.wVk, 0);
        assert_eq!(key_down.wScan, 0x0E01);
        assert_eq!(key_down.dwFlags, KEYEVENTF_UNICODE);

        let key_up = keyboard(&Input::Unicode {
            unit: 0x0E01,
            up: true,
        });
        assert_eq!(key_up.dwFlags, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
    }

    #[test]
    fn named_key_uses_virtual_key() {
        let input = Input::VirtualKey {
            vk: vk::RETURN,
            up: false,
        };
        let key = keyboard(&input);
        assert_eq!(key.wVk, vk::RETURN);
        assert_eq!(key.wScan, 0);
        assert_eq!(key.dwFlags, 0);

        let key = keyboard(&Input::VirtualKey {
            vk: vk::RETURN,
            up: true,
        });
        assert_eq!(key.dwFlags, KEYEVENTF_KEYUP);
    }

    #[test]
    fn relative_move_uses_plain_move() {
        let input = mouse(&Input::MouseMove { dx: -4, dy: 9 });
        assert_eq!((input.dx, input.dy), (-4, 9));
        assert_eq!(input.dwFlags, MOUSEEVENTF_MOVE);
        assert_eq!(input.mouseData, 0);
    }

    #[test]
    fn absolute_move_uses_virtual_desktop() {
        let input = mouse(&Input::MouseMoveAbs {
            x: 65_535,
            y: 32_767,
        });
        assert_eq!((input.dx, input.dy), (65_535, 32_767));
        assert_eq!(
            input.dwFlags,
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK
        );
    }

    #[test]
    fn buttons_map_to_down_and_up_flags() {
        let cases = [
            (MouseButton::Left, true, MOUSEEVENTF_LEFTDOWN),
            (MouseButton::Left, false, MOUSEEVENTF_LEFTUP),
            (MouseButton::Right, true, MOUSEEVENTF_RIGHTDOWN),
            (MouseButton::Right, false, MOUSEEVENTF_RIGHTUP),
            (MouseButton::Middle, true, MOUSEEVENTF_MIDDLEDOWN),
            (MouseButton::Middle, false, MOUSEEVENTF_MIDDLEUP),
        ];
        for (button, down, flags) in cases {
            let input = mouse(&Input::MouseButton { button, down });
            assert_eq!(input.dwFlags, flags, "{button:?} down={down}");
            assert_eq!((input.dx, input.dy), (0, 0));
        }
    }

    #[test]
    fn wheel_carries_delta_as_signed_u32() {
        let input = mouse(&Input::Wheel { delta: -120 });
        assert_eq!(input.mouseData, (-120i32) as u32);
        assert_eq!(input.dwFlags, MOUSEEVENTF_WHEEL);

        let input = mouse(&Input::HWheel { delta: 120 });
        assert_eq!(input.mouseData, 120);
        assert_eq!(input.dwFlags, MOUSEEVENTF_HWHEEL);
    }

    #[test]
    fn new_injector_holds_nothing() {
        assert!(SendInputInjector::new().held().is_empty());
    }
}
