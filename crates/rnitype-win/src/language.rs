//! ตรวจ layout/IME ที่ติดตั้งใน Windows — ใช้เติมแท็บ Languages (ADR-14)
//!
//! - [`language_code`] / [`classify`] เป็น policy ล้วน → test ได้ทุกเครื่อง
//! - [`installed_languages`] อ่าน `GetKeyboardLayoutList` + `GetLocaleInfoW` (FFI)
//!
//! `code` = รหัสภาษาหลัก (`en`, `th`, `ja`, …) · `klid` = layout id ของ Windows
//! (`00000409`) — ใช้ `klid` เป็น id ที่ไม่ซ้ำเมื่อมี layout เดียวกันหลายตัว

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;

/// ชนิดของภาษา — ตรงกับ ADR-14
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageKind {
    /// map ตัวอักษรเป็น 8×8 (QWERTY หรือตารางที่ออกแบบ)
    Layout,
    /// ส่ง physical key ให้ Windows IME ประกอบเอง (HUD แสดง QWERTY + badge IME)
    Ime,
}

impl LanguageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LanguageKind::Layout => "layout",
            LanguageKind::Ime => "ime",
        }
    }
}

/// ภาษาที่ตรวจพบ — ตรงกับแถวใน Settings → Languages
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageInfo {
    /// รหัสภาษาหลัก (`en`, `th`, …) — ค่าที่ไม่รู้จักใช้ `0xNNNN`
    pub code: String,
    /// layout id ของ Windows (`00000409`)
    pub klid: String,
    /// ชื่อที่อ่านได้ (อังกฤษเสมอ — UI เป็นภาษาอังกฤษ)
    pub label: String,
    pub kind: LanguageKind,
}

impl LanguageInfo {
    /// ภาษานี้พิมพ์ด้วยตาราง 8×8 ได้แล้วหรือยัง
    ///
    /// ตอนนี้ core มีตาราง EN/TH (ADR-12) — ภาษาอื่นจะแสดงในรายการแต่ปิด toggle
    /// จนกว่าจะมี universal QWERTY mapping (ADR-14)
    pub fn typing_supported(&self) -> bool {
        matches!(self.code.as_str(), "en" | "th")
    }
}

/// รหัสภาษาหลักจาก Windows language id (low word ของ HKL)
pub fn language_code(lang_id: u16) -> String {
    match lang_id & 0x03FF {
        0x09 => "en".to_string(),
        0x1E => "th".to_string(),
        0x19 => "ru".to_string(),
        0x08 => "el".to_string(),
        0x11 => "ja".to_string(),
        0x12 => "ko".to_string(),
        0x04 => "zh".to_string(),
        other => format!("0x{other:04X}"),
    }
}

/// ชนิดภาษา: JA/ZH/KO = IME · อื่น = layout (ADR-14 §1)
pub fn classify(code: &str) -> LanguageKind {
    match code {
        "ja" | "ko" | "zh" => LanguageKind::Ime,
        _ => LanguageKind::Layout,
    }
}

/// ค่า "ภาษาที่พิมพ์ได้จริง" ของภาษาเดียว — ใช้กับ runtime
pub fn typing_langs() -> [&'static str; 2] {
    ["en", "th"]
}

/// layout ที่ติดตั้งทั้งหมด — เรียงตามที่ Windows คืน (ค่าเริ่มต้นของผู้ใช้มาก่อน)
#[cfg(windows)]
pub fn installed_languages() -> Vec<LanguageInfo> {
    use windows_sys::Win32::Globalization::{GetLocaleInfoW, LOCALE_SENGLISHDISPLAYNAME};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayoutList, HKL};

    // SAFETY: เรียก API แบบ query; buffer เป็นของเราทั้งหมด
    unsafe {
        let count = GetKeyboardLayoutList(0, std::ptr::null_mut());
        if count <= 0 {
            return Vec::new();
        }
        let mut layouts: Vec<HKL> = vec![std::ptr::null_mut(); count as usize];
        let got = GetKeyboardLayoutList(count, layouts.as_mut_ptr());
        if got <= 0 {
            return Vec::new();
        }
        layouts.truncate(got as usize);

        let mut out: Vec<LanguageInfo> = Vec::new();
        for hkl in layouts {
            // low word ของ HKL = language identifier
            let lang_id = (hkl as usize as u32 & 0xFFFF) as u16;
            let code = language_code(lang_id);
            // `GetKeyboardLayoutNameW` คืน layout ของ thread ที่เรียก (ไม่ใช่ HKL ที่ส่ง)
            // → ใช้ lang id เป็น id แทน (พอสำหรับ dedupe/แสดงผล; layout variant เดียวกันยุบรวม)
            let klid = format!("{lang_id:08X}");
            if out.iter().any(|item| item.klid == klid) {
                continue;
            }

            // MAKELCID(lang_id, SORT_DEFAULT) = lang_id (sort 0)
            let mut label = [0u16; 128];
            let len = GetLocaleInfoW(
                lang_id as u32,
                LOCALE_SENGLISHDISPLAYNAME,
                label.as_mut_ptr(),
                label.len() as i32,
            );
            let label = if len > 0 {
                let end = label
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(label.len());
                OsString::from_wide(&label[..end])
                    .to_string_lossy()
                    .into_owned()
            } else {
                klid.clone()
            };

            out.push(LanguageInfo {
                kind: classify(&code),
                code,
                klid,
                label,
            });
        }
        out
    }
}

