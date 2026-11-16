//! Keymap 8 × 8 ตาม `Design/02-design/02-keymap-en-th.md`
//! index = กลุ่ม (LS) × ทิศ (RS)

use crate::action::{Key, NamedKey};
use crate::normalizer::Dir8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Lang {
    #[default]
    En,
    Th,
}

impl Lang {
    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Th => "TH",
        }
    }
}

const fn c(ch: char) -> Key {
    Key::Char(ch)
}

/// ชั้น 1 — Type-EN (v0.2: กลุ่ม ↑ = สระ AEIOU + T N S)
pub static EN: [[Key; 8]; 8] = [
    // ↑ สระ
    [
        c('a'),
        c('e'),
        c('i'),
        c('o'),
        c('u'),
        c('t'),
        c('n'),
        c('s'),
    ],
    // ↗ B
    [
        c('h'),
        c('r'),
        c('d'),
        c('l'),
        c('c'),
        c('m'),
        c('f'),
        c('w'),
    ],
    // → C
    [
        c('g'),
        c('p'),
        c('b'),
        c('v'),
        c('k'),
        c('x'),
        c('y'),
        c('j'),
    ],
    // ↘ D
    [
        c('q'),
        c('z'),
        c('\''),
        c('"'),
        c('-'),
        c('_'),
        c('.'),
        c(','),
    ],
    // ↓ 1
    [
        c('1'),
        c('2'),
        c('3'),
        c('4'),
        c('5'),
        c('6'),
        c('7'),
        c('8'),
    ],
    // ↙ 2
    [
        c('9'),
        c('0'),
        c('/'),
        c('\\'),
        c('('),
        c(')'),
        c('['),
        c(']'),
    ],
    // ← 3
    [
        c('!'),
        c('?'),
        c('@'),
        c('#'),
        c('$'),
        c('%'),
        c('&'),
        c('*'),
    ],
    // ↖ 4
    [
        c(':'),
        c(';'),
        c('='),
        c('+'),
        c('<'),
        c('>'),
        c('`'),
        c('~'),
    ],
];

/// ชั้น 1 — Type-TH
pub static TH: [[Key; 8]; 8] = [
    // ↑ พยัญชนะพบบ่อย
    [
        c('ก'),
        c('น'),
        c('ร'),
        c('อ'),
        c('ง'),
        c('ม'),
        c('ย'),
        c('ล'),
    ],
    // ↗ สระ
    [c('า'), c('ะ'), c('ิ'), c('ี'), c('ื'), c('ึ'), c('ุ'), c('ู')],
    // → สระนำ + อักษรกลาง
    [c('เ'), c('แ'), c('โ'), c('ใ'), c('ไ'), c('อ'), c('่'), c('้')],
    // ↘ วรรณยุกต์ / การันต์
    [c('๊'), c('๋'), c('์'), c('็'), c('ำ'), c('ั'), c('ฤ'), c('ฮ')],
    // ↓ พยัญชนะกลาง
    [
        c('ด'),
        c('ท'),
        c('ส'),
        c('ว'),
        c('ต'),
        c('ป'),
        c('ค'),
        c('ข'),
    ],
    // ↙ พยัญชนะกึ่งกลาง
    [
        c('จ'),
        c('ช'),
        c('บ'),
        c('พ'),
        c('ฟ'),
        c('ผ'),
        c('ฝ'),
        c('ห'),
    ],
    // ← หายาก / บาลี
    [
        c('ซ'),
        c('ญ'),
        c('ฎ'),
        c('ฏ'),
        c('ฐ'),
        c('ฑ'),
        c('ฒ'),
        c('ณ'),
    ],
    // ↖ เหลือ
    [
        c('ฉ'),
        c('ฌ'),
        c('ฆ'),
        c('ธ'),
        c('ศ'),
        c('ษ'),
        c('ถ'),
        c('ภ'),
    ],
];

pub fn table(lang: Lang) -> &'static [[Key; 8]; 8] {
    match lang {
        Lang::En => &EN,
        Lang::Th => &TH,
    }
}

const fn k(ch: char) -> Option<Key> {
    Some(Key::Char(ch))
}

const fn n(key: NamedKey) -> Option<Key> {
    Some(Key::Named(key))
}

