//! Strict, bounded ARIB STD-B24 text decoding for SI metadata.
//!
//! SDT service descriptors carry ARIB 8-bit text, not UTF-8. This is kept
//! separate from caption data-group decoding: service metadata has no caption
//! timing or DRCS state, but it still uses B24 locking shifts and JIS code sets.

use std::sync::OnceLock;

#[derive(Clone, Copy)]
enum CodeSet {
    Kanji,
    Alphanumeric,
    Hiragana,
    Katakana,
    AdditionalSymbols,
    UnsupportedOneByte,
    UnsupportedTwoByte,
}

/// Decodes the common ARIB B24 SI text profile. An unsupported character is
/// represented locally without discarding otherwise valid surrounding text.
pub(crate) fn decode_service_name(bytes: &[u8]) -> Option<String> {
    const MAX_SERVICE_NAME_BYTES: usize = 252;
    if bytes.is_empty() || bytes.len() > MAX_SERVICE_NAME_BYTES {
        return None;
    }
    let mut sets = [
        CodeSet::Kanji,
        CodeSet::Alphanumeric,
        CodeSet::Hiragana,
        CodeSet::Katakana,
    ];
    let mut gl = 0usize;
    let mut gr = 2usize;
    let mut single_shift = None;
    let mut output = String::new();
    let mut index = 0usize;

    while let Some(&byte) = bytes.get(index) {
        match byte {
            0x0e => {
                gl = 1;
                index += 1;
            }
            0x0f => {
                gl = 0;
                index += 1;
            }
            0x19 => {
                single_shift = Some(2);
                index += 1;
            }
            0x1d => {
                single_shift = Some(3);
                index += 1;
            }
            0x1b => {
                let Some((action, consumed)) = parse_escape(bytes.get(index + 1..).unwrap_or(&[]))
                else {
                    index += 1;
                    continue;
                };
                match action {
                    EscapeAction::Designate(slot, set) => sets[slot] = set,
                    EscapeAction::LockingShiftGr(slot) => gr = slot,
                }
                index += consumed + 1;
            }
            0x20 => {
                output.push(' ');
                index += 1;
            }
            0x21..=0x7e | 0xa1..=0xfe => {
                let from_gr = byte >= 0xa1;
                let slot = single_shift.take().unwrap_or(gl);
                let slot = if from_gr { gr } else { slot };
                let graphic = if from_gr { byte & 0x7f } else { byte };
                match sets[slot] {
                    CodeSet::Kanji | CodeSet::AdditionalSymbols => {
                        let Some(&second) = bytes.get(index + 1) else {
                            break;
                        };
                        let second = if from_gr { second & 0x7f } else { second };
                        if !(0x21..=0x7e).contains(&second) {
                            output.push('〓');
                            index += 1;
                            continue;
                        }
                        output.push(decode_arib_kanji(graphic, second).unwrap_or('〓'));
                        index += 2;
                    }
                    CodeSet::Alphanumeric => {
                        output.push(arib_alphanumeric(graphic));
                        index += 1;
                    }
                    CodeSet::Hiragana => {
                        output.push(decode_arib_single_byte(hiragana_table(), graphic));
                        index += 1;
                    }
                    CodeSet::Katakana => {
                        output.push(decode_arib_single_byte(katakana_table(), graphic));
                        index += 1;
                    }
                    CodeSet::UnsupportedOneByte => {
                        output.push('〓');
                        index += 1;
                    }
                    CodeSet::UnsupportedTwoByte => {
                        output.push('〓');
                        let has_graphic_second = bytes
                            .get(index + 1)
                            .map(|second| {
                                let second = if from_gr { second & 0x7f } else { *second };
                                (0x21..=0x7e).contains(&second)
                            })
                            .unwrap_or(false);
                        index += usize::from(has_graphic_second) + 1;
                    }
                }
            }
            0x00..=0x1f => {
                // C0 formatting controls other than shifts have no useful
                // service-name meaning. Ignore them without treating their
                // presence as text or silently replacing them.
                index += 1;
            }
            0x7f..=0xa0 | 0xff => {
                // C1 presentation controls and stuffing do not invalidate the
                // surrounding SI text. Their visual effects are not metadata.
                index += 1;
            }
        }
    }

    let output = output.trim().to_owned();
    (!output.is_empty()).then_some(output)
}

