use std::{collections::BTreeSet, sync::OnceLock};

// Keep this classifier tied to the pinned libaribcaption table. Rows 85-86
// are additional kanji and are intentionally excluded; only rows 90-94 are
// ARIB additional symbols used for special marks and pictograms.
const GAIJI_TABLE_SOURCE: &str =
    include_str!("../third_party/libaribcaption/src/decoder/b24_gaiji_table.hpp");
const ROW_WIDTH: usize = 94;
const ADDITIONAL_SYMBOL_START: usize = 5 * ROW_WIDTH;
const ADDITIONAL_SYMBOL_LEN: usize = 5 * ROW_WIDTH;

static ADDITIONAL_SYMBOLS: OnceLock<BTreeSet<u32>> = OnceLock::new();

pub(crate) fn is_arib_additional_symbol(character: char) -> bool {
    is_arib_additional_symbol_codepoint(character as u32)
}

pub(crate) fn is_arib_additional_symbol_codepoint(codepoint: u32) -> bool {
    additional_symbols().contains(&codepoint) && !is_daily_japanese_text(codepoint)
}

fn is_daily_japanese_text(codepoint: u32) -> bool {
    matches!(
        codepoint,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
            | 0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0x31F0..=0x31FF
            | 0x1B000..=0x1B16F
            | 0x1AFF0..=0x1AFFF
            | 0xFF66..=0xFF9D
    )
}

fn additional_symbols() -> &'static BTreeSet<u32> {
    ADDITIONAL_SYMBOLS.get_or_init(|| {
        [
            "kAdditionalSymbolsTable_Unicode",
            "kAdditionalSymbolsTable_PUA",
        ]
        .into_iter()
        .flat_map(|name| {
            table_values(name)
                .into_iter()
                .skip(ADDITIONAL_SYMBOL_START)
                .take(ADDITIONAL_SYMBOL_LEN)
        })
        .filter(|codepoint| *codepoint != 0xFFFD)
        .collect()
    })
}

fn table_values(name: &str) -> Vec<u32> {
    let declaration = format!("{name}[] = {{");
    let body = GAIJI_TABLE_SOURCE
        .split_once(&declaration)
        .and_then(|(_, rest)| rest.split_once("};"))
        .map(|(body, _)| body)
        .expect("pinned libaribcaption gaiji table must retain its declarations");
    let values = body
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter_map(|token| token.strip_prefix("0x"))
        .map(|hex| u32::from_str_radix(hex, 16).expect("gaiji table must be hexadecimal"))
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 10 * ROW_WIDTH);
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_special_symbols_but_excludes_daily_text_and_additional_kanji() {
        assert!(is_arib_additional_symbol('➡'));
        assert!(is_arib_additional_symbol('⚟'));
        assert!(is_arib_additional_symbol('♬'));
        assert!(!is_arib_additional_symbol('年'));
        assert!(!is_arib_additional_symbol('カ'));
        assert!(!is_arib_additional_symbol('→'));
    }
}
