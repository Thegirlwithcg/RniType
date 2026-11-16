//! Windows sender: แปลง plan ([`crate::plan::Input`]) เป็น `SendInput`
//!
//! `unsafe` ทั้งหมดของ crate อยู่ในไฟล์นี้ — มีแค่ 2 จุด คือ `SendInput` กับ
//! `GetSystemMetrics` (ทั้งคู่เป็น FFI ล้วน ไม่มี state ที่เราต้องดูแล)
//!
//! ยังไม่ทำใน slice นี้ (ยกไป P3 slice 2): scancode/VK path ด้วย `VkKeyScanEx`
//! สำหรับ IME, exclusion list ต่อโปรเซส, foreground check, admin/UIPI detection

use std::mem::size_of;

use rnitype_core::{Action, Key, MouseButton};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    MapVirtualKeyExW, SendInput, VkKeyScanExW, GetKeyboardLayout, HKL,
};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowThreadProcessId,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::plan::{self, HeldState, Input, ScanChord, ScreenRect};
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

/// ตำแหน่ง pointer ปัจจุบันของ Windows (พิกัดหน้าจอ/physical px)
pub fn cursor_position() -> Option<(f32, f32)> {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: GetCursorPos เขียนลง struct ที่เราถืออยู่เท่านั้น
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some((point.x as f32, point.y as f32))
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
#[derive(Debug)]
pub struct SendInputInjector {
    held: HeldState,
    /// เปิด scancode path อัตโนมัติเมื่อ layout เป้าหมายพิมพ์ตัวนั้นได้ (ADR-23)
    scancode: bool,
}

impl Default for SendInputInjector {
    fn default() -> Self {
        Self {
            held: HeldState::default(),
            scancode: true,
        }
    }
}

impl SendInputInjector {
    pub fn new() -> Self {
        Self::default()
    }

    /// ปิด/เปิด scancode path (ปิด = Unicode ทุกตัวอักษร เหมือน P3 slice 1)
    pub fn with_scancode(mut self, enabled: bool) -> Self {
        self.scancode = enabled;
        self
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
        let screen = ScreenRect::virtual_desktop();
        let chord = if self.scancode {
            scancode_chord(action)
        } else {
            None
        };
        let inputs = plan::plan_scancode(action, screen, chord);
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

/// เลือกใช้ scancode สำหรับ `Key::Char` เมื่อ layout เป้าหมายพิมพ์ตัวนั้นได้
///
/// - Ctrl/Alt chord → คงไว้ที่ plan เดิม (Ctrl+ASCII = VK)
/// - ตัวอักษรนอก BMP / layout พิมพ์ไม่ได้ → `None` = Unicode fallback
fn scancode_chord(action: Action) -> Option<ScanChord> {
    let Action::Key(press) = action else {
        return None;
    };
    if press.ctrl || press.alt {
        return None;
    }
    let Key::Char(ch) = press.key else {
        return None;
    };
    scan_chord_for(ch, foreground_layout())
}

/// HKL ของ thread ที่เป็นเจ้าของหน้าต่าง foreground (ถ้าไม่มี → thread เรา)
fn foreground_layout() -> HKL {
    // SAFETY: API ทั้งคู่อ่านค่าจาก OS เท่านั้น
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            if tid != 0 {
                let hkl = GetKeyboardLayout(tid);
                if !hkl.is_null() {
                    return hkl;
                }
            }
        }
        GetKeyboardLayout(0)
    }
}

/// แปลงอักขระเป็น `(scancode, ต้อง Shift?)` ใน layout ที่กำหนด
///
/// ใช้ `VkKeyScanExW` (char → VK + shift state) แล้ว `MapVirtualKeyExW` (VK → scan)
/// คืน `None` เมื่อ layout พิมพ์ตัวนั้นไม่ได้ หรือต้องใช้ Ctrl/AltGr
fn scan_chord_for(ch: char, hkl: HKL) -> Option<ScanChord> {
    let code = u32::from(ch);
    if code > 0xFFFF {
        return None;
    }
    // SAFETY: ทั้งสองฟังก์ชันเป็น query ล้วน ไม่เก็บ pointer
    unsafe {
        let mapped = VkKeyScanExW(code as u16, hkl);
        if mapped == -1 {
            return None;
        }
        let value = mapped as u32;
        let vk = value & 0xFF;
        let state = (value >> 8) & 0xFF;
        if state & 0b110 != 0 {
            // Ctrl / AltGr chord — ปล่อยให้ Unicode path จัดการ
            return None;
        }
        let scan = MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, hkl) as u16;
        if scan == 0 {
            return None;
        }
        Some(ScanChord {
            scan,
            extended: is_extended_vk(vk as u16),
            shift: state & 1 != 0,
        })
    }
}

