//! Conversion from WordprocessingML flow values to shared layout values.

use oxml_layout::{
    Align, LayoutLine, LineBreakParams, LineSpacing, TabAlign, TabLeader, TabStop, TextDirection,
    Underline,
};
use rdocx_oxml::borders::CT_TabStop;
use rdocx_oxml::properties::CT_PPr;
use rdocx_oxml::shared::{ST_Jc, ST_TabJc, ST_TabLeader, ST_Underline};

pub(crate) fn alignment(value: Option<ST_Jc>) -> Option<Align> {
    value.map(|value| match value {
        ST_Jc::Start | ST_Jc::Left => Align::Start,
        ST_Jc::Center => Align::Center,
        ST_Jc::End | ST_Jc::Right => Align::End,
        ST_Jc::Both => Align::Justify,
        ST_Jc::Distribute => Align::Distribute,
    })
}

pub(crate) fn alignment_for_direction(
    value: Option<ST_Jc>,
    direction: TextDirection,
) -> Option<Align> {
    value.map(|value| match (value, direction) {
        (ST_Jc::Start, TextDirection::RightToLeft) => Align::End,
        (ST_Jc::End, TextDirection::RightToLeft) => Align::Start,
        (ST_Jc::Start | ST_Jc::Left, _) => Align::Start,
        (ST_Jc::End | ST_Jc::Right, _) => Align::End,
        (ST_Jc::Center, _) => Align::Center,
        (ST_Jc::Both, _) => Align::Justify,
        (ST_Jc::Distribute, _) => Align::Distribute,
    })
}

pub(crate) fn tab_stops(values: &[CT_TabStop]) -> Vec<TabStop> {
    values
        .iter()
        .filter_map(|value| {
            let align = match value.val {
                ST_TabJc::Left => TabAlign::Left,
                ST_TabJc::Center => TabAlign::Center,
                ST_TabJc::Right => TabAlign::Right,
                ST_TabJc::Decimal | ST_TabJc::Num => TabAlign::Decimal,
                ST_TabJc::Bar => TabAlign::Bar,
                ST_TabJc::Clear => return None,
            };
            let leader = value.leader.map(|leader| match leader {
                ST_TabLeader::None => TabLeader::None,
                ST_TabLeader::Dot => TabLeader::Dot,
                ST_TabLeader::Hyphen => TabLeader::Hyphen,
                ST_TabLeader::Underscore => TabLeader::Underscore,
                ST_TabLeader::Heavy => TabLeader::Heavy,
                ST_TabLeader::MiddleDot => TabLeader::MiddleDot,
            });
            Some(TabStop {
                pos_pt: value.pos.to_pt(),
                align,
                leader,
            })
        })
        .collect()
}

pub(crate) fn underline(value: Option<ST_Underline>) -> Option<Underline> {
    value.and_then(|value| match value {
        ST_Underline::None => None,
        ST_Underline::Single => Some(Underline::Single),
        ST_Underline::Words => Some(Underline::Words),
        ST_Underline::Double => Some(Underline::Double),
        ST_Underline::Thick => Some(Underline::Thick),
        ST_Underline::Dotted => Some(Underline::Dotted),
        ST_Underline::Dash => Some(Underline::Dash),
        ST_Underline::DotDash => Some(Underline::DotDash),
        ST_Underline::DotDotDash => Some(Underline::DotDotDash),
        ST_Underline::Wave => Some(Underline::Wave),
    })
}

pub(crate) fn line_spacing(properties: &CT_PPr) -> LineSpacing {
    match (properties.line_spacing, properties.line_rule.as_deref()) {
        // Non-Word sentinel used by non-DOCX frontends (e.g. rodf): keep the
        // shared engine's natural, line-gap-inclusive height untouched.
        (_, Some("font-natural")) => LineSpacing::Single,
        (Some(spacing), Some("exact")) => LineSpacing::Exact(spacing.to_pt()),
        (Some(spacing), Some("atLeast")) => LineSpacing::AtLeast(spacing.to_pt()),
        (Some(spacing), _) => LineSpacing::Multiple(spacing.0 as f64 / 240.0),
        (None, _) => LineSpacing::Single,
    }
}

