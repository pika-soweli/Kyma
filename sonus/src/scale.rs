//! 音阶类型字典 — 数据驱动，30 种音阶。
//!
//! 每种 `ScaleType` 通过 `intervals()` 返回半音间隔模式，
//! 由 `Scale` / `Key` 负责生成具体音高。

use super::pitch::{Accidental, NoteName, Pitch, PitchClass};

/// 30 种音阶类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleType {
    // ── 七声教会调式 ──
    Major, // = Ionian
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Minor, // = Aeolian (Natural Minor)
    Locrian,

    // ── 小调变体 ──
    HarmonicMinor,
    MelodicMinor,

    // ── 五声 ──
    MajorPentatonic,
    MinorPentatonic,

    // ── 布鲁斯 ──
    Blues,

    // ── 对称 ──
    WholeTone,
    Chromatic,
    Octatonic,

    // ── 民族 / 异域 ──
    HungarianMinor,
    PhrygianDominant,
    NeapolitanMajor,
    NeapolitanMinor,
    Enigmatic,
    Oriental,
    HungarianGypsy,
    Romanian,
    Persian,
    Arabic,
    Byzantine,
    Egyptian,
    Hindu,

    // ── 日本 ──
    Hirajoshi,
    Insen,
}

impl ScaleType {
    /// 返回音阶的半音间隔模式（从根音开始的半音偏移量）。
    pub fn intervals(&self) -> &'static [i8] {
        match self {
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Self::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Self::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Self::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Self::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Self::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            Self::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Self::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            Self::MajorPentatonic => &[0, 2, 4, 7, 9],
            Self::MinorPentatonic => &[0, 3, 5, 7, 10],
            Self::Blues => &[0, 3, 5, 6, 7, 10],
            Self::WholeTone => &[0, 2, 4, 6, 8, 10],
            Self::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Self::Octatonic => &[0, 2, 3, 5, 6, 8, 9, 11],
            Self::HungarianMinor => &[0, 2, 3, 6, 7, 8, 11],
            Self::PhrygianDominant => &[0, 1, 4, 5, 7, 8, 10],
            Self::NeapolitanMajor => &[0, 1, 3, 5, 7, 9, 11],
            Self::NeapolitanMinor => &[0, 1, 3, 5, 7, 8, 11],
            Self::Enigmatic => &[0, 1, 4, 6, 8, 10, 11],
            Self::Oriental => &[0, 1, 4, 5, 6, 8, 9],
            Self::HungarianGypsy => &[0, 2, 3, 6, 7, 8, 10],
            Self::Romanian => &[0, 2, 3, 5, 6, 7, 10],
            Self::Persian => &[0, 1, 4, 5, 6, 8, 11],
            Self::Arabic => &[0, 1, 4, 5, 7, 8, 11],
            Self::Byzantine => &[0, 1, 4, 5, 7, 8, 11],
            Self::Egyptian => &[0, 2, 5, 7, 10],
            Self::Hindu => &[0, 2, 4, 5, 7, 8, 10],
            Self::Hirajoshi => &[0, 2, 3, 7, 8],
            Self::Insen => &[0, 1, 5, 7, 10],
        }
    }

    pub fn note_count(&self) -> usize {
        self.intervals().len()
    }

    pub fn is_heptatonic(&self) -> bool {
        self.note_count() == 7
    }

    pub fn is_pentatonic(&self) -> bool {
        self.note_count() == 5
    }

    /// 短标识符（用于词法解析 / from_str）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Dorian => "dorian",
            Self::Phrygian => "phrygian",
            Self::Lydian => "lydian",
            Self::Mixolydian => "mixolydian",
            Self::Minor => "minor",
            Self::Locrian => "locrian",
            Self::HarmonicMinor => "harmonic_minor",
            Self::MelodicMinor => "melodic_minor",
            Self::MajorPentatonic => "major_pentatonic",
            Self::MinorPentatonic => "minor_pentatonic",
            Self::Blues => "blues",
            Self::WholeTone => "whole_tone",
            Self::Chromatic => "chromatic",
            Self::Octatonic => "octatonic",
            Self::HungarianMinor => "hungarian_minor",
            Self::PhrygianDominant => "phrygian_dominant",
            Self::NeapolitanMajor => "neapolitan_major",
            Self::NeapolitanMinor => "neapolitan_minor",
            Self::Enigmatic => "enigmatic",
            Self::Oriental => "oriental",
            Self::HungarianGypsy => "hungarian_gypsy",
            Self::Romanian => "romanian",
            Self::Persian => "persian",
            Self::Arabic => "arabic",
            Self::Byzantine => "byzantine",
            Self::Egyptian => "egyptian",
            Self::Hindu => "hindu",
            Self::Hirajoshi => "hirajoshi",
            Self::Insen => "insen",
        }
    }

    /// 人类可读名称。
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Major => "Major (Ionian)",
            Self::Dorian => "Dorian",
            Self::Phrygian => "Phrygian",
            Self::Lydian => "Lydian",
            Self::Mixolydian => "Mixolydian",
            Self::Minor => "Natural Minor (Aeolian)",
            Self::Locrian => "Locrian",
            Self::HarmonicMinor => "Harmonic Minor",
            Self::MelodicMinor => "Melodic Minor",
            Self::MajorPentatonic => "Major Pentatonic",
            Self::MinorPentatonic => "Minor Pentatonic",
            Self::Blues => "Blues",
            Self::WholeTone => "Whole Tone",
            Self::Chromatic => "Chromatic",
            Self::Octatonic => "Octatonic (Diminished)",
            Self::HungarianMinor => "Hungarian Minor",
            Self::PhrygianDominant => "Phrygian Dominant",
            Self::NeapolitanMajor => "Neapolitan Major",
            Self::NeapolitanMinor => "Neapolitan Minor",
            Self::Enigmatic => "Enigmatic",
            Self::Oriental => "Oriental",
            Self::HungarianGypsy => "Hungarian Gypsy",
            Self::Romanian => "Romanian",
            Self::Persian => "Persian",
            Self::Arabic => "Arabic",
            Self::Byzantine => "Byzantine (Double Harmonic)",
            Self::Egyptian => "Egyptian",
            Self::Hindu => "Hindu",
            Self::Hirajoshi => "Hirajoshi",
            Self::Insen => "Insen",
        }
    }

    /// 从短标识符解析。
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "major" | "ionian" => Some(Self::Major),
            "minor" | "natural_minor" | "aeolian" => Some(Self::Minor),
            "dorian" => Some(Self::Dorian),
            "phrygian" => Some(Self::Phrygian),
            "lydian" => Some(Self::Lydian),
            "mixolydian" => Some(Self::Mixolydian),
            "locrian" => Some(Self::Locrian),
            "harmonic_minor" => Some(Self::HarmonicMinor),
            "melodic_minor" => Some(Self::MelodicMinor),
            "major_pentatonic" => Some(Self::MajorPentatonic),
            "minor_pentatonic" => Some(Self::MinorPentatonic),
            "blues" => Some(Self::Blues),
            "whole_tone" => Some(Self::WholeTone),
            "chromatic" => Some(Self::Chromatic),
            "octatonic" => Some(Self::Octatonic),
            "hungarian_minor" => Some(Self::HungarianMinor),
            "phrygian_dominant" => Some(Self::PhrygianDominant),
            "neapolitan_major" => Some(Self::NeapolitanMajor),
            "neapolitan_minor" => Some(Self::NeapolitanMinor),
            "enigmatic" => Some(Self::Enigmatic),
            "oriental" => Some(Self::Oriental),
            "hungarian_gypsy" => Some(Self::HungarianGypsy),
            "romanian" => Some(Self::Romanian),
            "persian" => Some(Self::Persian),
            "arabic" => Some(Self::Arabic),
            "byzantine" | "double_harmonic" => Some(Self::Byzantine),
            "egyptian" => Some(Self::Egyptian),
            "hindu" => Some(Self::Hindu),
            "hirajoshi" => Some(Self::Hirajoshi),
            "insen" => Some(Self::Insen),
            _ => None,
        }
    }

    /// 所有音阶类型。
    pub fn all() -> &'static [ScaleType] {
        &[
            Self::Major, Self::Dorian, Self::Phrygian, Self::Lydian, Self::Mixolydian,
            Self::Minor, Self::Locrian, Self::HarmonicMinor, Self::MelodicMinor,
            Self::MajorPentatonic, Self::MinorPentatonic, Self::Blues,
            Self::WholeTone, Self::Chromatic, Self::Octatonic,
            Self::HungarianMinor, Self::PhrygianDominant, Self::NeapolitanMajor,
            Self::NeapolitanMinor, Self::Enigmatic, Self::Oriental,
            Self::HungarianGypsy, Self::Romanian, Self::Persian,
            Self::Arabic, Self::Byzantine, Self::Egyptian, Self::Hindu,
            Self::Hirajoshi, Self::Insen,
        ]
    }
}