/// VK ที่ต้องส่ง `KEYEVENTF_EXTENDEDKEY` (ปุ่มลูกศร/numpad บางตัว)
fn is_extended_vk(vk: u16) -> bool {
    matches!(
        vk,
        0x21..=0x28 // PageUp/Down, End, Home, Left, Up, Right, Down
            | 0x2D | 0x2E // Insert, Delete
            | 0x6F // Divide
            | 0x90 // NumLock
            | 0xA3 | 0xA5 // Right Ctrl, Right Alt
    )
}

/// ส่ง plan ทั้งก้อนใน `SendInput` ครั้งเดียว (ลำดับภายในคงเดิม)
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
        Input::ScanCode {
            scan,
            extended,
            up,
        } => INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: scan,
                    dwFlags: KEYEVENTF_SCANCODE
                        | if extended { KEYEVENTF_EXTENDEDKEY } else { 0 }
                        | keyup_flag(up),
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

    /* ---------------- P3 slice 2: scancode path ---------------- */

    #[test]
    fn scancode_input_sets_the_scan_flag() {
        let down = keyboard(&Input::ScanCode {
            scan: 0x1E,
            extended: false,
            up: false,
        });
        assert_eq!(down.wVk, 0);
        assert_eq!(down.wScan, 0x1E);
        assert_eq!(down.dwFlags, KEYEVENTF_SCANCODE);

        let up = keyboard(&Input::ScanCode {
            scan: 0x1E,
            extended: false,
            up: true,
        });
        assert_eq!(up.dwFlags, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);

        let extended = keyboard(&Input::ScanCode {
            scan: 0x4B,
            extended: true,
            up: false,
        });
        assert_eq!(
            extended.dwFlags,
            KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY
        );
    }

    fn load_layout(id: &str) -> HKL {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::LoadKeyboardLayoutW;
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: ส่ง string ที่มี null terminator; ไม่ activate layout
        unsafe { LoadKeyboardLayoutW(wide.as_ptr(), 0) }
    }

    #[test]
    fn scan_chord_maps_english_letters_and_shift() {
        let en = load_layout("00000409");
        if en.is_null() {
            return; // เครื่องนี้โหลด layout EN ไม่ได้ — ข้าม (ไม่ใช่ failure ของ logic)
        }
        let upper = scan_chord_for('A', en).expect("EN ต้องพิมพ์ A ได้");
        assert_eq!(upper.shift, true);
        assert_eq!(upper.scan, 0x1E, "A = physical key ที่ scan 0x1E");

        let lower = scan_chord_for('a', en).expect("EN ต้องพิมพ์ a ได้");
        assert_eq!(lower.shift, false);
        assert_eq!(lower.scan, upper.scan);
    }

    #[test]
    fn scan_chord_respects_the_target_layout() {
        let en = load_layout("00000409");
        let th = load_layout("0000041E");
        if en.is_null() || th.is_null() {
            return;
        }
        // ไทย 'ก' มีใน layout ไทย แต่ไม่มีใน EN → fallback Unicode เมื่อเป้าหมายเป็น EN
        assert!(scan_chord_for('ก', th).is_some());
        assert_eq!(scan_chord_for('ก', en), None);
    }

    #[test]
    fn foreground_layout_is_always_available() {
        assert!(!foreground_layout().is_null());
    }

    #[test]
    fn scancode_is_not_used_for_ctrl_chords_or_named_keys() {
        use rnitype_core::{Key as CoreKey, KeyPress};

        let ctrl_c = Action::Key(KeyPress {
            key: CoreKey::Char('c'),
            shift: false,
            ctrl: true,
            alt: false,
        });
        assert_eq!(scancode_chord(ctrl_c), None);

        let tab = Action::Key(KeyPress {
            key: CoreKey::Named(rnitype_core::NamedKey::Tab),
            shift: false,
            ctrl: false,
            alt: false,
        });
        assert_eq!(scancode_chord(tab), None);
    }
}
