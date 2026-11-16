//! Target guard — กันการส่งคีย์เข้าแอปที่ผู้ใช้ใส่ exclusion list ไว้
//! และเตือนเมื่อหน้าต่างเป้าหมาย elevated (UIPI จะบล็อก SendInput)
//!
//! - [`guard_reason`] เป็น policy ล้วน → test ได้ทุกเครื่อง
//! - [`foreground_target`] / [`own_elevated`] เป็น FFI ของ Windows (cfg)
//!
//! อ้างอิง `Design/03-engineering/01-architecture.md` §3 (UIPI / exclusion)
//! และ `Design/02-design/04-state-machine-spec.md` (DISABLED + reason)

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;

/// เหตุผลที่หยุดรับ input ชั่วคราว
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    /// เปิดแอปที่อยู่ใน `system.exclusion` (เช่น เกมที่บล็อก SendInput)
    Exclusion,
    /// หน้าต่างเป้าหมาย elevated แต่เราไม่ได้รันเป็น admin → UIPI บล็อก
    AdminRequired,
}

/// ข้อมูลหน้าต่างเป้าหมายที่ใช้ตัดสิน (เฉพาะที่จำเป็น)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetInfo {
    /// ชื่อไฟล์ .exe (basename) ของโปรเซสเจ้าของหน้าต่าง
    pub exe: Option<String>,
    /// โปรเซสเจ้าของหน้าต่างรันแบบ elevated หรือไม่
    pub elevated: bool,
}

impl TargetInfo {
    pub fn new(exe: Option<impl Into<String>>, elevated: bool) -> Self {
        Self {
            exe: exe.map(Into::into),
            elevated,
        }
    }
}

/// ควรหยุดส่ง input หรือไม่ — `None` = ส่งได้ตามปกติ
///
/// `own_elevated` = ตัวแอปเองรันแบบ admin (ถ้าใช่ UIPI ไม่บล็อก)
pub fn guard_reason(
    target: Option<&TargetInfo>,
    exclusion: &[String],
    own_elevated: bool,
) -> Option<BlockReason> {
    let target = target?;
    if let Some(exe) = &target.exe
        && matches_exclusion(exe, exclusion)
    {
        return Some(BlockReason::Exclusion);
    }
    if target.elevated && !own_elevated {
        return Some(BlockReason::AdminRequired);
    }
    None
}

/// ชื่อ exe แบบ basename ตัวเล็ก (ใช้เทียบกับ exclusion list)
pub fn normalize_exe(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .trim()
        .to_ascii_lowercase()
}

/// เทียบชื่อ exe กับรายการ exclusion (ไม่สนตัวพิมพ์ · รองรับ `*` และ `?`)
pub fn matches_exclusion(exe: &str, patterns: &[String]) -> bool {
    let name = normalize_exe(exe);
    patterns.iter().any(|pattern| {
        let pattern = normalize_exe(pattern.trim());
        !pattern.is_empty() && wildcard_match(&pattern, &name)
    })
}

/// glob แบบง่าย: `*` = ตัวอักษรใดก็ได้กี่ตัว · `?` = 1 ตัว
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(star_pos) = star {
            pi = star_pos + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// ตัวแอปเองรันแบบ elevated หรือไม่
#[cfg(windows)]
pub fn own_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = std::ptr::null_mut();
    // SAFETY: เปิด token ของโปรเซสตัวเองแบบ query เท่านั้น แล้วปิดด้วย CloseHandle
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn own_elevated() -> bool {
    false
}

/// อ่านหน้าต่างที่ focus อยู่ (exe + elevated) — `None` = อ่านไม่ได้
#[cfg(windows)]
pub fn foreground_target() -> Option<TargetInfo> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    // SAFETY: ทุก handle ถูกปิดก่อนออกจากฟังก์ชัน; buffer เป็นของเราทั้งหมด
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 1024];
        let mut len = buffer.len() as u32;
        let exe = if QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut len) != 0 {
            let path = OsString::from_wide(&buffer[..len as usize])
                .to_string_lossy()
                .into_owned();
            Some(normalize_exe(&path))
        } else {
            None
        };

        let mut elevated = false;
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) != 0 {
            let mut info = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                (&mut info as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );
            CloseHandle(token);
            elevated = ok != 0 && info.TokenIsElevated != 0;
        }

        CloseHandle(process);
        Some(TargetInfo { exe, elevated })
    }
}

#[cfg(not(windows))]
pub fn foreground_target() -> Option<TargetInfo> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    #[test]
    fn exclusion_matches_case_insensitively_by_basename() {
        let list = patterns(&["Game.exe", "C:\\Tools\\Editor.EXE"]);
        assert!(matches_exclusion("game.EXE", &list));
        assert!(matches_exclusion("D:\\Games\\GAME.exe", &list));
        assert!(matches_exclusion("editor.exe", &list));
        assert!(!matches_exclusion("notepad.exe", &list));
    }

    #[test]
    fn exclusion_supports_wildcards() {
        let list = patterns(&["*.exe", "steam*"]);
        assert!(matches_exclusion("any.exe", &list));
        assert!(matches_exclusion("steamwebhelper.exe", &list));

        let only_exe = patterns(&["game?.exe"]);
        assert!(matches_exclusion("game1.exe", &only_exe));
        assert!(!matches_exclusion("game12.exe", &only_exe));
    }

    #[test]
    fn blank_patterns_never_match() {
        let list = patterns(&["", "   "]);
        assert!(!matches_exclusion("game.exe", &list));
    }

    #[test]
    fn guard_blocks_exclusion_before_admin() {
        let target = TargetInfo::new(Some("game.exe"), true);
        assert_eq!(
            guard_reason(Some(&target), &patterns(&["game.exe"]), false),
            Some(BlockReason::Exclusion)
        );
    }

    #[test]
    fn guard_blocks_elevated_target_unless_we_are_admin() {
        let target = TargetInfo::new(Some("Taskmgr.exe"), true);
        assert_eq!(
            guard_reason(Some(&target), &[], false),
            Some(BlockReason::AdminRequired)
        );
        assert_eq!(guard_reason(Some(&target), &[], true), None);
    }

    #[test]
    fn normal_target_is_allowed() {
        let target = TargetInfo::new(Some("notepad.exe"), false);
        assert_eq!(guard_reason(Some(&target), &patterns(&["game.exe"]), false), None);
        assert_eq!(guard_reason(None, &patterns(&["game.exe"]), false), None);
    }
}