// ── 音阶 ──────────────────────────────────────────────────

/// 音阶：根音 + 音阶类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scale {
    pub root: NoteName,
    pub scale_type: ScaleType,
}

impl Scale {
    pub fn new(root: NoteName, scale_type: ScaleType) -> Self {
        Self { root, scale_type }
    }

    /// 返回音阶中所有音级类。
    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        let root_semi = self.root.base_semitone();
        self.scale_type
            .intervals()
            .iter()
            .map(|&i| PitchClass::new((((root_semi + i) % 12 + 12) % 12) as u8))
            .collect()
    }

    /// 返回音阶中所有音高（无八度）。
    ///
    /// 七声音阶使用音名序列拼写（保留正确的音名关系），
    /// 非七声音阶使用升号拼写表。
    pub fn degrees(&self) -> Vec<Pitch> {
        generate_scale_pitches(self.root, self.scale_type)
    }

    /// 判断给定音高是否属于此音阶（按音级类比较）。
    pub fn contains(&self, pitch: &Pitch) -> bool {
        let pc = pitch.pitch_class().get();
        self.pitch_classes().iter().any(|p| p.get() == pc)
    }

    pub fn display(&self) -> String {
        format!("{} {}", self.root.as_char(), self.scale_type.display_name())
    }
}

