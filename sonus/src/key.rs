//! 调式 — 基于 `ScaleType` 的调号、音阶生成与转调。
//!
//! `Key` 取代旧 `KeyMode` 枚举，统一使用 30 种 `ScaleType` 定义调式。

use super::pitch::{Accidental, NoteName, Pitch};
use super::scale::{ScaleType, generate_scale_pitches};

/// 调式：根音 + 音阶类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub root: Pitch,
    pub scale_type: ScaleType,
}

impl Key {
    pub fn new(root: Pitch, scale_type: ScaleType) -> Self {
        Self { root, scale_type }
    }

    // ── 常用快捷构造 ──

    pub fn major(root: Pitch) -> Self {
        Self::new(root, ScaleType::Major)
    }

    pub fn minor(root: Pitch) -> Self {
        Self::new(root, ScaleType::Minor)
    }

    pub fn harmonic_minor(root: Pitch) -> Self {
        Self::new(root, ScaleType::HarmonicMinor)
    }

    pub fn melodic_minor(root: Pitch) -> Self {
        Self::new(root, ScaleType::MelodicMinor)
    }

    pub fn dorian(root: Pitch) -> Self {
        Self::new(root, ScaleType::Dorian)
    }

    pub fn mixolydian(root: Pitch) -> Self {
        Self::new(root, ScaleType::Mixolydian)
    }

    // ── 全局 / 局部（语义别名，结构相同）──

    pub fn global(root: Pitch, scale_type: ScaleType) -> Self {
        Self::new(root, scale_type)
    }

    pub fn local(root: Pitch, scale_type: ScaleType) -> Self {
        Self::new(root, scale_type)
    }

    // ── 查询 ──

    pub fn root_name(&self) -> String {
        self.root.display()
    }

    pub fn root_semitone(&self) -> i8 {
        self.root.semitone()
    }

    pub fn is_major(&self) -> bool {
        self.scale_type == ScaleType::Major
    }

    pub fn is_minor(&self) -> bool {
        matches!(
            self.scale_type,
            ScaleType::Minor | ScaleType::HarmonicMinor | ScaleType::MelodicMinor
        )
    }

    // ── 音阶生成 ──

    /// 生成该调的音阶音高列表（无八度）。
    pub fn scale(&self) -> Vec<Pitch> {
        generate_scale_pitches(self.root.name, self.scale_type)
    }

    /// 返回该调号需要的变音记号列表。
    pub fn accidentals(&self) -> Vec<Pitch> {
        self.scale()
            .into_iter()
            .filter(|p| p.acc != Accidental::Natural)
            .collect()
    }

    /// 调号中的升降号数量（正数=升号，负数=降号）。
    pub fn accidental_count(&self) -> i8 {
        let mut count: i8 = 0;
        for pc in &self.scale() {
            match pc.acc {
                Accidental::Sharp | Accidental::DoubleSharp => count += 1,
                Accidental::Flat | Accidental::DoubleFlat => count -= 1,
                Accidental::Natural => {}
            }
        }
        count
    }

    /// 判断给定音级在该调中是否需要变音记号。
    pub fn accidental_for_degree(&self, degree: u8) -> Option<Accidental> {
        if degree == 0 {
            return None;
        }
        let scale = self.scale();
        let idx = (degree as usize).saturating_sub(1);
        if idx < scale.len() {
            Some(scale[idx].acc)
        } else {
            None
        }
    }

    /// 判断给定音名在该调号中是否需要变音记号。
    pub fn accidental_for_note_name(&self, note_name: NoteName) -> Accidental {
        for pc in &self.scale() {
            if pc.name == note_name {
                return pc.acc;
            }
        }
        Accidental::Natural
    }

    // ── 转调 ──

    /// 转调：返回新 Key（不修改自身）。
    pub fn transpose(&self, semitones: i8) -> Self {
        Self {
            root: self.root.transpose(semitones),
            scale_type: self.scale_type,
        }
    }

