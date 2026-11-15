//! ต่อ gesture → keymap → Action (ตัว injector อยู่ใน crate ของแอป)

use crate::action::{Action, KeyPress};
use crate::gesture::{Commit, GestureConfig, GestureEngine, GestureState};
use crate::keymap::{Lang, lookup};
use crate::pad::PadState;

#[derive(Clone, Debug)]
pub struct TypingPipeline {
    engine: GestureEngine,
    lang: Lang,
}

impl TypingPipeline {
    pub fn new(lang: Lang, cfg: GestureConfig) -> Self {
        Self { engine: GestureEngine::new(cfg), lang }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    pub fn state(&self) -> GestureState {
        self.engine.state()
    }

    /// `LT` ค้าง = Shift, `LB` ค้าง = Ctrl (ตาม spec ข้อ 2.3)
    pub fn tick(&mut self, pad: &PadState, dt_ms: u32) -> Option<Action> {
        let Commit { group, sector } = self.engine.tick(pad, dt_ms)?;
        Some(Action::Key(KeyPress {
            key: lookup(self.lang, group, sector),
            shift: pad.buttons.lt,
            ctrl: pad.buttons.lb,
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
        assert_eq!(action, Action::Key(KeyPress { key: Key::Char('a'), shift: false, ctrl: false }));

        let mut th = TypingPipeline::new(Lang::Th, GestureConfig::default());
        let action = press(&mut th, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(action, Action::Key(KeyPress { key: Key::Char('ก'), shift: false, ctrl: false }));
    }

    #[test]
    fn LT_becomes_shift_on_commit() {
        let mut pipe = TypingPipeline::default();
        let mut pad = p((0.0, 0.9), (0.0, 0.0));
        pad.buttons.lt = true;
        pipe.tick(&pad, 4);
        pipe.tick(&p((0.0, 0.9), (0.0, 0.9)), 4);
        pipe.tick(&p((0.0, 0.9), (0.0, 0.0)), 4);
        let action = pipe.tick(&pad, 64).expect("commit");
        assert_eq!(action, Action::Key(KeyPress { key: Key::Char('a'), shift: true, ctrl: false }));
    }
}