#[cfg(not(windows))]
pub fn installed_languages() -> Vec<LanguageInfo> {
    vec![LanguageInfo {
        code: "en".into(),
        klid: "00000409".into(),
        label: "English (United States)".into(),
        kind: LanguageKind::Layout,
    }]
}

/* ============================================================
   Universal QWERTY mapping (ADR-28)
   ============================================================ */

#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::HKL;

/// สร้างตาราง 8×8 จาก layout ของ Windows โดยยึด **ตำแหน่ง QWERTY** ของตาราง EN
///
/// - เอาตัวอักษรในตาราง EN (default) → หา VK/Shift ด้วย layout EN (`VkKeyScanExW`)
/// - แปลง VK+Shift เป็นตัวอักษรของ layout เป้าหมาย (`ToUnicodeEx`)
/// - ช่องที่แปลงไม่ได้ (dead key / ต้อง Ctrl/AltGr) คงตัวอักษร EN ไว้
/// - ปุ่ม named ใน layer 2 คงเดิม (F1, Tab, …)
///
/// คืน `None` เมื่อโหลด layout ไม่ได้ (ไม่มีในเครื่อง)
#[cfg(windows)]
pub fn build_keymap(klid: &str) -> Option<rnitype_core::keymap::Keymap> {
    use rnitype_core::keymap::{Keymap, Lang};
    use rnitype_core::Key;

    let target = load_layout(klid)?;
    let reference = load_layout("00000409")?;
    // กัน Windows ที่ยอมโหลด KLID แปลก ๆ แล้วคืน layout อื่น: lang id ต้องตรง
    if let Some(expected) = klid_lang_id(klid) {
        let actual = (target as usize as u32 & 0xFFFF) as u16;
        if actual != expected {
            return None;
        }
    }
    let builtin = Keymap::builtin(Lang::En);
    let mut keymap = builtin.clone();

    for group in 0..8 {
        for sector in 0..8 {
            if let Key::Char(ch) = builtin.cells[group][sector]
                && let Some(mapped) = map_char(ch, reference, target)
            {
                keymap.cells[group][sector] = Key::Char(mapped);
            }
            if let Some(Key::Char(ch)) = builtin.cells2[group][sector]
                && let Some(mapped) = map_char(ch, reference, target)
            {
                keymap.cells2[group][sector] = Some(Key::Char(mapped));
            }
        }
    }

    Some(keymap)
}

#[cfg(not(windows))]
pub fn build_keymap(_klid: &str) -> Option<rnitype_core::keymap::Keymap> {
    None
}

/// 4 หลักท้ายของ KLID = language id (`0000041E` → `0x041E`)
fn klid_lang_id(klid: &str) -> Option<u16> {
    let tail = klid.get(klid.len().checked_sub(4)?..)?;
    u16::from_str_radix(tail, 16).ok()
}

#[cfg(windows)]
fn load_layout(klid: &str) -> Option<HKL> {    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{KLF_NOTELLSHELL, LoadKeyboardLayoutW};

    let wide: Vec<u16> = klid.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: ส่ง wide string ที่ null-terminate; ไม่ activate layout
    let hkl = unsafe { LoadKeyboardLayoutW(wide.as_ptr(), KLF_NOTELLSHELL) };
    (!hkl.is_null()).then_some(hkl)
}

