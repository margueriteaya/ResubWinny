use std::ops::Range;

pub(crate) fn gaiji_ranges(text: &str) -> Vec<Range<usize>> {
    text.chars()
        .enumerate()
        .filter(|(_, character)| super::arib_symbols::is_arib_additional_symbol(*character))
        .map(|(index, _)| index..index + 1)
        .collect()
}

#[allow(
    dead_code,
    reason = "the Worker reports detail flags while the desktop inspector shares the ranges"
)]
pub(crate) struct AccessibilityEvidence {
    pub(crate) ranges: Vec<Range<usize>>,
    pub(crate) cue_ranges: Vec<Vec<Range<usize>>>,
    pub(crate) observed_count: usize,
    pub(crate) leading_annotation: bool,
    pub(crate) music_cue: bool,
    pub(crate) narration_delimiter: bool,
}

pub(crate) fn accessibility_ranges(text: &str) -> Vec<Range<usize>> {
    accessibility_evidence(text).ranges
}

fn single_range_cue(range: Range<usize>) -> Vec<Range<usize>> {
    std::iter::once(range).collect()
}

pub(crate) fn accessibility_evidence(text: &str) -> AccessibilityEvidence {
    let chars = text.chars().collect::<Vec<_>>();
    let mut cue_ranges = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if matches!(chars[index], '♪' | '♬' | '♫' | '♩') {
            let mut end = index + 1;
            while end < chars.len() && matches!(chars[end], '～' | '〜' | '~') {
                end += 1;
            }
            cue_ranges.push(single_range_cue(index..end));
            index = end;
        } else {
            index += 1;
        }
    }
    let music_cue = !cue_ranges.is_empty();
    let leading_annotation_start = cue_ranges.len();
    for (open, close) in [('(', ')'), ('（', '）')] {
        add_leading_bracket_ranges(&chars, open, close, &mut cue_ranges);
    }
    let leading_annotation = cue_ranges.len() > leading_annotation_start;
    // Only paired delimiters are candidates. An isolated bracket does not
    // establish narration, even at a line boundary.
    let narration_delimiter_start = cue_ranges.len();
    add_narration_delimiter_ranges(&chars, &mut cue_ranges);
    let ranges = cue_ranges.iter().flatten().cloned().collect();
    AccessibilityEvidence {
        narration_delimiter: cue_ranges.len() > narration_delimiter_start,
        ranges,
        observed_count: cue_ranges.len(),
        cue_ranges,
        leading_annotation,
        music_cue,
    }
}

fn add_narration_delimiter_ranges(chars: &[char], cue_ranges: &mut Vec<Vec<Range<usize>>>) {
    let mut line_start = 0;
    while line_start < chars.len() {
        let line_end = chars[line_start..]
            .iter()
            .position(|character| matches!(character, '\n' | '\r'))
            .map_or(chars.len(), |offset| line_start + offset);
        let content_start = (line_start..line_end).find(|index| !chars[*index].is_whitespace());
        let content_end = (line_start..line_end)
            .rev()
            .find(|index| !chars[*index].is_whitespace());
        if let (Some(start), Some(end)) = (content_start, content_end) {
            let close = match chars[start] {
                '<' => Some('>'),
                '＜' => Some('＞'),
                _ => None,
            };
            if let Some(close) = close {
                let mut ranges = Vec::with_capacity(2);
                if let Some(index) = (start + 1..=end).find(|index| chars[*index] == close) {
                    ranges.push(start..start + 1);
                    ranges.push(index..index + 1);
                    cue_ranges.push(ranges);
                }
            }
        }
        line_start = line_end + 1;
    }
}