/// ชั้น 2 — Type-EN (RB/RT) ตาม `02-keymap-en-th.md` §4
///
/// ช่องที่ยังไม่มีคีย์ = `None` (ภาษาไทยมีกลุ่มว่าง) → commit แล้วไม่ส่งอะไร
pub static EN_L2: [[Option<Key>; 8]; 8] = [
    // ↑ A–H
    [
        k('A'),
        k('B'),
        k('C'),
        k('D'),
        k('E'),
        k('F'),
        k('G'),
        k('H'),
    ],
    // ↗ I–P
    [
        k('I'),
        k('J'),
        k('K'),
        k('L'),
        k('M'),
        k('N'),
        k('O'),
        k('P'),
    ],
    // → Q–X
    [
        k('Q'),
        k('R'),
        k('S'),
        k('T'),
        k('U'),
        k('V'),
        k('W'),
        k('X'),
    ],
    // ↘ สัญลักษณ์ (สเปกมี 6 ตัว เติม ° ± § ให้ครบ 8 ช่องตามม็อกอัป)
    [
        k('{'),
        k('}'),
        k('|'),
        k('^'),
        k('¬'),
        k('°'),
        k('±'),
        k('§'),
    ],
    // ↓ F1–F8
    [
        n(NamedKey::F(1)),
        n(NamedKey::F(2)),
        n(NamedKey::F(3)),
        n(NamedKey::F(4)),
        n(NamedKey::F(5)),
        n(NamedKey::F(6)),
        n(NamedKey::F(7)),
        n(NamedKey::F(8)),
    ],
    // ↙ F9–F12 + PrtSc + Ins
    [
        n(NamedKey::F(9)),
        n(NamedKey::F(10)),
        n(NamedKey::F(11)),
        n(NamedKey::F(12)),
        n(NamedKey::PrintScreen),
        n(NamedKey::Insert),
        None,
        None,
    ],
    // ← Nav
    [
        n(NamedKey::Home),
        n(NamedKey::End),
        n(NamedKey::PageUp),
        n(NamedKey::PageDown),
        n(NamedKey::Left),
        n(NamedKey::Up),
        n(NamedKey::Down),
        n(NamedKey::Right),
    ],
    // ↖ แป้นควบคุม (Ctrl/Shift ยังไม่ทำ — ต้องมีคีย์ผสมที่ค้างได้)
    [
        n(NamedKey::Delete),
        n(NamedKey::Backspace),
        n(NamedKey::Escape),
        n(NamedKey::Enter),
        n(NamedKey::Tab),
        n(NamedKey::Space),
        None,
        None,
    ],
];

/// ชั้น 2 — Type-TH (RB/RT)
pub static TH_L2: [[Option<Key>; 8]; 8] = [
    // ↑ อักขระที่ไม่มีในชั้น 1
    [k('ฬ'), k('ฃ'), k('ฅ'), k('ํ'), k('ๅ'), None, None, None],
    // ↗ พิเศษ
    [k('ฦ'), k('ฯ'), k('ๆ'), k('๚'), k('๛'), None, None, None],
    // → เลขไทย ๑–๘
    [
        k('๑'),
        k('๒'),
        k('๓'),
        k('๔'),
        k('๕'),
        k('๖'),
        k('๗'),
        k('๘'),
    ],
    // ↘ เลขไทยที่เหลือ + สกุลเงิน/ขีด
    [k('๙'), k('๐'), k('฿'), k('–'), k('—'), None, None, None],
    // ↓ ว่าง
    [None, None, None, None, None, None, None, None],
    // ↙ ว่าง
    [None, None, None, None, None, None, None, None],
    // ← Nav (เหมือน EN)
    [
        n(NamedKey::Home),
        n(NamedKey::End),
        n(NamedKey::PageUp),
        n(NamedKey::PageDown),
        n(NamedKey::Left),
        n(NamedKey::Up),
        n(NamedKey::Down),
        n(NamedKey::Right),
    ],
    // ↖ แป้นควบคุม (เหมือน EN)
    [
        n(NamedKey::Delete),
        n(NamedKey::Backspace),
        n(NamedKey::Escape),
        n(NamedKey::Enter),
        n(NamedKey::Tab),
        n(NamedKey::Space),
        None,
        None,
    ],
];