/// แปลงตัวอักษร EN 1 ตัว → ตัวอักษรที่คีย์แป้นเดียวกันให้ใน layout เป้าหมาย
#[cfg(windows)]
fn map_char(ch: char, reference: HKL, target: HKL) -> Option<char> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MAPVK_VK_TO_VSC, MapVirtualKeyExW, ToUnicodeEx, VkKeyScanExW,
    };

    if u32::from(ch) > 0xFFFF {
        return None;
    }
    // SAFETY: API ทั้งหมดเป็น query; buffer เป็นของเรา
    unsafe {
        let mapped = VkKeyScanExW(ch as u16, reference);
        if mapped == -1 {
            return None;
        }
        let value = mapped as u16;
        let vk = u32::from(value & 0xFF);
        let state = (value >> 8) as u8;
        if state & 0b110 != 0 {
            // Ctrl / AltGr chord — ไม่ใช้ในตาราง 8×8
            return None;
        }

        let scan = MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, reference);
        let mut keys = [0u8; 256];
        if state & 1 != 0 {
            keys[0x10] = 0x80; // VK_SHIFT
        }

        let mut buffer = [0u16; 8];
        let mut len = ToUnicodeEx(
            vk,
            scan,
            keys.as_ptr(),
            buffer.as_mut_ptr(),
            8,
            0,
            target,
        );
        // dead key: ToUnicodeEx คืน 0 และทิ้ง composed state → เรียกซ้ำเพื่อเคลียร์
        if len == 0 {
            len = ToUnicodeEx(vk, scan, keys.as_ptr(), buffer.as_mut_ptr(), 8, 0, target);
        }
        match len {
            1 => char::from_u32(u32::from(buffer[0])),
            2 => {
                let high = u32::from(buffer[0]);
                let low = u32::from(buffer[1]);
                if (0xD800..=0xDBFF).contains(&high) && (0xDC00..=0xDFFF).contains(&low) {
                    char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
                } else {
                    // ผลลัพธ์ 2 หน่วยแต่ไม่ใช่ surrogate pair (base + combining)
                    // → ไม่ใช้ เพื่อไม่ให้ได้ตัวอักษรขาดวรรณยุกต์
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_follow_windows_ids() {
        assert_eq!(language_code(0x0409), "en");
        assert_eq!(language_code(0x041E), "th");
        assert_eq!(language_code(0x0411), "ja");
        assert_eq!(language_code(0x0412), "ko");
        assert_eq!(language_code(0x0404), "zh");
        assert_eq!(language_code(0x0419), "ru");
        assert_eq!(language_code(0x0408), "el");
        // sublanguage ไม่มีผล
        assert_eq!(language_code(0x0809), "en");
        assert_eq!(language_code(0x1234), "0x0234");
    }

    #[test]
    fn ime_languages_are_classified_as_ime() {
        assert_eq!(classify("ja"), LanguageKind::Ime);
        assert_eq!(classify("zh"), LanguageKind::Ime);
        assert_eq!(classify("ko"), LanguageKind::Ime);
        assert_eq!(classify("en"), LanguageKind::Layout);
        assert_eq!(classify("th"), LanguageKind::Layout);
    }

    #[cfg(windows)]
    #[test]
    fn english_layout_keymap_matches_the_builtin_table() {
        use rnitype_core::Key;

        let Some(keymap) = build_keymap("00000409") else {
            return; // เครื่องนี้โหลด EN layout ไม่ได้
        };
        assert_eq!(keymap.cells[0][0], Key::Char('a'));
        assert_eq!(keymap.cells[0][1], Key::Char('e'));
        assert_eq!(keymap.cells2[0][0], Some(Key::Char('A')));
        // ตัวเลข/สัญลักษณ์ที่ต้อง Shift
        assert_eq!(keymap.cells[4][0], Key::Char('1'));
        assert_eq!(keymap.cells[6][0], Key::Char('!'));
    }

    #[cfg(windows)]
    #[test]
    fn russian_layout_maps_qwerty_positions() {
        use rnitype_core::Key;

        let Some(keymap) = build_keymap("00000419") else {
            return; // เครื่องนี้ไม่มี layout RU
        };
        // ตำแหน่งเดียวกับ 'a' ใน EN → 'ф' ใน RU
        assert_eq!(keymap.cells[0][0], Key::Char('ф'));
        assert_eq!(keymap.cells2[0][0], Some(Key::Char('Ф')));
        // ตัวเลข/สัญลักษณ์ยังอยู่ตำแหน่งเดิม
        assert_eq!(keymap.cells[4][0], Key::Char('1'));
    }

    #[test]
    fn only_en_and_th_are_typing_supported_today() {        for (code, expected) in [
            ("en", true),
            ("th", true),
            ("ru", false),
            ("ja", false),
            ("0x1234", false),
        ] {
            let info = LanguageInfo {
                code: code.into(),
                klid: "00000000".into(),
                label: String::new(),
                kind: classify(code),
            };
            assert_eq!(info.typing_supported(), expected, "code={code}");
        }
        assert_eq!(typing_langs(), ["en", "th"]);
    }
}
