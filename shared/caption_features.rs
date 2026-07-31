use std::ops::Range;

pub(crate) fn gaiji_ranges(text: &str) -> Vec<Range<usize>> {
    text.chars()
        .enumerate()
        .filter(|(_, character)| super::arib_symbols::is_arib_additional_symbol(*character))
        .map(|(index, _)| index..index + 1)
        .collect()
}

pub(crate) fn accessibility_ranges(text: &str) -> Vec<Range<usize>> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if matches!(chars[index], '♪' | '♬' | '♫' | '♩') {
            let mut end = index + 1;
            while end < chars.len() && matches!(chars[end], '～' | '〜' | '~') {
                end += 1;
            }
            ranges.push(index..end);
            index = end;
        } else {
            index += 1;
        }
    }
    for (open, close) in [('(', ')'), ('（', '）')] {
        add_bracket_ranges(&chars, open, close, true, &mut ranges);
    }
    // Narration brackets may be split across regions or consecutive archive
    // records. Their contents are spoken text and must remain, so each
    // delimiter is independently classifiable without its matching partner.
    ranges.extend(
        chars
            .iter()
            .enumerate()
            .filter(|(_, character)| matches!(character, '<' | '>' | '＜' | '＞'))
            .map(|(index, _)| index..index + 1),
    );
    ranges
}

fn add_bracket_ranges(
    chars: &[char],
    open: char,
    close: char,
    include_contents: bool,
    ranges: &mut Vec<Range<usize>>,
) {
    let mut starts = Vec::new();
    for (index, character) in chars.iter().enumerate() {
        if *character == open {
            starts.push(index);
        } else if *character == close
            && let Some(begin) = starts.pop()
        {
            if include_contents {
                ranges.push(begin..index + 1);
            } else {
                ranges.push(begin..begin + 1);
                ranges.push(index..index + 1);
            }
        }
    }
}

#[allow(
    dead_code,
    reason = "the desktop inspector uses ranges while the Worker also filters text"
)]
pub(crate) fn filtered_text(
    text: &str,
    preserve_gaiji: bool,
    preserve_accessibility: bool,
) -> String {
    text.chars()
        .zip(retained_characters(
            text,
            preserve_gaiji,
            preserve_accessibility,
        ))
        .filter(|(_, retained)| *retained)
        .map(|(character, _)| character)
        .collect()
}

#[allow(
    dead_code,
    reason = "the Worker maps retained characters back to styled source cells"
)]
pub(crate) fn retained_characters(
    text: &str,
    preserve_gaiji: bool,
    preserve_accessibility: bool,
) -> Vec<bool> {
    let length = text.chars().count();
    if preserve_gaiji && preserve_accessibility {
        return vec![true; length];
    }
    let gaiji = (!preserve_gaiji).then(|| gaiji_ranges(text));
    let accessibility = (!preserve_accessibility).then(|| accessibility_ranges(text));
    (0..length)
        .map(|index| {
            !gaiji
                .as_ref()
                .is_some_and(|ranges| ranges.iter().any(|range| range.contains(&index)))
                && !accessibility
                    .as_ref()
                    .is_some_and(|ranges| ranges.iter().any(|range| range.contains(&index)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_marks_include_only_their_following_wave_marks() {
        assert_eq!(accessibility_ranges("♪〜本文〜"), vec![0..2]);
        assert!(accessibility_ranges("人物の語調〜").is_empty());
        assert!(gaiji_ranges("人物の語調〜").is_empty());
    }

    #[test]
    fn narration_delimiters_do_not_require_their_partner_in_one_text_segment() {
        assert_eq!(accessibility_ranges("<語り"), vec![0..1]);
        assert_eq!(accessibility_ranges("続き>"), vec![2..3]);
        assert_eq!(filtered_text("＜語り", true, false), "語り");
        assert_eq!(filtered_text("続き＞", true, false), "続き");
    }
}
