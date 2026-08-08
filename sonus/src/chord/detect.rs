//! 和弦识别 — 从音级类列表识别和弦（多候选）。
//!
//! 使用数据驱动的模式表，支持三和弦至十三和弦的常见类型。
//! 当输入匹配多个模式时，返回所有候选（按匹配度排序）。

use super::quality::ChordQuality;

/// 和弦模式定义。
struct ChordPattern {
    #[allow(dead_code)]
    name: &'static str,
    intervals: &'static [i8],
    quality: ChordQuality,
    extension: Option<u32>,
    major_seventh: bool,
}

/// 常见和弦模式表（按精确度从高到低排列）。
const PATTERNS: &[ChordPattern] = &[
    // ── 七和弦 ──
    ChordPattern { name: "maj7", intervals: &[0, 4, 7, 11], quality: ChordQuality::Maj, extension: Some(7), major_seventh: true },
    ChordPattern { name: "7", intervals: &[0, 4, 7, 10], quality: ChordQuality::Maj, extension: Some(7), major_seventh: false },
    ChordPattern { name: "m7", intervals: &[0, 3, 7, 10], quality: ChordQuality::Min, extension: Some(7), major_seventh: false },
    ChordPattern { name: "m(maj7)", intervals: &[0, 3, 7, 11], quality: ChordQuality::Min, extension: Some(7), major_seventh: true },
    ChordPattern { name: "dim7", intervals: &[0, 3, 6, 9], quality: ChordQuality::Dim, extension: Some(7), major_seventh: false },
    ChordPattern { name: "m7b5", intervals: &[0, 3, 6, 10], quality: ChordQuality::Dim, extension: Some(7), major_seventh: false },
    ChordPattern { name: "aug7", intervals: &[0, 4, 8, 10], quality: ChordQuality::Aug, extension: Some(7), major_seventh: false },
    ChordPattern { name: "7sus4", intervals: &[0, 5, 7, 10], quality: ChordQuality::Sus4, extension: Some(7), major_seventh: false },

    // ── 六和弦 ──
    ChordPattern { name: "6", intervals: &[0, 4, 7, 9], quality: ChordQuality::Maj, extension: Some(6), major_seventh: false },
    ChordPattern { name: "m6", intervals: &[0, 3, 7, 9], quality: ChordQuality::Min, extension: Some(6), major_seventh: false },

    // ── 九和弦 ──
    ChordPattern { name: "9", intervals: &[0, 4, 7, 10, 14], quality: ChordQuality::Maj, extension: Some(9), major_seventh: false },
    ChordPattern { name: "maj9", intervals: &[0, 4, 7, 11, 14], quality: ChordQuality::Maj, extension: Some(9), major_seventh: true },
    ChordPattern { name: "m9", intervals: &[0, 3, 7, 10, 14], quality: ChordQuality::Min, extension: Some(9), major_seventh: false },

    // ── 十一和弦 ──
    ChordPattern { name: "11", intervals: &[0, 4, 7, 10, 14, 17], quality: ChordQuality::Maj, extension: Some(11), major_seventh: false },

    // ── 十三和弦 ──
    ChordPattern { name: "13", intervals: &[0, 4, 7, 10, 14, 21], quality: ChordQuality::Maj, extension: Some(13), major_seventh: false },

    // ── 三和弦 ──
    ChordPattern { name: "", intervals: &[0, 4, 7], quality: ChordQuality::Maj, extension: None, major_seventh: false },
    ChordPattern { name: "m", intervals: &[0, 3, 7], quality: ChordQuality::Min, extension: None, major_seventh: false },
    ChordPattern { name: "dim", intervals: &[0, 3, 6], quality: ChordQuality::Dim, extension: None, major_seventh: false },
    ChordPattern { name: "aug", intervals: &[0, 4, 8], quality: ChordQuality::Aug, extension: None, major_seventh: false },
    ChordPattern { name: "sus2", intervals: &[0, 2, 7], quality: ChordQuality::Sus2, extension: None, major_seventh: false },
    ChordPattern { name: "sus4", intervals: &[0, 5, 7], quality: ChordQuality::Sus4, extension: None, major_seventh: false },
    ChordPattern { name: "5", intervals: &[0, 7], quality: ChordQuality::Power, extension: None, major_seventh: false },
];

/// 识别结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifiedChord {
    pub root_name: String,
    pub quality: ChordQuality,
    pub extension: Option<u32>,
    pub major_seventh: bool,
    /// 匹配类型：exact = 完全匹配，subset = 输入包含模式所有音
    pub match_type: MatchType,
}

/// 匹配类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// 输入音集与模式音集完全相同。
    Exact,
    /// 模式音集是输入音集的子集。
    Subset,
}

