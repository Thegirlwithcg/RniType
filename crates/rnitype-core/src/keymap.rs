//! Keymap 8 × 8 ตาม `Design/02-design/02-keymap-en-th.md`
//! index = กลุ่ม (LS) × ทิศ (RS)

use crate::action::Key;
use crate::normalizer::Dir8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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
        assert_eq!(label_of(Key::Named(crate::action::NamedKey::Space)), "Space");
    }
}
