//! Timer resolution — ยกความละเอียดของ `Sleep` เป็น 1 ms ระหว่างรัน
//!
//! Windows ปกติให้ `Sleep(3)` ได้ ~15 ms → loop 250 Hz จะช้ากว่าที่ออกแบบ.
//! ใช้ [`TimerResolution`] ครอบช่วงที่ runtime ทำงาน แล้วปล่อยคืนตอนจบ (RAII)
//! — เป็นการตั้งค่าทั้งระบบ จึงต้องไม่ลืม `timeEndPeriod`

/// ระหว่างที่มีค่านี้อยู่ Windows scheduler จะให้ sleep ละเอียดตาม `period_ms`
#[derive(Debug)]
pub struct TimerResolution {
    period_ms: u32,
    active: bool,
}

impl TimerResolution {
    /// ขอความละเอียด `period_ms` (ปกติ 1 ms) — ล้มเหลว = ยังรันต่อได้ (แค่ sleep หยาบขึ้น)
    pub fn new(period_ms: u32) -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Media::timeBeginPeriod;
            // SAFETY: timeBeginPeriod เป็นการตั้งค่า global ของ process เท่านั้น
            let result = unsafe { timeBeginPeriod(period_ms) };
            Self {
                period_ms,
                active: result == 0,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                period_ms,
                active: false,
            }
        }
    }

    /// `true` = ขอความละเอียดสำเร็จ (ค่าที่ขอมีผลจริง)
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for TimerResolution {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.active {
            use windows_sys::Win32::Media::timeEndPeriod;
            // SAFETY: คืนค่าที่เราขอไว้เท่านั้น (จับคู่กับ timeBeginPeriod)
            unsafe {
                timeEndPeriod(self.period_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_guard_can_be_created_and_dropped() {
        let timer = TimerResolution::new(1);
        // บน Windows เครื่องจริงควรสำเร็จ; ที่อื่นเป็น no-op
        if cfg!(windows) {
            assert!(timer.is_active());
        } else {
            assert!(!timer.is_active());
        }
    }
}