/// 从音级类列表识别和弦，返回所有候选（按匹配度排序）。
///
/// 输入为 0-11 的音级类列表。输出按 Exact > Subset、
/// 音数多 > 音数少 排序。
pub fn identify_chord(pitch_classes: &[u8]) -> Vec<IdentifiedChord> {
    if pitch_classes.is_empty() {
        return Vec::new();
    }

    // 去重排序
    let mut pcs: Vec<u8> = pitch_classes.iter().map(|&n| n % 12).collect();
    pcs.sort();
    pcs.dedup();

    if pcs.is_empty() {
        return Vec::new();
    }

    let note_names = ['C', 'C', 'D', 'D', 'E', 'F', 'F', 'G', 'G', 'A', 'A', 'B'];
    let mut candidates: Vec<IdentifiedChord> = Vec::new();

    for &root_pc in &pcs {
        // 计算相对 root 的音程
        let intervals: Vec<i8> = pcs
            .iter()
            .map(|&pc| ((pc as i8 - root_pc as i8) % 12 + 12) % 12)
            .collect();

        for pattern in PATTERNS {
            let pattern_pcs: Vec<i8> = pattern
                .intervals
                .iter()
                .map(|&i| i % 12)
                .collect();

            let is_exact = intervals.len() == pattern_pcs.len()
                && intervals.iter().all(|i| pattern_pcs.contains(i))
                && pattern_pcs.iter().all(|i| intervals.contains(i));

            let is_subset = !is_exact
                && pattern_pcs.iter().all(|i| intervals.contains(i));

            if is_exact || is_subset {
                candidates.push(IdentifiedChord {
                    root_name: note_names[root_pc as usize].to_string(),
                    quality: pattern.quality,
                    extension: pattern.extension,
                    major_seventh: pattern.major_seventh,
                    match_type: if is_exact { MatchType::Exact } else { MatchType::Subset },
                });
            }
        }
    }

    // 排序：Exact 优先，然后按模式音数降序
    candidates.sort_by(|a, b| {
        let a_priority = pattern_note_count(a) + if a.match_type == MatchType::Exact { 100 } else { 0 };
        let b_priority = pattern_note_count(b) + if b.match_type == MatchType::Exact { 100 } else { 0 };
        b_priority.cmp(&a_priority)
    });

    candidates
}

/// 返回最佳匹配（第一个候选）。
pub fn identify_chord_best(pitch_classes: &[u8]) -> Option<IdentifiedChord> {
    identify_chord(pitch_classes).into_iter().next()
}

fn pattern_note_count(chord: &IdentifiedChord) -> usize {
    let triad_count = chord.quality.intervals().len();
    let ext_count = match chord.extension {
        Some(6) => 1,
        Some(7) => 1,
        Some(9) => 2,
        Some(11) => 3,
        Some(13) => 4,
        _ => 0,
    };
    triad_count + ext_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_c_major() {
        let result = identify_chord(&[0, 4, 7]);
        assert!(result.iter().any(|c| c.root_name == "C" && c.quality == ChordQuality::Maj));
    }

    #[test]
    fn test_identify_a_minor() {
        let result = identify_chord(&[9, 0, 4]);
        assert!(result.iter().any(|c| c.root_name == "A" && c.quality == ChordQuality::Min));
    }

    #[test]
    fn test_identify_g7() {
        let result = identify_chord(&[7, 11, 2, 5]);
        assert!(result.iter().any(|c| c.root_name == "G" && c.extension == Some(7)));
    }

    #[test]
    fn test_identify_cmaj7() {
        let _result = identify_chord(&[0, 4, 7, 11]);
        let best = identify_chord_best(&[0, 4, 7, 11]);
        assert!(best.is_some());
        let best = best.unwrap();
        assert_eq!(best.root_name, "C");
        assert_eq!(best.quality, ChordQuality::Maj);
        assert_eq!(best.extension, Some(7));
        assert!(best.major_seventh);
        assert_eq!(best.match_type, MatchType::Exact);
    }

    #[test]
    fn test_identify_dim7() {
        let result = identify_chord_best(&[0, 3, 6, 9]);
        assert!(result.is_some());
        let chord = result.unwrap();
        assert_eq!(chord.quality, ChordQuality::Dim);
        assert_eq!(chord.extension, Some(7));
    }

    #[test]
    fn test_identify_half_diminished() {
        let result = identify_chord_best(&[0, 3, 6, 10]);
        assert!(result.is_some());
        let chord = result.unwrap();
        assert_eq!(chord.quality, ChordQuality::Dim);
        assert_eq!(chord.extension, Some(7));
    }

    #[test]
    fn test_identify_power_chord() {
        let result = identify_chord(&[0, 7]);
        assert!(result.iter().any(|c| c.quality == ChordQuality::Power));
    }

    #[test]
    fn test_identify_empty() {
        assert!(identify_chord(&[]).is_empty());
    }

    #[test]
    fn test_multi_candidate() {
        // C major triad {0, 4, 7} can be identified as C major
        // but also as subset of Cmaj7, C6, C9, etc.
        let candidates = identify_chord(&[0, 4, 7]);
        // At minimum, should find C major (exact) and several subset matches
        assert!(candidates.len() >= 1);
        let has_exact = candidates.iter().any(|c| c.match_type == MatchType::Exact);
        assert!(has_exact);
    }
}
