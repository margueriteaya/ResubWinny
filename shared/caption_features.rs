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
    pub(crate) speaker_cue: bool,
    pub(crate) continuation_cue: bool,
    pub(crate) phone_cue: bool,
    pub(crate) offscreen_cue: bool,
}

pub(crate) struct CaptionSemantics {
    pub(crate) text_accessibility: AccessibilityEvidence,
    pub(crate) declared_accessibility_ranges: Vec<Range<usize>>,
    pub(crate) removable_accessibility_ranges: Vec<Range<usize>>,
}

pub(crate) struct CaptionGroupSemantics {
    pub(crate) fragments: Vec<CaptionSemantics>,
    pub(crate) cross_fragment_delimiter_count: usize,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CaptionSequenceState {
    japanese_quote_stack: Vec<char>,
    semantic_delimiter_closes: Vec<char>,
}

const SEMANTIC_DELIMITER_PAIRS: [(char, char); 6] = [
    ('<', '>'),
    ('＜', '＞'),
    ('≪', '≫'),
    ('《', '》'),
    ('｟', '｠'),
    ('⦅', '⦆'),
];

#[allow(dead_code, reason = "convenience view of the shared semantic result")]
pub(crate) fn accessibility_ranges(text: &str) -> Vec<Range<usize>> {
    caption_semantics(text, &[]).removable_accessibility_ranges
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
        } else if matches!(chars[index], '➡' | '☎' | '⚟') {
            cue_ranges.push(single_range_cue(index..index + 1));
            index += 1;
        } else {
            index += 1;
        }
    }
    let music_cue = chars
        .iter()
        .any(|character| matches!(character, '♪' | '♬' | '♫' | '♩'));
    let continuation_cue = chars.contains(&'➡');
    let phone_cue = chars.contains(&'☎');
    let offscreen_cue = chars.contains(&'⚟');
    let leading_annotation_start = cue_ranges.len();
    for (open, close) in [('(', ')'), ('（', '）')] {
        add_leading_bracket_ranges(&chars, open, close, &mut cue_ranges);
    }
    let leading_annotation = cue_ranges.len() > leading_annotation_start;
    let speaker_cue_start = cue_ranges.len();
    add_leading_speaker_cue_ranges(&chars, &mut cue_ranges);
    let speaker_cue = cue_ranges.len() > speaker_cue_start;
    // Only paired delimiters are candidates. An isolated bracket does not
    // establish narration, even at a line boundary.
    let narration_delimiter_start = cue_ranges.len();
    add_narration_delimiter_ranges(&chars, &mut cue_ranges);
    let mut ranges = cue_ranges.iter().flatten().cloned().collect();
    normalize_ranges(&mut ranges);
    AccessibilityEvidence {
        narration_delimiter: cue_ranges.len() > narration_delimiter_start,
        ranges,
        observed_count: cue_ranges.len(),
        cue_ranges,
        leading_annotation,
        music_cue,
        speaker_cue,
        continuation_cue,
        phone_cue,
        offscreen_cue,
    }
}

pub(crate) fn caption_semantics(
    text: &str,
    declared_accessibility_ranges: &[Range<usize>],
) -> CaptionSemantics {
    let length = text.chars().count();
    let mut declared = declared_accessibility_ranges
        .iter()
        .map(|range| range.start.min(length)..range.end.min(length))
        .filter(|range| range.end > range.start)
        .collect::<Vec<_>>();
    normalize_ranges(&mut declared);

    let text_accessibility = accessibility_evidence(text);
    let mut removable = declared.clone();
    removable.extend(text_accessibility.ranges.iter().cloned());
    normalize_ranges(&mut removable);
    CaptionSemantics {
        text_accessibility,
        declared_accessibility_ranges: declared,
        removable_accessibility_ranges: removable,
    }
}

#[allow(
    dead_code,
    reason = "callers without a caption sequence use a fresh state"
)]
pub(crate) fn caption_group_semantics(
    texts: &[&str],
    declared_accessibility_ranges: &[Vec<Range<usize>>],
) -> CaptionGroupSemantics {
    caption_group_semantics_with_state(
        texts,
        declared_accessibility_ranges,
        &mut CaptionSequenceState::default(),
    )
}