pub(crate) fn line_break_params(properties: &CT_PPr, available_width: f64) -> LineBreakParams {
    LineBreakParams {
        line_prefix_widths: Vec::new(),
        line_suffix_widths: Vec::new(),
        available_width,
        ind_left: properties.ind_left.map_or(0.0, |value| value.to_pt()),
        ind_right: properties.ind_right.map_or(0.0, |value| value.to_pt()),
        ind_first_line: properties.ind_first_line.map_or(0.0, |value| value.to_pt()),
        ind_hanging: properties.ind_hanging.map_or(0.0, |value| value.to_pt()),
        tab_stops: properties
            .tabs
            .as_ref()
            .map_or_else(Vec::new, |tabs| tab_stops(&tabs.tabs)),
        line_spacing: line_spacing(properties),
        jc: alignment(properties.jc),
        wrap: true,
        default_tab_interval_pt: 36.0, // Word 기본 0.5"
        // The "font-natural" sentinel marks LibreOffice-convention layout
        // (non-Word frontends): Korean text then wraps by word (어절), not
        // by syllable as Word does.
        hangul_word_wrap: properties.line_rule.as_deref() == Some("font-natural"),
    }
}

/// Hangul as LibreOffice word wrap sees it (어절 단위 줄바꿈).
fn is_hangul(c: char) -> bool {
    matches!(u32::from(c),
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF)
}

pub(crate) fn text_segments(segment: TextSegment, hangul_word_wrap: bool) -> Vec<InlineItem> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    for (end, _) in linebreaks(&segment.text) {
        if end > start {
            // 한글 어절 단위 모드: 음절 사이 분할점은 직전 구간과 병합해
            // 어절이 하나의 아이템으로 남게 한다 (아이템 경계 = 줄바꿈 기회).
            let suppress = hangul_word_wrap
                && end < segment.text.len()
                && segment.text[..end].chars().next_back().is_some_and(is_hangul)
                && segment.text[end..].chars().next().is_some_and(is_hangul);
            if suppress {
                if let Some(last) = ranges.last_mut() {
                    last.1 = end;
                } else {
                    ranges.push((start, end));
                }
            } else {
                ranges.push((start, end));
            }
            start = end;
        }
    }
    if ranges.is_empty() && !segment.text.is_empty() {
        ranges.push((0, segment.text.len()));
    }

    ranges
        .into_iter()
        .map(|(start, end)| InlineItem::Text(slice_text_segment(&segment, start, end)))
        .collect()
}

