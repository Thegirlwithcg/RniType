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
}