pub(crate) fn caption_group_semantics_with_state(
    texts: &[&str],
    declared_accessibility_ranges: &[Vec<Range<usize>>],
    sequence_state: &mut CaptionSequenceState,
) -> CaptionGroupSemantics {
    let mut fragments = texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            caption_semantics(
                text,
                declared_accessibility_ranges
                    .get(index)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    let characters = texts
        .iter()
        .map(|text| text.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut quote_stack = std::mem::take(&mut sequence_state.japanese_quote_stack);
    for (fragment, chars) in fragments.iter_mut().zip(&characters) {
        if !quote_stack.is_empty() {
            suppress_quoted_leading_parenthetical(fragment, chars);
        }
        update_japanese_quote_stack(&mut quote_stack, chars);
    }
    let continues = caption_group_continues(texts);
    let keeps_sequence_state = texts.is_empty() || continues;
    if keeps_sequence_state {
        sequence_state.japanese_quote_stack = quote_stack;
    }
    let mut inherited_closes = std::mem::take(&mut sequence_state.semantic_delimiter_closes);
    let mut cross_fragment_delimiter_count = 0;
    for (open, close) in SEMANTIC_DELIMITER_PAIRS {
        let mut pending: Option<(usize, usize)> = None;
        let mut inherited = if let Some(index) = inherited_closes
            .iter()
            .position(|candidate| *candidate == close)
        {
            inherited_closes.remove(index);
            true
        } else {
            false
        };
        for (fragment_index, chars) in characters.iter().enumerate() {
            let Some((start, end)) = semantic_content_bounds(chars) else {
                continue;
            };
            let start_is_unmatched = chars[start] == open
                && !fragments[fragment_index]
                    .removable_accessibility_ranges
                    .iter()
                    .any(|range| range.contains(&start));
            let end_is_unmatched = chars[end] == close
                && !fragments[fragment_index]
                    .removable_accessibility_ranges
                    .iter()
                    .any(|range| range.contains(&end));
            if end_is_unmatched && (inherited || pending.is_some()) {
                if let Some((open_fragment, open_index)) = pending.take() {
                    fragments[open_fragment]
                        .removable_accessibility_ranges
                        .push(open_index..open_index + 1);
                    fragments[open_fragment]
                        .text_accessibility
                        .narration_delimiter = true;
                }
                fragments[fragment_index]
                    .removable_accessibility_ranges
                    .push(end..end + 1);
                fragments[fragment_index]
                    .text_accessibility
                    .narration_delimiter = true;
                inherited = false;
                cross_fragment_delimiter_count += 1;
            }
            if start_is_unmatched {
                pending = Some((fragment_index, start));
            }
        }
        if keeps_sequence_state {
            if let Some((open_fragment, open_index)) = pending {
                fragments[open_fragment]
                    .removable_accessibility_ranges
                    .push(open_index..open_index + 1);
                fragments[open_fragment]
                    .text_accessibility
                    .narration_delimiter = true;
                inherited = true;
            }
            if inherited {
                sequence_state.semantic_delimiter_closes.push(close);
            }
        }
    }
    for fragment in &mut fragments {
        normalize_ranges(&mut fragment.removable_accessibility_ranges);
    }
    CaptionGroupSemantics {
        fragments,
        cross_fragment_delimiter_count,
    }
}

fn caption_group_continues(texts: &[&str]) -> bool {
    texts
        .iter()
        .rev()
        .flat_map(|text| text.chars().rev())
        .find(|character| !character.is_whitespace())
        == Some('➡')
}

fn update_japanese_quote_stack(stack: &mut Vec<char>, chars: &[char]) {
    for character in chars {
        match character {
            '「' => stack.push('」'),
            '『' => stack.push('』'),
            '｢' => stack.push('｣'),
            '」' | '』' | '｣' if stack.last() == Some(character) => {
                stack.pop();
            }
            _ => {}
        }
    }
}

fn leading_parenthetical_range(chars: &[char]) -> Option<Range<usize>> {
    let start = chars
        .iter()
        .position(|character| !character.is_whitespace())?;
    let close = match chars[start] {
        '(' => ')',
        '（' => '）',
        _ => return None,
    };
    let end = (start + 1..chars.len()).find(|index| chars[*index] == close)?;
    Some(start..end + 1)
}

fn parenthetical_has_sound_evidence(chars: &[char], range: &Range<usize>) -> bool {
    let content = chars[range.start + 1..range.end - 1]
        .iter()
        .collect::<String>();
    content.ends_with('音')
        || [
            "笑い声",
            "話し声",
            "泣き声",
            "鳴き声",
            "叫び声",
            "歌声",
            "歓声",
        ]
        .iter()
        .any(|ending| content.ends_with(ending))
        || ["拍手", "ノック", "チャイム", "ベル", "アラート"]
            .iter()
            .any(|marker| content.contains(marker))
        || ["鳴る", "鳴く", "吠える"]
            .iter()
            .any(|ending| content.ends_with(ending))
}

fn suppress_quoted_leading_parenthetical(semantics: &mut CaptionSemantics, chars: &[char]) {
    let Some(range) = leading_parenthetical_range(chars) else {
        return;
    };
    if parenthetical_has_sound_evidence(chars, &range) {
        return;
    }
    semantics
        .text_accessibility
        .cue_ranges
        .retain(|cue| cue.as_slice() != std::slice::from_ref(&range));
    semantics.text_accessibility.ranges = semantics
        .text_accessibility
        .cue_ranges
        .iter()
        .flatten()
        .cloned()
        .collect();
    normalize_ranges(&mut semantics.text_accessibility.ranges);
    semantics.text_accessibility.observed_count = semantics.text_accessibility.cue_ranges.len();

    let mut leading_ranges = Vec::new();
    add_leading_bracket_ranges(chars, '(', ')', &mut leading_ranges);
    add_leading_bracket_ranges(chars, '（', '）', &mut leading_ranges);
    semantics.text_accessibility.leading_annotation = leading_ranges.iter().any(|candidate| {
        semantics
            .text_accessibility
            .cue_ranges
            .iter()
            .any(|cue| cue == candidate)
    });
    semantics.removable_accessibility_ranges = semantics.declared_accessibility_ranges.clone();
    semantics
        .removable_accessibility_ranges
        .extend(semantics.text_accessibility.ranges.iter().cloned());
    normalize_ranges(&mut semantics.removable_accessibility_ranges);
}

fn semantic_content_bounds(chars: &[char]) -> Option<(usize, usize)> {
    let mut start = chars
        .iter()
        .position(|character| !character.is_whitespace())?;
    let end = chars
        .iter()
        .rposition(|character| !character.is_whitespace())?;
    if matches!(chars[start], '(' | '（') {
        let close = if chars[start] == '(' { ')' } else { '）' };
        if let Some(annotation_end) = (start + 1..=end).find(|index| chars[*index] == close) {
            start = (annotation_end + 1..=end)
                .find(|index| !chars[*index].is_whitespace())
                .unwrap_or(end);
        }
    }
    Some((start, end))
}

fn normalize_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut normalized: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        if let Some(previous) = normalized.last_mut()
            && range.start < previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            normalized.push(range);
        }
    }
    *ranges = normalized;
}