pub(crate) fn restore_word_line_heights(lines: &mut [LayoutLine], properties: &CT_PPr) {
    // "font-natural" opts out of Word line-height emulation entirely: the
    // shared layout's height (ascent + descent + line gap) and the line's
    // gap value are preserved so the paginator can seat the gap above the
    // line, matching font-metric renderers such as LibreOffice.
    if properties.line_rule.as_deref() == Some("font-natural") {
        return;
    }
    for line in lines {
        let natural = line.ascent + line.descent;
        let natural = if natural < 1.0 { 12.0 } else { natural };
        line.line_gap = 0.0;
        line.height = match (properties.line_spacing, properties.line_rule.as_deref()) {
            (Some(spacing), Some("exact")) => spacing.to_pt(),
            (Some(spacing), Some("atLeast")) => natural.max(spacing.to_pt()),
            (Some(spacing), _) => natural * spacing.0 as f64 / 240.0,
            (None, _) => natural,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxml_layout::{Align, LineSpacing, TabAlign, TabLeader, Underline};
    use rdocx_oxml::units::Twips;

    #[test]
    fn every_word_alignment_maps_to_the_shared_alignment() {
        let cases = [
            (ST_Jc::Start, Align::Start),
            (ST_Jc::Left, Align::Start),
            (ST_Jc::Center, Align::Center),
            (ST_Jc::End, Align::End),
            (ST_Jc::Right, Align::End),
            (ST_Jc::Both, Align::Justify),
            (ST_Jc::Distribute, Align::Distribute),
        ];
        for (word, shared) in cases {
            assert_eq!(alignment(Some(word)), Some(shared));
        }
        assert_eq!(alignment(None), None);
    }

    #[test]
    fn logical_alignment_uses_the_paragraph_base_direction() {
        assert_eq!(
            alignment_for_direction(Some(ST_Jc::Start), TextDirection::RightToLeft),
            Some(Align::End)
        );
        assert_eq!(
            alignment_for_direction(Some(ST_Jc::End), TextDirection::RightToLeft),
            Some(Align::Start)
        );
        assert_eq!(
            alignment_for_direction(Some(ST_Jc::Left), TextDirection::RightToLeft),
            Some(Align::Start)
        );
        assert_eq!(
            alignment_for_direction(Some(ST_Jc::Right), TextDirection::LeftToRight),
            Some(Align::End)
        );
    }

    #[test]
    fn every_word_tab_and_leader_maps_without_rounding_its_position() {
        let cases = [
            (ST_TabJc::Left, TabAlign::Left),
            (ST_TabJc::Center, TabAlign::Center),
            (ST_TabJc::Right, TabAlign::Right),
            (ST_TabJc::Decimal, TabAlign::Decimal),
            (ST_TabJc::Bar, TabAlign::Bar),
            (ST_TabJc::Num, TabAlign::Decimal),
        ];
        let leaders = [
            (ST_TabLeader::None, TabLeader::None),
            (ST_TabLeader::Dot, TabLeader::Dot),
            (ST_TabLeader::Hyphen, TabLeader::Hyphen),
            (ST_TabLeader::Underscore, TabLeader::Underscore),
            (ST_TabLeader::Heavy, TabLeader::Heavy),
            (ST_TabLeader::MiddleDot, TabLeader::MiddleDot),
        ];

        for ((word_align, shared_align), (word_leader, shared_leader)) in
            cases.into_iter().zip(leaders)
        {
            let converted = tab_stops(&[CT_TabStop {
                val: word_align,
                pos: Twips(1),
                leader: Some(word_leader),
                source_occurrence: None,
            }]);
            assert_eq!(converted.len(), 1);
            assert_eq!(converted[0].align, shared_align);
            assert_eq!(converted[0].leader, Some(shared_leader));
            assert!((converted[0].pos_pt - 0.05).abs() < f64::EPSILON);
        }

        assert!(tab_stops(&[CT_TabStop::new(ST_TabJc::Clear, Twips(720))]).is_empty());
    }

    #[test]
    fn every_rendered_word_underline_maps_to_the_shared_style() {
        let cases = [
            (ST_Underline::Single, Underline::Single),
            (ST_Underline::Words, Underline::Words),
            (ST_Underline::Double, Underline::Double),
            (ST_Underline::Thick, Underline::Thick),
            (ST_Underline::Dotted, Underline::Dotted),
            (ST_Underline::Dash, Underline::Dash),
            (ST_Underline::DotDash, Underline::DotDash),
            (ST_Underline::DotDotDash, Underline::DotDotDash),
            (ST_Underline::Wave, Underline::Wave),
        ];
        for (word, shared) in cases {
            assert_eq!(underline(Some(word)), Some(shared));
        }
        assert_eq!(underline(Some(ST_Underline::None)), None);
        assert_eq!(underline(None), None);
    }

    #[test]
    fn every_word_line_spacing_rule_maps_exactly() {
        let cases = [
            (Some(Twips(1)), Some("exact"), LineSpacing::Exact(0.05)),
            (
                Some(Twips(480)),
                Some("atLeast"),
                LineSpacing::AtLeast(24.0),
            ),
            (Some(Twips(360)), Some("auto"), LineSpacing::Multiple(1.5)),
            (Some(Twips(240)), None, LineSpacing::Multiple(1.0)),
            (None, None, LineSpacing::Single),
        ];
        for (spacing, rule, expected) in cases {
            let properties = CT_PPr {
                line_spacing: spacing,
                line_rule: rule.map(str::to_owned),
                ..Default::default()
            };
            assert_eq!(line_spacing(&properties), expected);
        }
    }

    #[test]
    fn word_line_parameters_keep_wrap_enabled() {
        let params = line_break_params(&CT_PPr::default(), 321.0);
        assert_eq!(params.available_width, 321.0);
        assert!(params.wrap);
    }

    #[test]
    fn word_text_preserves_pre_cutover_glyph_slices_at_wrap_boundaries() {
        use oxml_layout::{Color, FontId};

        let segment = TextSegment {
            text: "one two".to_string(),
            source: Some(oxml_layout::SourceSpan {
                node: oxml_layout::SourceNodeId::new(3).expect("non-zero source"),
                char_start: 20,
                char_end: 27,
            }),
            font_id: FontId(1),
            font_size: 12.0,
            glyph_ids: (1..=7).collect(),
            advances: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            width: 28.0,
            ascent: 9.0,
            descent: 3.0,
            line_gap: 0.0,
            color: Color::BLACK,
            bold: false,
            italic: false,
            underline: None,
            strike: false,
            dstrike: false,
            highlight: None,
            baseline_offset: 0.0,
            hyperlink_url: None,
            field_kind: None,
            note: None,
        };
        let items = text_segments(segment, false);
        assert_eq!(items.len(), 2);
        let InlineItem::Text(first) = &items[0] else {
            panic!("expected a text segment");
        };
        assert_eq!(first.text, "one ");
        assert_eq!(first.glyph_ids, vec![1, 2, 3, 4]);
        assert_eq!(first.width, 10.0);
        assert_eq!(
            first.source,
            Some(oxml_layout::SourceSpan {
                node: oxml_layout::SourceNodeId::new(3).expect("non-zero source"),
                char_start: 20,
                char_end: 24,
            })
        );
        let InlineItem::Text(second) = &items[1] else {
            panic!("expected a text segment");
        };
        assert_eq!(second.source.expect("second source").char_start, 24);
        assert_eq!(second.source.expect("second source").char_end, 27);
    }

    #[test]
    fn word_unicode_split_counts_scalars_instead_of_utf8_bytes() {
        use oxml_layout::{Color, FontId, SourceNodeId, SourceSpan};

        let segment = TextSegment {
            text: "A🚀界Z".to_owned(),
            source: Some(SourceSpan {
                node: SourceNodeId::new(9).expect("non-zero source"),
                char_start: 100,
                char_end: 104,
            }),
            font_id: FontId(1),
            font_size: 12.0,
            glyph_ids: vec![1, 2, 3, 4],
            advances: vec![1.0; 4],
            width: 4.0,
            ascent: 9.0,
            descent: 3.0,
            line_gap: 0.0,
            color: Color::BLACK,
            bold: false,
            italic: false,
            underline: None,
            strike: false,
            dstrike: false,
            highlight: None,
            baseline_offset: 0.0,
            hyperlink_url: None,
            field_kind: None,
            note: None,
        };

        let middle = slice_text_segment(&segment, 1, 8);
        assert_eq!(middle.text, "🚀界");
        assert_eq!(
            middle.source,
            Some(SourceSpan {
                node: SourceNodeId::new(9).expect("non-zero source"),
                char_start: 101,
                char_end: 103,
            })
        );
    }

    #[test]
    fn word_auto_spacing_uses_glyph_height_not_point_size() {
        let mut lines = vec![LayoutLine {
            items: Vec::new(),
            width: 0.0,
            ascent: 10.0,
            descent: 3.0,
            line_gap: 2.0,
            height: 12.0,
            indent_left: 0.0,
            available_width: 468.0,
            is_last: true,
        }];
        let properties = CT_PPr {
            line_spacing: Some(Twips(480)),
            line_rule: Some("auto".to_string()),
            ..Default::default()
        };
        restore_word_line_heights(&mut lines, &properties);
        assert_eq!(lines[0].height, 26.0);
        assert_eq!(lines[0].line_gap, 0.0);
    }
}