    pub fn display(&self) -> String {
        format!("key({} {})", self.root.display(), self.scale_type.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: NoteName, acc: Accidental) -> Pitch {
        Pitch::new(name, acc, None)
    }

    #[test]
    fn test_c_major_scale() {
        let key = Key::major(p(NoteName::C, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[0], p(NoteName::C, Accidental::Natural));
        assert_eq!(scale[2], p(NoteName::E, Accidental::Natural));
        assert_eq!(scale[6], p(NoteName::B, Accidental::Natural));
    }

    #[test]
    fn test_g_major_scale() {
        let key = Key::major(p(NoteName::G, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[0], p(NoteName::G, Accidental::Natural));
        assert_eq!(scale[6], p(NoteName::F, Accidental::Sharp));
    }

    #[test]
    fn test_d_major_scale() {
        let key = Key::major(p(NoteName::D, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[0], p(NoteName::D, Accidental::Natural));
        assert_eq!(scale[2], p(NoteName::F, Accidental::Sharp));
        assert_eq!(scale[6], p(NoteName::C, Accidental::Sharp));
    }

    #[test]
    fn test_f_major_scale() {
        let key = Key::major(p(NoteName::F, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[0], p(NoteName::F, Accidental::Natural));
        assert_eq!(scale[3], p(NoteName::B, Accidental::Flat));
    }

    #[test]
    fn test_a_minor_scale() {
        let key = Key::minor(p(NoteName::A, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[0], p(NoteName::A, Accidental::Natural));
        assert_eq!(scale[2], p(NoteName::C, Accidental::Natural));
        assert_eq!(scale[5], p(NoteName::F, Accidental::Natural));
    }

    #[test]
    fn test_a_harmonic_minor() {
        let key = Key::harmonic_minor(p(NoteName::A, Accidental::Natural));
        let scale = key.scale();
        assert_eq!(scale[6], p(NoteName::G, Accidental::Sharp));
    }

    #[test]
    fn test_accidentals_g_major() {
        let key = Key::major(p(NoteName::G, Accidental::Natural));
        let accs = key.accidentals();
        assert_eq!(accs.len(), 1);
        assert_eq!(accs[0], p(NoteName::F, Accidental::Sharp));
    }

    #[test]
    fn test_accidentals_c_major() {
        let key = Key::major(p(NoteName::C, Accidental::Natural));
        assert!(key.accidentals().is_empty());
    }

    #[test]
    fn test_accidental_count() {
        assert_eq!(
            Key::major(p(NoteName::C, Accidental::Natural)).accidental_count(),
            0
        );
        assert_eq!(
            Key::major(p(NoteName::G, Accidental::Natural)).accidental_count(),
            1
        );
        assert_eq!(
            Key::major(p(NoteName::F, Accidental::Natural)).accidental_count(),
            -1
        );
    }

    #[test]
    fn test_accidental_for_degree() {
        let key = Key::major(p(NoteName::G, Accidental::Natural));
        assert_eq!(key.accidental_for_degree(7), Some(Accidental::Sharp));
    }

    #[test]
    fn test_accidental_for_note_name() {
        let key = Key::major(p(NoteName::G, Accidental::Natural));
        assert_eq!(key.accidental_for_note_name(NoteName::F), Accidental::Sharp);
        assert_eq!(key.accidental_for_note_name(NoteName::C), Accidental::Natural);
    }

    #[test]
    fn test_transpose() {
        let key = Key::major(p(NoteName::C, Accidental::Natural));
        let transposed = key.transpose(2);
        assert_eq!(transposed.root, p(NoteName::D, Accidental::Natural));
        assert_eq!(transposed.scale_type, ScaleType::Major);
    }

    #[test]
    fn test_is_major_minor() {
        assert!(Key::major(p(NoteName::C, Accidental::Natural)).is_major());
        assert!(Key::minor(p(NoteName::A, Accidental::Natural)).is_minor());
        assert!(Key::harmonic_minor(p(NoteName::A, Accidental::Natural)).is_minor());
        assert!(!Key::major(p(NoteName::C, Accidental::Natural)).is_minor());
    }

    #[test]
    fn test_display() {
        let key = Key::major(p(NoteName::C, Accidental::Natural));
        assert_eq!(key.display(), "key(C major)");
    }

    #[test]
    fn test_key_clone_copy() {
        let k = Key::dorian(p(NoteName::D, Accidental::Natural));
        let k2 = k;
        assert_eq!(k.root_name(), k2.root_name());
        assert_eq!(k.scale_type, k2.scale_type);
    }

    #[test]
    fn test_key_eq() {
        let a = Key::major(p(NoteName::C, Accidental::Natural));
        let b = Key::major(p(NoteName::C, Accidental::Natural));
        let c = Key::major(p(NoteName::G, Accidental::Natural));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_key_transposition_boundary() {
        let k = Key::major(p(NoteName::B, Accidental::Natural));
        let transposed = k.transpose(1);
        assert_eq!(transposed.root_name(), "C");
        assert!(transposed.is_major());
    }

    #[test]
    fn test_scale_type_count() {
        use super::super::scale::ScaleType;
        let count = ScaleType::all().len();
        assert!(count > 0);
        assert!(count <= 50);
    }

    #[test]
    fn test_key_scale_coverage() {
        // All 7 diatonic notes are covered by scale()
        let k = Key::major(p(NoteName::C, Accidental::Natural));
        let notes = k.scale();
        assert_eq!(notes.len(), 7);
    }
}