/// ตารางชั้น 2 ของภาษา (`layer` ใดก็ได้ ≥ 2 ใช้ตารางเดียวกัน)
pub fn table_layer2(lang: Lang) -> &'static [[Option<Key>; 8]; 8] {
    match lang {
        Lang::En => &EN_L2,
        Lang::Th => &TH_L2,
    }
}

/// ชื่อกลุ่ม 8 กลุ่ม (index = [`Dir8`] จาก ↑ ไล่ตามเข็มนาฬิกา)
///
/// มาจาก `Design/02-design/02-keymap-en-th.md` §1 — HUD ใช้เป็นป้ายวงใน
pub fn group_names(lang: Lang) -> [&'static str; 8] {
    match lang {
        Lang::En => ["AEIOU", "B", "C", "D", "1", "2", "3", "4"],
        Lang::Th => [
            "พยัญชนะ",
            "สระ",
            "สระนำ",
            "วรรณยุกต์",
            "กลาง",
            "กึ่งกลาง",
            "หายาก",
            "เหลือ",
        ],
    }
}

/// ชื่อกลุ่มของชั้น 2 (HUD ใช้วงในตอน RB/RT เปิดอยู่)
pub fn group_names_layer2(lang: Lang) -> [&'static str; 8] {
    match lang {
        Lang::En => ["A–H", "I–P", "Q–X", "Sym", "F1–F8", "F9–F12", "Nav", "Keys"],
        Lang::Th => [
            "ฬ ฃ ฅ",
            "ฦ ฯ ๆ",
            "๑–๘",
            "๙ ๐ ฿",
            "ว่าง",
            "ว่าง",
            "Nav",
            "Keys",
        ],
    }
}

/// ชื่อกลุ่มตามชั้น (`layer >= 2` = ชั้น 2)
pub fn group_names_for(lang: Lang, layer: u8) -> [&'static str; 8] {
    if layer >= 2 {
        group_names_layer2(lang)
    } else {
        group_names(lang)
    }
}

/// ป้ายของคีย์ 1 ช่อง (HUD): ตัวอักษรตรงตัว, ปุ่มมีชื่อใช้ชื่อสั้น
pub fn label_of(key: Key) -> String {
    match key {
        Key::Char(ch) => ch.to_string(),
        Key::Named(named) => format!("{named:?}"),
    }
}

/// ดึงคีย์จาก gesture (กลุ่มจาก LS, ลำดับจาก RS)
pub fn lookup(lang: Lang, group: Dir8, sector: Dir8) -> Key {
    table(lang)[group.index()][sector.index()]
}

