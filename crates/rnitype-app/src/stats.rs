//! ตัวเลขวัดผลการใช้งาน (P6 slice 1)
//!
//! - WPM มาตรฐาน = **5 ตัวอักษร = 1 คำ** (คำนวณจาก commits ในหน้าต่าง 60 วินาที)
//! - latency ของรอบ action = เวลาตั้งแต่ sample จอย → ส่งคีย์ออก (วัดใน `run_loop`)

/// WPM จากจำนวนตัวอักษรที่พิมพ์ในช่วงเวลาหนึ่ง
///
/// `window_secs` = ความยาวหน้าต่างที่เก็บ commits (ปกติ 60 วินาที)
pub fn wpm(chars: usize, window_secs: f64) -> f64 {
    if window_secs <= 0.0 {
        return 0.0;
    }
    chars as f64 / 5.0 * (60.0 / window_secs)
}

/// CPU ของโปรเซส (%) จากผลต่างของเวลาที่ใช้ CPU กับเวลาจริง
///
/// `cpu_100ns` = ผลต่าง kernel+user time (หน่วย 100 ns) · `wall_ms` = เวลาจริงที่ผ่าน
pub fn cpu_percent(cpu_100ns: u64, wall_ms: u64) -> f64 {
    if wall_ms == 0 {
        return 0.0;
    }
    let cpu_ms = cpu_100ns as f64 / 10_000.0;
    (cpu_ms / wall_ms as f64 * 100.0 * 10.0).round() / 10.0
}

/// หน่วยความจำที่ใช้ (Working Set) → MB
pub fn format_mb(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
}

/// CPU (100 ns) + Working Set (bytes) ของโปรเซสนี้ — `None` = อ่านไม่ได้
#[cfg(windows)]
pub fn process_usage() -> Option<(u64, u64)> {
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{FILETIME, GetLastError};
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    // SAFETY: handle เป็นของโปรเซสตัวเอง; struct เป็นของเราและ zeroed ตาม API
    unsafe {
        let process = GetCurrentProcess();
        let mut creation = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit = creation;
        let mut kernel = creation;
        let mut user = creation;
        if GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) == 0 {
            return None;
        }
        let filetime = |time: FILETIME| {
            (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
        };
        let cpu = filetime(kernel) + filetime(user);

        let mut memory: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let ok = K32GetProcessMemoryInfo(
            process,
            &mut memory,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        let _ = GetLastError();
        Some((cpu, if ok != 0 { memory.WorkingSetSize as u64 } else { 0 }))
    }
}

#[cfg(not(windows))]
pub fn process_usage() -> Option<(u64, u64)> {
    None
}

/// ค่า latency ที่อ่านง่าย: `0.12 ms` / `1.4 ms`
pub fn format_ms(micros: u64) -> String {
    let ms = micros as f64 / 1000.0;
    if ms < 1.0 {
        format!("{ms:.3} ms")
    } else if ms < 10.0 {
        format!("{ms:.2} ms")
    } else {
        format!("{ms:.1} ms")
    }
}

/// `3725` วินาที → `1h 02m 05s`
pub fn format_uptime(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpm_uses_five_characters_per_word() {
        assert_eq!(wpm(5, 60.0), 1.0);
        assert_eq!(wpm(100, 60.0), 20.0);
        assert_eq!(wpm(0, 60.0), 0.0);
    }

    #[test]
    fn shorter_windows_scale_up() {
        // 10 ตัวอักษรใน 30 วิ = 4 WPM (เทียบหนึ่งนาที)
        assert_eq!(wpm(10, 30.0), 4.0);
        assert_eq!(wpm(10, 0.0), 0.0);
    }

    #[test]
    fn latency_and_uptime_are_human_readable() {
        assert_eq!(format_ms(120), "0.120 ms");
        assert_eq!(format_ms(1_400), "1.40 ms");
        assert_eq!(format_ms(12_345), "12.3 ms");
        assert_eq!(format_uptime(5), "5s");
        assert_eq!(format_uptime(65), "1m 05s");
        assert_eq!(format_uptime(3725), "1h 02m 05s");
    }

    #[test]
    fn cpu_percent_uses_cpu_time_over_wall_time() {
        // ใช้ CPU 50 ms ใน 1 วินาที = 5%
        assert_eq!(cpu_percent(500_000, 1_000), 5.0);
        // ครึ่งหนึ่งของหนึ่งคอร์
        assert_eq!(cpu_percent(5_000_000, 1_000), 50.0);
        assert_eq!(cpu_percent(100, 0), 0.0);
    }

    #[test]
    fn memory_formats_in_megabytes() {
        assert_eq!(format_mb(0), "0 MB");
        assert_eq!(format_mb(64 * 1024 * 1024), "64 MB");
        assert_eq!(format_mb(157_286_400), "150 MB");
    }
}