fn add_leading_bracket_ranges(
    chars: &[char],
    open: char,
    close: char,
    cue_ranges: &mut Vec<Vec<Range<usize>>>,
) {
    let mut start = None;
    let mut only_leading_whitespace = true;
    for (index, character) in chars.iter().enumerate() {
        if *character == '\n' || *character == '\r' {
            start = None;
            only_leading_whitespace = true;
        } else if start.is_none() && only_leading_whitespace && *character == open {
            start = Some(index);
            only_leading_whitespace = false;
        } else if *character == close
            && let Some(begin) = start.take()
        {
            cue_ranges.push(single_range_cue(begin..index + 1));
        } else if start.is_none() && !character.is_whitespace() {
            only_leading_whitespace = false;
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
    retained_characters_with_accessibility_ranges(text, preserve_gaiji, preserve_accessibility, &[])
}

pub(crate) fn retained_characters_with_accessibility_ranges(
    text: &str,
    preserve_gaiji: bool,
    preserve_accessibility: bool,
    additional_accessibility_ranges: &[Range<usize>],
) -> Vec<bool> {
    let length = text.chars().count();
    if preserve_gaiji && preserve_accessibility {
        return vec![true; length];
    }
    let gaiji = (!preserve_gaiji).then(|| gaiji_ranges(text));
    let accessibility = (!preserve_accessibility).then(|| {
        let mut ranges = accessibility_ranges(text);
        ranges.extend_from_slice(additional_accessibility_ranges);
        ranges
    });
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
    fn isolated_delimiters_remain_source_text() {
        assert!(accessibility_ranges("<語り").is_empty());
        assert!(accessibility_ranges("続き>").is_empty());
        assert!(accessibility_ranges("本文\n ＜語り").is_empty());
        assert!(accessibility_ranges("続き＞  \n本文").is_empty());
        assert_eq!(filtered_text("＜語り", true, false), "＜語り");
        assert_eq!(filtered_text("続き＞", true, false), "続き＞");
    }

    #[test]
    fn inline_angle_brackets_remain_ordinary_caption_text() {
        for text in ["1＜2", "価格<税込>です", "A > B", "演算子 >", "比較 ＞"] {
            assert!(accessibility_ranges(text).is_empty());
            assert_eq!(filtered_text(text, true, false), text);
        }
    }

    #[test]
    fn line_start_narration_pair_marks_only_its_delimiters() {
        assert_eq!(
            accessibility_ranges("<ついに！>語りは残す"),
            vec![0..1, 5..6]
        );
        assert_eq!(accessibility_ranges(" ＜説明＞本文"), vec![1..2, 4..5]);
    }

    #[test]
    fn parentheses_are_accessibility_only_at_a_caption_line_start() {
        assert_eq!(accessibility_ranges("（シンジ）本文"), vec![0..5]);
        assert_eq!(accessibility_ranges("  (speaker) text"), vec![2..11]);
        assert_eq!(accessibility_ranges("本文\n（拍手）"), vec![3..7]);
        assert!(accessibility_ranges("価格（税込）です").is_empty());
        assert_eq!(
            filtered_text("価格（税込）です", true, false),
            "価格（税込）です"
        );
    }

    #[test]
    fn accessibility_evidence_keeps_text_patterns_distinct() {
        let evidence = accessibility_evidence("（話者）本文\n♪〜音楽\n＜語り＞");

        assert!(evidence.leading_annotation);
        assert!(evidence.music_cue);
        assert!(evidence.narration_delimiter);
        assert_eq!(evidence.observed_count, 3);
        assert_eq!(evidence.ranges.len(), 4);
        assert_eq!(
            evidence.ranges,
            accessibility_ranges("（話者）本文\n♪〜音楽\n＜語り＞")
        );

        let ordinary = accessibility_evidence("価格（税込）です");
        assert!(ordinary.ranges.is_empty());
        assert_eq!(ordinary.observed_count, 0);
        assert!(!ordinary.leading_annotation);
        assert!(!ordinary.music_cue);
        assert!(!ordinary.narration_delimiter);
    }

    #[test]
    fn explicit_accessibility_ranges_share_the_export_mask() {
        let ranges = std::iter::once(1..4).collect::<Vec<_>>();
        let retained =
            retained_characters_with_accessibility_ranges("前ドア音後", true, false, &ranges);
        let filtered = "前ドア音後"
            .chars()
            .zip(retained)
            .filter(|(_, keep)| *keep)
            .map(|(character, _)| character)
            .collect::<String>();

        assert_eq!(filtered, "前後");
    }
}
