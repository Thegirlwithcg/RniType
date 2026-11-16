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

    #[test]
    fn only_en_and_th_are_typing_supported_today() {
        for (code, expected) in [
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