fn add_leading_speaker_cue_ranges(chars: &[char], cue_ranges: &mut Vec<Vec<Range<usize>>>) {
    let mut line_start = 0;
    while line_start < chars.len() {
        let line_end = chars[line_start..]
            .iter()
            .position(|character| matches!(character, '\n' | '\r'))
            .map_or(chars.len(), |offset| line_start + offset);
        let Some(start) = (line_start..line_end).find(|index| !chars[*index].is_whitespace())
        else {
            line_start = line_end + 1;
            continue;
        };
        let Some(marker) = (start..line_end).find(|index| chars[*index] == '≫') else {
            line_start = line_end + 1;
            continue;
        };
        let has_label = marker > start && chars[start..marker].iter().any(|c| !c.is_whitespace());
        let has_dialogue = chars[marker + 1..line_end]
            .iter()
            .any(|character| !character.is_whitespace());
        let label_has_range_or_sentence_syntax = chars[start..marker].iter().any(|character| {
            matches!(
                character,
                '≪' | '《'
                    | '》'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '｢'
                    | '｣'
                    | '<'
                    | '>'
                    | '＜'
                    | '＞'
                    | '。'
                    | '！'
                    | '？'
                    | '!'
                    | '?'
                    | '♪'
                    | '♬'
                    | '♫'
                    | '♩'
                    | '➡'
            )
        });
        if has_label && has_dialogue && !label_has_range_or_sentence_syntax {
            cue_ranges.push(single_range_cue(start..marker + 1));
        }
        line_start = line_end + 1;
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
        if let (Some(mut start), Some(end)) = (content_start, content_end) {
            if matches!(chars[start], '(' | '（') {
                let close = if chars[start] == '(' { ')' } else { '）' };
                if let Some(annotation_end) = (start + 1..=end).find(|index| chars[*index] == close)
                {
                    start = (annotation_end + 1..=end)
                        .find(|index| !chars[*index].is_whitespace())
                        .unwrap_or(end);
                }
            }
            let close = SEMANTIC_DELIMITER_PAIRS
                .iter()
                .find_map(|(open, close)| (*open == chars[start]).then_some(*close));
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
    let mut line_start = 0;
    while line_start < chars.len() {
        let line_end = chars[line_start..]
            .iter()
            .position(|character| matches!(character, '\n' | '\r'))
            .map_or(chars.len(), |offset| line_start + offset);
        let Some(mut start) = (line_start..line_end).find(|index| !chars[*index].is_whitespace())
        else {
            line_start = line_end.saturating_add(1);
            continue;
        };
        if matches!(chars[start], '☎' | '⚟') {
            start = (start + 1..line_end)
                .find(|index| !chars[*index].is_whitespace())
                .unwrap_or(line_end);
        }
        if start < line_end
            && chars[start] == open
            && let Some(end) = (start + 1..line_end).find(|index| chars[*index] == close)
        {
            cue_ranges.push(single_range_cue(start..end + 1));
        }
        line_start = line_end.saturating_add(1);
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
        caption_semantics(text, additional_accessibility_ranges).removable_accessibility_ranges
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
        let evidence = accessibility_evidence("（話者）本文\n橋本≫進行\n♪〜音楽\n＜語り＞");

        assert!(evidence.leading_annotation);
        assert!(evidence.music_cue);
        assert!(evidence.narration_delimiter);
        assert!(evidence.speaker_cue);
        assert_eq!(evidence.observed_count, 4);
        assert_eq!(evidence.ranges.len(), 5);
        assert_eq!(
            evidence.ranges,
            accessibility_ranges("（話者）本文\n橋本≫進行\n♪〜音楽\n＜語り＞")
        );

        let ordinary = accessibility_evidence("価格（税込）です");
        assert!(ordinary.ranges.is_empty());
        assert_eq!(ordinary.observed_count, 0);
        assert!(!ordinary.leading_annotation);
        assert!(!ordinary.music_cue);
        assert!(!ordinary.narration_delimiter);
        assert!(!ordinary.speaker_cue);
        assert!(!ordinary.continuation_cue);
        assert!(!ordinary.phone_cue);
        assert!(!ordinary.offscreen_cue);
    }

    #[test]
    fn broadcast_sound_and_continuation_marks_are_accessibility_cues() {
        let evidence = accessibility_evidence("⚟画面外☎電話の声 本文➡");
        assert_eq!(evidence.ranges, vec![0..1, 4..5, 12..13]);
        assert_eq!(evidence.observed_count, 3);
        assert!(evidence.offscreen_cue);
        assert!(evidence.phone_cue);
        assert!(evidence.continuation_cue);
        assert_eq!(
            filtered_text("⚟画面外☎電話の声 本文➡", true, false),
            "画面外電話の声 本文"
        );
        assert_eq!(
            accessibility_ranges("☎(リツコ)艦長 Bad Newsよ"),
            vec![0..1, 1..6]
        );
    }

    #[test]
    fn leading_name_and_double_angle_are_a_removable_speaker_cue() {
        assert_eq!(
            accessibility_ranges("橋本≫いよいよ始まりました"),
            vec![0..3]
        );
        assert_eq!(accessibility_ranges("  有吉≫「紅白」！"), vec![2..5]);
        assert_eq!(
            filtered_text("伊藤≫では、そろそろいきましょうか。", true, false),
            "では、そろそろいきましょうか。"
        );
    }

    #[test]
    fn double_angle_range_closures_are_not_speaker_cues() {
        let paired = "≪いっしょに、未来を描いていこう。≫";
        assert_eq!(accessibility_ranges(paired), vec![0..1, 17..18]);
        assert_eq!(
            filtered_text(paired, true, false),
            "いっしょに、未来を描いていこう。"
        );
        let closure = "続く世界｣をつくりたい。≫";
        assert!(accessibility_ranges(closure).is_empty());
        assert_eq!(filtered_text(closure, true, false), closure);
        let nested = accessibility_evidence("(女性A)≪どのオレにする？≫");
        assert!(!nested.speaker_cue);
        assert_eq!(nested.ranges, vec![0..5, 5..6, 14..15]);
        assert_eq!(
            filtered_text("(女性A)≪どのオレにする？≫", true, false),
            "どのオレにする？"
        );
    }

    #[test]
    fn speaker_annotation_can_precede_a_semantic_delimiter() {
        assert_eq!(
            accessibility_ranges("(伊藤)＜花粉に…＞"),
            vec![0..4, 4..5, 9..10]
        );
        assert_eq!(filtered_text("(伊藤)＜花粉に…＞", true, false), "花粉に…");
        assert_eq!(accessibility_ranges("《頼むぞ！》"), vec![0..1, 5..6]);
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

    #[test]
    fn declared_and_text_cues_form_one_broadcast_semantic_result() {
        let semantics = caption_semantics("♪〜前ドア音後➡", &[3..6, 99..100]);
        assert!(semantics.text_accessibility.music_cue);
        assert!(semantics.text_accessibility.continuation_cue);
        assert_eq!(semantics.declared_accessibility_ranges, vec![3..6]);
        assert_eq!(
            semantics.removable_accessibility_ranges,
            vec![0..2, 3..6, 7..8]
        );

        let adjacent_roles = caption_semantics("前後", &[0..1, 1..2]);
        assert_eq!(
            adjacent_roles.declared_accessibility_ranges,
            vec![0..1, 1..2]
        );
    }

    #[test]
    fn semantic_delimiters_pair_across_caption_fragments() {
        let group =
            caption_group_semantics(&["＜たった１錠。", "わたしオン「アレジオン」！＞"], &[]);
        assert_eq!(group.cross_fragment_delimiter_count, 1);
        assert_eq!(
            group.fragments[0].removable_accessibility_ranges,
            vec![0..1]
        );
        assert_eq!(
            group.fragments[1].removable_accessibility_ranges,
            vec![13..14]
        );

        let unrelated = caption_group_semantics(&["1＜2", "価格＞税込"], &[]);
        assert_eq!(unrelated.cross_fragment_delimiter_count, 0);
        assert!(
            unrelated
                .fragments
                .iter()
                .all(|fragment| fragment.removable_accessibility_ranges.is_empty())
        );

        for (first, second) in [
            ("<語りは", "続きます>"),
            ("＜語りは", "続きます＞"),
            ("≪心の声は", "続きます≫"),
            ("《回想は", "続きます》"),
            ("（伊藤）＜説明は", "続きます＞"),
        ] {
            let group = caption_group_semantics(&[first, second], &[]);
            assert_eq!(group.cross_fragment_delimiter_count, 1, "{first}{second}");
            assert!(!group.fragments[0].removable_accessibility_ranges.is_empty());
            assert!(!group.fragments[1].removable_accessibility_ranges.is_empty());
        }
    }

    #[test]
    fn japanese_quote_state_distinguishes_title_parentheses_from_sound_cues() {
        let title = caption_group_semantics(
            &["曲「スゥ・ル・シエル・ド・パリ", "（パリの空の下）」。"],
            &[],
        );
        assert!(title.fragments[1].removable_accessibility_ranges.is_empty());
        assert!(!title.fragments[1].text_accessibility.leading_annotation);
        let title_with_voice_word = caption_group_semantics(&["曲「", "（君の声）」"], &[]);
        assert!(
            title_with_voice_word.fragments[1]
                .removable_accessibility_ranges
                .is_empty()
        );

        for sound in ["（拍手）」", "（笑い声）」", "（ノック）」", "（爆発音）」"]
        {
            let group = caption_group_semantics(&["「会場から音が聞こえる", sound], &[]);
            let end = sound
                .chars()
                .position(|character| character == '）')
                .unwrap()
                + 1;
            assert_eq!(
                group.fragments[1].removable_accessibility_ranges,
                vec![0..end],
                "{sound}"
            );
        }

        let outside_quote = caption_group_semantics(&["本文", "（伊藤）説明"], &[]);
        assert_eq!(
            outside_quote.fragments[1].removable_accessibility_ranges,
            vec![0..4]
        );
    }

    #[test]
    fn continuation_arrow_carries_quote_state_to_exactly_the_next_caption_page() {
        let mut state = CaptionSequenceState::default();
        let first = caption_group_semantics_with_state(
            &["曲「スゥ・ル・シエル・ド・パリ➡"],
            &[],
            &mut state,
        );
        assert_eq!(
            first.fragments[0].removable_accessibility_ranges,
            vec![15..16]
        );
        caption_group_semantics_with_state(&[], &[], &mut state);
        let second = caption_group_semantics_with_state(&["（パリの空の下）」。"], &[], &mut state);
        assert!(
            second.fragments[0]
                .removable_accessibility_ranges
                .is_empty()
        );

        let mut unlinked = CaptionSequenceState::default();
        caption_group_semantics_with_state(&["曲「スゥ・ル・シエル・ド・パリ"], &[], &mut unlinked);
        let next =
            caption_group_semantics_with_state(&["（パリの空の下）」。"], &[], &mut unlinked);
        assert_eq!(next.fragments[0].removable_accessibility_ranges, vec![0..8]);
    }

    #[test]
    fn continuation_arrow_carries_broadcast_delimiters_to_the_next_page() {
        for (open, close) in [('｟', '｠'), ('⦅', '⦆'), ('＜', '＞'), ('《', '》')] {
            let mut state = CaptionSequenceState::default();
            let first_text = format!("(加寿彦){open}ダークエネルギーっていうのは➡");
            let first = caption_group_semantics_with_state(&[&first_text], &[], &mut state);
            let open_index = first_text
                .chars()
                .position(|character| character == open)
                .unwrap();
            assert!(
                first.fragments[0]
                    .removable_accessibility_ranges
                    .contains(&(open_index..open_index + 1)),
                "{first_text}"
            );

            caption_group_semantics_with_state(&[], &[], &mut state);
            let second_text = format!("作用する{close}");
            let second = caption_group_semantics_with_state(&[&second_text], &[], &mut state);
            let close_index = second_text.chars().count() - 1;
            assert!(
                second.fragments[0]
                    .removable_accessibility_ranges
                    .contains(&(close_index..close_index + 1)),
                "{second_text}"
            );
        }

        let mut unlinked = CaptionSequenceState::default();
        caption_group_semantics_with_state(&["｟前の字幕"], &[], &mut unlinked);
        let next = caption_group_semantics_with_state(&["次の字幕｠"], &[], &mut unlinked);
        assert!(next.fragments[0].removable_accessibility_ranges.is_empty());
    }

    #[test]
    fn japanese_broadcast_notation_examples_form_a_fixed_semantic_matrix() {
        // NHK G-Media and the domestic captioned-CM handbook describe these
        // editorial uses. Transport-specific B24/B62 evidence enters the same
        // semantic classifier separately.
        let cases = [
            ("（鈴木）こんにちは。", "こんにちは。"),
            ("（笑い声）本文", "本文"),
            ("（ノック）本文", "本文"),
            ("☎(リツコ)艦長 Bad Newsよ", "艦長 Bad Newsよ"),
            ("⚟画面の外からの声", "画面の外からの声"),
            ("橋本≫進行します", "進行します"),
            ("♪〜音楽", "音楽"),
            ("＜ナレーション＞", "ナレーション"),
            ("《心の声》", "心の声"),
            ("｟フィルター音声｠", "フィルター音声"),
            ("⦅フィルター音声⦆", "フィルター音声"),
            ("本文➡", "本文"),
        ];
        for (source, expected) in cases {
            assert_eq!(filtered_text(source, true, false), expected, "{source}");
        }

        assert_eq!(
            filtered_text("♪「僕らは自由だね」君の声が", true, false),
            "「僕らは自由だね」君の声が"
        );
        for ordinary_text in ["価格（税込）です", "「商品名」", "続く世界｣をつくりたい。≫"]
        {
            assert_eq!(
                filtered_text(ordinary_text, true, false),
                ordinary_text,
                "{ordinary_text}"
            );
        }
    }
}