#[derive(Clone, Copy)]
enum EscapeAction {
    Designate(usize, CodeSet),
    LockingShiftGr(usize),
}

fn parse_escape(bytes: &[u8]) -> Option<(EscapeAction, usize)> {
    match bytes {
        [0x7e, ..] => Some((EscapeAction::LockingShiftGr(1), 1)),
        [0x7d, ..] => Some((EscapeAction::LockingShiftGr(2), 1)),
        [0x7c, ..] => Some((EscapeAction::LockingShiftGr(3), 1)),
        [b'$', b'B' | b'@', ..] => Some((EscapeAction::Designate(0, CodeSet::Kanji), 2)),
        [b'$', b';', ..] => Some((EscapeAction::Designate(0, CodeSet::AdditionalSymbols), 2)),
        [b'$', b'9' | b':', ..] => Some((EscapeAction::Designate(0, CodeSet::Kanji), 2)),
        [selector @ (b'(' | b')' | b'*' | b'+'), final_byte, ..] => {
            let slot = usize::from(*selector - b'(');
            let set = match *final_byte {
                b'B' | b'J' | b'6' => CodeSet::Alphanumeric,
                b'0' | b'7' => CodeSet::Hiragana,
                b'1' | b'8' | b'I' => CodeSet::Katakana,
                _ => CodeSet::UnsupportedOneByte,
            };
            Some((EscapeAction::Designate(slot, set), 2))
        }
        [b'$', selector @ (b'(' | b')' | b'*' | b'+'), final_byte, ..] => {
            let slot = usize::from(*selector - b'(');
            let set = match *final_byte {
                b'B' | b'@' => CodeSet::Kanji,
                b';' => CodeSet::AdditionalSymbols,
                _ => CodeSet::UnsupportedTwoByte,
            };
            Some((EscapeAction::Designate(slot, set), 3))
        }
        _ => None,
    }
}

fn arib_alphanumeric(byte: u8) -> char {
    match byte {
        0x5c => '¥',
        0x7e => '‾',
        _ => char::from(byte),
    }
}

fn decode_arib_kanji(first: u8, second: u8) -> Option<char> {
    let ku = usize::from(first.checked_sub(0x21)?);
    let ten = usize::from(second.checked_sub(0x21)?);
    if ku >= 84 {
        return additional_symbols().get((ku - 84) * 94 + ten).copied();
    }
    kanji_table().get(ku * 94 + ten).copied()
}

fn decode_arib_single_byte(table: &[char], byte: u8) -> char {
    byte.checked_sub(0x21)
        .and_then(|index| table.get(usize::from(index)))
        .copied()
        .unwrap_or('〓')
}

fn hiragana_table() -> &'static Vec<char> {
    static TABLE: OnceLock<Vec<char>> = OnceLock::new();
    TABLE.get_or_init(|| b24_conversion_table("kHiraganaTable"))
}

fn katakana_table() -> &'static Vec<char> {
    static TABLE: OnceLock<Vec<char>> = OnceLock::new();
    TABLE.get_or_init(|| b24_conversion_table("kKatakanaTable"))
}

fn kanji_table() -> &'static Vec<char> {
    static TABLE: OnceLock<Vec<char>> = OnceLock::new();
    TABLE.get_or_init(|| b24_conversion_table("kKanjiTable"))
}

fn b24_conversion_table(name: &str) -> Vec<char> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../third_party/libaribcaption/src/decoder/b24_conv_tables.hpp"
    ));
    parse_cpp_unicode_table(source, name)
}