// ── 内部辅助 ──────────────────────────────────────────────

/// 生成音阶音高列表。
///
/// 七声音阶（7 个音）使用音名序列法，确保每个音级有正确的音名。
/// 其他音阶使用升号拼写表。
pub(crate) fn generate_scale_pitches(root: NoteName, scale_type: ScaleType) -> Vec<Pitch> {
    let intervals = scale_type.intervals();
    let root_semi = root.base_semitone();

    if intervals.len() == 7 {
        // 七声音阶：音名序列法
        let letter_seq = [
            root,
            root.step(1),
            root.step(2),
            root.step(3),
            root.step(4),
            root.step(5),
            root.step(6),
        ];
        intervals
            .iter()
            .enumerate()
            .map(|(i, &interval)| {
                let target_semi = ((root_semi + interval) % 12 + 12) % 12;
                let name = letter_seq[i];
                let base = name.base_semitone();
                let diff = ((target_semi - base) % 12 + 12) % 12;
                let acc = Accidental::from_offset(diff as i8);
                Pitch::new(name, acc, None)
            })
            .collect()
    } else {
        // 非七声音阶：升号拼写表
        intervals
            .iter()
            .map(|&interval| {
                let target_pc = ((root_semi + interval) % 12 + 12) % 12;
                Pitch::from_pitch_class_sharp(PitchClass::new(target_pc as u8))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_type_count() {
        assert_eq!(ScaleType::all().len(), 30);
    }

    #[test]
    fn test_intervals() {
        assert_eq!(ScaleType::Major.intervals(), &[0, 2, 4, 5, 7, 9, 11]);
        assert_eq!(ScaleType::Minor.intervals(), &[0, 2, 3, 5, 7, 8, 10]);
        assert_eq!(ScaleType::MajorPentatonic.intervals(), &[0, 2, 4, 7, 9]);
        assert_eq!(ScaleType::Chromatic.note_count(), 12);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(ScaleType::from_str("major"), Some(ScaleType::Major));
        assert_eq!(ScaleType::from_str("minor"), Some(ScaleType::Minor));
        assert_eq!(ScaleType::from_str("aeolian"), Some(ScaleType::Minor));
        assert_eq!(ScaleType::from_str("blues"), Some(ScaleType::Blues));
        assert_eq!(ScaleType::from_str("nonexistent"), None);
    }

    #[test]
    fn test_c_major_scale() {
        let scale = Scale::new(NoteName::C, ScaleType::Major);
        let degrees = scale.degrees();
        assert_eq!(degrees[0], Pitch::new(NoteName::C, Accidental::Natural, None));
        assert_eq!(degrees[2], Pitch::new(NoteName::E, Accidental::Natural, None));
        assert_eq!(degrees[6], Pitch::new(NoteName::B, Accidental::Natural, None));
    }

    #[test]
    fn test_g_major_scale() {
        let scale = Scale::new(NoteName::G, ScaleType::Major);
        let degrees = scale.degrees();
        assert_eq!(degrees[0], Pitch::new(NoteName::G, Accidental::Natural, None));
        assert_eq!(degrees[6], Pitch::new(NoteName::F, Accidental::Sharp, None));
    }

    #[test]
    fn test_d_major_scale() {
        let scale = Scale::new(NoteName::D, ScaleType::Major);
        let degrees = scale.degrees();
        assert_eq!(degrees[0], Pitch::new(NoteName::D, Accidental::Natural, None));
        assert_eq!(degrees[2], Pitch::new(NoteName::F, Accidental::Sharp, None));
        assert_eq!(degrees[6], Pitch::new(NoteName::C, Accidental::Sharp, None));
    }

    #[test]
    fn test_f_major_scale() {
        let scale = Scale::new(NoteName::F, ScaleType::Major);
        let degrees = scale.degrees();
        assert_eq!(degrees[0], Pitch::new(NoteName::F, Accidental::Natural, None));
        assert_eq!(degrees[3], Pitch::new(NoteName::B, Accidental::Flat, None));
    }

    #[test]
    fn test_a_minor_scale() {
        let scale = Scale::new(NoteName::A, ScaleType::Minor);
        let degrees = scale.degrees();
        assert_eq!(degrees[0], Pitch::new(NoteName::A, Accidental::Natural, None));
        assert_eq!(degrees[2], Pitch::new(NoteName::C, Accidental::Natural, None));
        assert_eq!(degrees[5], Pitch::new(NoteName::F, Accidental::Natural, None));
    }

    #[test]
    fn test_a_harmonic_minor() {
        let scale = Scale::new(NoteName::A, ScaleType::HarmonicMinor);
        let degrees = scale.degrees();
        assert_eq!(degrees[6], Pitch::new(NoteName::G, Accidental::Sharp, None));
    }

    #[test]
    fn test_pentatonic_scale() {
        let scale = Scale::new(NoteName::C, ScaleType::MajorPentatonic);
        let pcs = scale.pitch_classes();
        assert_eq!(pcs.len(), 5);
        assert_eq!(pcs[0], PitchClass::new(0));
        assert_eq!(pcs[4], PitchClass::new(9));
    }

    #[test]
    fn test_contains() {
        let scale = Scale::new(NoteName::C, ScaleType::Major);
        let c = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        let f_sharp = Pitch::new(NoteName::F, Accidental::Sharp, Some(4));
        assert!(scale.contains(&c));
        assert!(!scale.contains(&f_sharp));
    }

    #[test]
    fn test_is_heptatonic() {
        assert!(ScaleType::Major.is_heptatonic());
        assert!(!ScaleType::MajorPentatonic.is_heptatonic());
        assert!(ScaleType::MajorPentatonic.is_pentatonic());
    }
}
