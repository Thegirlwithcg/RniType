//! ต่อ gesture → keymap → Action (ตัว injector อยู่ใน crate ของแอป)

use std::collections::BTreeMap;

use crate::action::{Action, Key, KeyPress};
use crate::gesture::{Commit, GestureConfig, GestureEngine, GestureState};
use crate::keymap::{Keymap, Lang, lang_code, lookup_layer};
use crate::normalizer::Dir8;
use crate::pad::PadState;

#[derive(Clone, Debug)]
pub struct TypingPipeline {
    engine: GestureEngine,
    lang: Lang,
    /// รหัสภาษาที่ใช้ lookup override (`en`, `th`, `ru`, …)
    code: String,
    /// ชื่อที่แสดงบน HUD (จาก layout ที่ตรวจพบ)
    label: String,
    /// ตารางที่สร้างจาก layout ของ Windows — `None` = built-in ของ `lang`
    table: Option<Keymap>,
    layer: u8,
    /// ช่องที่ผู้ใช้แก้เอง (config `languages[].overrides`) — `None` = ไม่มีคีย์ (layer 2)
    ///
    /// key = (code, layer, กลุ่ม, ช่อง) · เก็บแยกภาษาจึงสลับภาษาแล้วยังอยู่
    overrides: BTreeMap<(String, u8, u8, u8), Option<Key>>,
}

impl TypingPipeline {
    pub fn new(lang: Lang, cfg: GestureConfig) -> Self {
        Self {
            engine: GestureEngine::new(cfg),
            lang,
            code: lang_code(lang).to_string(),
            label: lang.label().to_string(),
            table: None,
            layer: 1,
            overrides: BTreeMap::new(),
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        self.code = lang_code(lang).to_string();
        self.label = lang.label().to_string();
        self.table = None;
    }

    /// ตั้งภาษาที่มีตารางของตัวเอง (layout จาก Windows)
    pub fn set_profile(&mut self, lang: Lang, code: &str, label: &str, table: Option<Keymap>) {
        self.lang = lang;
        self.code = code.to_string();
        self.label = label.to_string();
        self.table = table;
    }

    /// ชั้นของ keymap (1 = ปกติ · 2 = RB/RT)
    pub fn layer(&self) -> u8 {
        self.layer
    }

    pub fn set_layer(&mut self, layer: u8) {
        self.layer = layer.max(1);
    }

    /// คีย์ของช่องนั้น — override ของผู้ใช้ชนะตาราง (built-in หรือ layout ที่สร้างไว้)
    ///
    /// คืน `None` = ช่องว่าง (layer 2) → commit แล้วไม่ส่งคีย์
    pub fn key_at(&self, group: Dir8, sector: Dir8) -> Option<Key> {
        let index = (
            self.code.clone(),
            self.layer,
            group.index() as u8,
            sector.index() as u8,
        );
        if let Some(value) = self.overrides.get(&index) {
            return *value;
        }
        match &self.table {
            Some(table) => table.key_at(self.layer, group.index(), sector.index()),
            None => lookup_layer(self.lang, self.layer, group, sector),
        }
    }

    /// ตั้ง override 1 ช่อง (`None` = ทำให้ช่องว่าง เฉพาะ layer 2)
    pub fn set_override(
        &mut self,
        code: &str,
        layer: u8,
        group: u8,
        sector: u8,
        key: Option<Key>,
    ) {
        if layer < 1 || group > 7 || sector > 7 {
            return;
        }
        self.overrides
            .insert((code.to_string(), layer, group, sector), key);
    }

    pub fn clear_overrides(&mut self) {
        self.overrides.clear();
    }

    pub fn override_count(&self) -> usize {
        self.overrides.len()
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
        let key = self.key_at(group, sector)?;
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
    fn overrides_win_over_the_builtin_table_and_stay_per_language() {
        let mut en = TypingPipeline::default();
        en.set_override("en", 1, 0, 0, Some(Key::Char('z')));
        let action = press(&mut en, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('z'),
                shift: false,
                ctrl: false,
                alt: false,
            })
        );

        // override ของ EN ต้องไม่รั่วไป TH
        let mut th = TypingPipeline::new(Lang::Th, GestureConfig::default());
        th.set_override("en", 1, 0, 0, Some(Key::Char('z')));
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
    fn layer_two_can_be_emptied_and_overrides_can_be_cleared() {
        let mut pipe = TypingPipeline::default();
        pipe.set_layer(2);
        pipe.set_override("en", 2, 0, 0, None);
        assert_eq!(press(&mut pipe, (0.0, 0.9), (0.0, 0.9)), None);
        assert_eq!(pipe.override_count(), 1);

        pipe.clear_overrides();
        assert_eq!(pipe.override_count(), 0);
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
    }

    #[test]
    fn custom_table_overrides_the_builtin_lookup() {
        let mut table = Keymap::builtin(Lang::En);
        table.set_cell(1, 0, 0, Some(Key::Char('ф')));
        let mut pipe = TypingPipeline::default();
        pipe.set_profile(Lang::En, "ru", "Russian", Some(table));
        assert_eq!(pipe.code(), "ru");
        let action = press(&mut pipe, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert_eq!(
            action,
            Action::Key(KeyPress {
                key: Key::Char('ф'),
                shift: false,
                ctrl: false,
                alt: false,
            })
        );

        // override ยังชนะตาราง custom
        pipe.set_override("ru", 1, 0, 0, Some(Key::Char('z')));
        let action = press(&mut pipe, (0.0, 0.9), (0.0, 0.9)).expect("commit");
        assert!(matches!(action, Action::Key(KeyPress { key: Key::Char('z'), .. })));
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