fn parse_cpp_unicode_table(source: &str, name: &str) -> Vec<char> {
    let declaration = format!("{name}[] = {{");
    source
        .split(&declaration)
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .unwrap_or_default()
        .split(',')
        .filter_map(|token| {
            let token = token.trim().strip_prefix("0x")?;
            u32::from_str_radix(token, 16).ok().and_then(char::from_u32)
        })
        .collect()
}

fn additional_symbols() -> &'static Vec<char> {
    static TABLE: OnceLock<Vec<char>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../third_party/libaribcaption/src/decoder/b24_gaiji_table.hpp"
        ));
        let values = source
            .split("kAdditionalSymbolsTable_Unicode[] = {")
            .nth(1)
            .and_then(|tail| tail.split("};").next())
            .unwrap_or_default();
        values
            .split(',')
            .filter_map(|token| {
                let token = token.trim().strip_prefix("0x")?;
                u32::from_str_radix(token, 16).ok().and_then(char::from_u32)
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::decode_service_name;

    #[test]
    fn decodes_arib_locking_shifts_and_kanji_space_in_a_service_name() {
        assert_eq!(
            decode_service_name(&[
                0x0e, b'N', b'H', b'K', 0x0f, 0x21, 0x21, 0x0e, b'B', b'S', b'P', b'4', b'K'
            ]),
            Some("NHK　BSP4K".into())
        );
    }

    #[test]
    fn decodes_designated_hiragana_without_utf8_assumptions() {
        assert_eq!(decode_service_name(&[0x19, 0x21]), Some("ぁ".into()));
    }

    #[test]
    fn keeps_valid_text_before_an_incomplete_kanji() {
        assert_eq!(
            decode_service_name(&[0x0e, b'A', 0x0f, 0x24]),
            Some("A".into())
        );
    }

    #[test]
    fn keeps_surrounding_text_when_an_unsupported_set_is_designated() {
        assert_eq!(
            decode_service_name(&[
                0x0e, b'A', 0x1b, b'$', b')', b'Z', 0x21, 0x21, 0x1b, b')', b'B', 0x0e, b'B',
            ]),
            Some("A〓B".into())
        );
    }

    #[test]
    fn maps_arib_additional_kanji_from_the_pinned_libaribcaption_table() {
        assert_eq!(decode_service_name(&[0x0f, 0x75, 0x21]), Some("㐂".into()));
    }

    #[test]
    fn uses_arib_hiragana_and_katakana_tables_for_long_vowels_and_punctuation() {
        assert_eq!(decode_service_name(&[0x19, 0x79]), Some("ー".into()));
        assert_eq!(decode_service_name(&[0x1d, 0x79]), Some("ー".into()));
        assert_eq!(decode_service_name(&[0x19, 0x7a]), Some("。".into()));
        assert_eq!(decode_service_name(&[0x1d, 0x7e]), Some("・".into()));
    }

    #[test]
    fn supports_direct_g0_additional_symbol_designation() {
        assert_eq!(
            decode_service_name(&[0x1b, b'$', b';', 0x7a, 0x50, 0x0e, b'A']),
            Some("🅊A".into())
        );
    }

    #[test]
    fn unsupported_two_byte_set_does_not_consume_a_following_shift() {
        assert_eq!(
            decode_service_name(&[0x1b, b'$', b'(', b'Z', 0x21, 0x0e, b'A']),
            Some("〓A".into())
        );
    }

    #[test]
    fn decodes_jis_x0213_designations_through_the_pinned_kanji_table() {
        assert_eq!(
            decode_service_name(&[0x1b, b'$', b'9', 0x3f, 0x3c]),
            Some("深".into())
        );
        assert_eq!(
            decode_service_name(&[0x1b, b'$', b':', 0x45, 0x44]),
            Some("田".into())
        );
    }
}
