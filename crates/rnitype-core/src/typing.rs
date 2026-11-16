//! ต่อ gesture → keymap → Action (ตัว injector อยู่ใน crate ของแอป)

use crate::action::{Action, KeyPress};
use crate::gesture::{Commit, GestureConfig, GestureEngine, GestureState};
use crate::keymap::{Lang, lookup_layer};
use crate::normalizer::Dir8;
use crate::pad::PadState;

#[derive(Clone, Debug)]
pub struct TypingPipeline {
    engine: GestureEngine,
    lang: Lang,
    layer: u8,
}

impl TypingPipeline {
    pub fn new(lang: Lang, cfg: GestureConfig) -> Self {
        Self {
            engine: GestureEngine::new(cfg),
            lang,
            layer: 1,
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// ชั้นของ keymap (1 = ปกติ · 2 = RB/RT)
    pub fn layer(&self) -> u8 {
        self.layer
    }

    pub fn set_layer(&mut self, layer: u8) {
        self.layer = layer.max(1);
    }

    pub fn state(&self) -> GestureState {
        self.engine.state()
    }

    /// จำนวนครั้งที่พิมพ์ซ้ำรอบปัจจุบัน (HUD แสดง ×n)
    pub fn repeat_count(&self) -> u32 {
        self.engine.repeat_count()
    }

    /// กลุ่มที่ LS เลือกอยู่ (index ตรงกับ [`Dir8`]) — HUD ใช้ไฮไลต์วงใน
    pub fn group(&self) -> Option<Dir8> {
        self.engine.group()
    }

    /// ช่องที่ RS เลือกอยู่ — HUD ใช้วาด preview + ไฮไลต์วงนอก
    pub fn sector(&self) -> Option<Dir8> {
        self.engine.sector()
    }

    /// ยกเลิก gesture ที่ค้าง (สลับ mode / เปิดวงคำสั่ง / ปิดฉุกเฉิน)
    /// หลังเรียก ต้องปล่อย LS กลับก่อนจึงจะเริ่ม gesture ใหม่ได้
    pub fn cancel(&mut self) {
        self.engine.cancel();
    }

    /// `true` = gesture ถูกยกเลิกและยังรอ LS กลับเข้าใกล้ศูนย์
    pub fn locked(&self) -> bool {
        self.engine.locked()
    }

    /// `LT` ค้าง = Shift, `LB` ค้าง = Ctrl (ตาม spec ข้อ 2.3)
    ///
    /// ชั้น 2 มีช่องว่าง (ไทย) → คืน `None` = commit แล้วไม่ส่งคีย์
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32) -> Option<Action> {
        let Commit { group, sector } = self.engine.tick(pad, dt_ms)?;
        let key = lookup_layer(self.lang, self.layer, group, sector)?;
        Some(Action::Key(KeyPress {
            key,
            shift: pad.buttons.lt,
            ctrl: pad.buttons.lb,
            alt: false,
        }))
    }
}

impl Default for TypingPipeline {
    fn default() -> Self {
        Self::new(Lang::default(), GestureConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Key;
    use crate::pad::Vec2;

    fn p(ls: (f32, f32), rs: (f32, f32)) -> PadState {
        PadState {
            ls: Vec2::new(ls.0, ls.1),
            rs: Vec2::new(rs.0, rs.1),
            ..PadState::default()
        }
    }

    fn press(pipe: &mut TypingPipeline, ls: (f32, f32), rs: (f32, f32)) -> Option<Action> {
        pipe.tick(&p(ls, rs), 4); // arm group
        pipe.tick(&p(ls, rs), 4); // select char
        pipe.tick(&p(ls, (0.0, 0.0)), 4); // release RS
        pipe.tick(&p(ls, (0.0, 0.0)), 64) // dwell
    }

    #[test]
    fn types_en_and_th_from_same_gesture() {
        let mut en = TypingPipeline::default();
        let action = press(&mut en, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: false,
                ctrl: false,
                alt: false,
            })
        );

        let mut th = TypingPipeline::new(Lang::Th, GestureConfig::default());
        let action = press(&mut th, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('ก'),
                shift: false,
                ctrl: false,
                alt: false,
            })
        );
    }

    #[test]
    fn lt_becomes_shift_on_commit() {
        let mut pipe = TypingPipeline::default();
        let mut pad = p((0.0, 0.9), (0.0, 0.0));
        pad.buttons.lt = true;
        pipe.tick(&pad, 4);
        pipe.tick(&p((0.0, 0.9), (0.0, 0.9)), 4);
        pipe.tick(&p((0.0, 0.9), (0.0, 0.0)), 4);
        let action = pipe.tick(&pad, 64).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('a'),
                shift: true,
                ctrl: false,
                alt: false,
            })
        );
    }

    #[test]
    fn layer_two_lookup_drives_the_commit_key() {
        let mut pipe = TypingPipeline::default();
        pipe.set_layer(2);
        let action = press(&mut pipe, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('A'),
                shift: false,
                ctrl: false,
                alt: false,
            })
        );

        // ชั้น 2 ไทยグループ ↓ ว่าง → commit เงียบ ไม่มีคีย์
        let mut th = TypingPipeline::new(Lang::Th, GestureConfig::default());
        th.set_layer(2);
        assert_eq!(press(&mut th, (0.0, -0.9), (0.0, 0.9)), None);
        assert_eq!(th.state(), GestureState::Idle);
    }
}