/// ดึงคีย์ตามชั้น — ชั้น 2 บางช่องว่าง (`None` = ไม่ส่งคีย์)
pub fn lookup_layer(lang: Lang, layer: u8, group: Dir8, sector: Dir8) -> Option<Key> {
    if layer >= 2 {
        table_layer2(lang)[group.index()][sector.index()]
    } else {
        Some(lookup(lang, group, sector))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn chars(lang: Lang) -> Vec<char> {
        table(lang)
            .iter()
            .flatten()
            .filter_map(|k| match k {
                Key::Char(ch) => Some(*ch),
                Key::Named(_) => None,
            })
            .collect()
    }

    #[test]
    fn en_layer1_covers_letters_digits_and_basic_punctuation() {
        let all: HashSet<char> = chars(Lang::En).into_iter().collect();
        for ch in 'a'..='z' {
            assert!(all.contains(&ch), "ขาดตัวอักษร {ch}");
        }
        for ch in '0'..='9' {
            assert!(all.contains(&ch), "ขาดเลข {ch}");
        }
        assert_eq!(all.len(), 64, "ตาราง EN ต้องมี 64 ช่องไม่ซ้ำ");
    }

    #[test]
    fn th_layer1_has_no_duplicates_and_covers_common_consonants() {
        let list = chars(Lang::Th);
        let all: HashSet<char> = list.iter().copied().collect();
        // 63 ไม่ใช่ 64 เพราะตารางใน spec มี "อ" ซ้ำ (กลุ่ม ↑ กับ →)
        // → บันทึกเป็น Q15 ใน 04-decisions/01-open-questions.md
        assert_eq!(all.len(), 63, "ตาราง TH ต้องไม่ซ้ำ (ยกเว้น อ ที่ spec ซ้ำ)");
        for ch in ['ก', 'น', 'ร', 'อ', 'า', 'เ', '่', '้', 'ำ', 'ั'] {
            assert!(all.contains(&ch), "ขาดอักขระไทย {ch}");
        }
    }

    #[test]
    fn lookup_follows_spec_table() {
        assert_eq!(lookup(Lang::En, Dir8::N, Dir8::N), Key::Char('a'));
        assert_eq!(lookup(Lang::En, Dir8::SE, Dir8::SE), Key::Char('"'));
        assert_eq!(lookup(Lang::Th, Dir8::N, Dir8::N), Key::Char('ก'));
        assert_eq!(lookup(Lang::Th, Dir8::NW, Dir8::NW), Key::Char('ภ'));
        assert_eq!(lookup(Lang::En, Dir8::S, Dir8::W), Key::Char('7'));
        assert_eq!(lookup(Lang::En, Dir8::NW, Dir8::W), Key::Char('`'));
    }

    #[test]
    fn named_key_variant_is_usable_for_layer_two() {
        let press = Key::Named(crate::action::NamedKey::Enter);
        assert_eq!(format!("{press}"), "Enter");
    }

    #[test]
    fn group_names_follow_the_design_doc() {
        assert_eq!(group_names(Lang::En)[0], "AEIOU");
        assert_eq!(group_names(Lang::En)[7], "4");
        assert_eq!(group_names(Lang::Th)[1], "สระ");
        assert_eq!(group_names(Lang::Th)[3], "วรรณยุกต์");
        for lang in [Lang::En, Lang::Th] {
            assert_eq!(group_names(lang).len(), 8);
            assert!(group_names(lang).iter().all(|name| !name.is_empty()));
        }
    }

    #[test]
    fn label_of_prefers_the_character() {
        assert_eq!(label_of(Key::Char('ก')), "ก");
        assert_eq!(
            label_of(Key::Named(crate::action::NamedKey::Space)),
            "Space"
        );
    }

    #[test]
    fn layer_two_en_matches_the_design_groups() {
        // ↑ = A–H · ↗ = I–P · → = Q–X · ↓ = F1–F8
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::N, Dir8::N),
            Some(Key::Char('A'))
        );
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::NE, Dir8::NW),
            Some(Key::Char('P'))
        );
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::E, Dir8::NW),
            Some(Key::Char('X'))
        );
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::S, Dir8::N),
            Some(Key::Named(crate::action::NamedKey::F(1)))
        );
        // ← = Nav · ↖ = Keys
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::W, Dir8::N),
            Some(Key::Named(crate::action::NamedKey::Home))
        );
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::NW, Dir8::S),
            Some(Key::Named(crate::action::NamedKey::Tab))
        );
        // ช่องที่ยังไม่ทำ = ไม่มีคีย์
        assert_eq!(lookup_layer(Lang::En, 2, Dir8::NW, Dir8::W), None);
    }

    #[test]
    fn layer_two_th_fills_the_gaps_and_leaves_empty_groups_empty() {
        let all: HashSet<char> = table_layer2(Lang::Th)
            .iter()
            .flatten()
            .filter_map(|key| {
                key.and_then(|key| match key {
                    Key::Char(ch) => Some(ch),
                    Key::Named(_) => None,
                })
            })
            .collect();
        for ch in ['ฬ', 'ฃ', 'ฅ', 'ํ', 'ๅ', 'ฦ', 'ฯ', 'ๆ', '๑', '๙', '฿'] {
            assert!(all.contains(&ch), "ชั้น 2 ไทยขาด {ch}");
        }
        for dir in Dir8::ALL {
            assert_eq!(
                lookup_layer(Lang::Th, 2, Dir8::S, dir),
                None,
                "ชั้น 2 ไทยกลุ่ม ↓ ต้องว่าง"
            );
        }
    }

    #[test]
    fn layer_one_lookup_is_unchanged() {
        assert_eq!(
            lookup_layer(Lang::En, 1, Dir8::N, Dir8::N),
            Some(Key::Char('a'))
        );
        assert_eq!(
            lookup_layer(Lang::En, 2, Dir8::N, Dir8::N),
            Some(Key::Char('A'))
        );
        assert!(group_names_for(Lang::En, 1).contains(&"AEIOU"));
        assert_eq!(group_names_for(Lang::En, 2)[0], "A–H");
    }
}
