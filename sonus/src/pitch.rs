//! 音高领域模型 — 纯乐理，零 MIDI 耦合。
//!
//! 核心类型层级：`NoteName` → `Accidental` → `PitchClass` → `Pitch`。

// ── 变音记号 ──────────────────────────────────────────────

/// 变音记号，对应词法 `# | b | bb | x | =`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Accidental {
    Natural,
    Sharp,
    DoubleSharp,
    Flat,
    DoubleFlat,
}

impl Accidental {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "=" | "" => Some(Self::Natural),
            "#" => Some(Self::Sharp),
            "x" => Some(Self::DoubleSharp),
            "b" => Some(Self::Flat),
            "bb" => Some(Self::DoubleFlat),
            _ => None,
        }
    }

    /// 相对自然音的半音偏移。
    pub fn semitone_offset(&self) -> i8 {
        match self {
            Self::Natural => 0,
            Self::Sharp => 1,
            Self::DoubleSharp => 2,
            Self::Flat => -1,
            Self::DoubleFlat => -2,
        }
    }

    /// 从半音偏移推断变音记号。
    pub fn from_offset(offset: i8) -> Self {
        match offset {
            2 | -10 => Self::DoubleSharp,
            1 | -11 => Self::Sharp,
            -1 | 11 => Self::Flat,
            -2 | 10 => Self::DoubleFlat,
            _ => Self::Natural,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Natural => "=",
            Self::Sharp => "#",
            Self::DoubleSharp => "x",
            Self::Flat => "b",
            Self::DoubleFlat => "bb",
        }
    }

    /// 用于 display 的紧凑形式（Natural 输出空串）。
    pub fn as_display_str(&self) -> &'static str {
        match self {
            Self::Natural => "",
            Self::Sharp => "#",
            Self::DoubleSharp => "x",
            Self::Flat => "b",
            Self::DoubleFlat => "bb",
        }
    }
}

// ── 音名 ──────────────────────────────────────────────────

/// 七个基本音名 C D E F G A B。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteName {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl NoteName {
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'C' => Some(Self::C),
            'D' => Some(Self::D),
            'E' => Some(Self::E),
            'F' => Some(Self::F),
            'G' => Some(Self::G),
            'A' => Some(Self::A),
            'B' => Some(Self::B),
            _ => None,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            Self::C => 'C',
            Self::D => 'D',
            Self::E => 'E',
            Self::F => 'F',
            Self::G => 'G',
            Self::A => 'A',
            Self::B => 'B',
        }
    }

    /// 自然音半音值：C=0, D=2, E=4, F=5, G=7, A=9, B=11。
    pub fn base_semitone(&self) -> i8 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }

    /// 在 C-B 序列中的索引（0-6）。
    pub fn index(&self) -> u8 {
        match self {
            Self::C => 0,
            Self::D => 1,
            Self::E => 2,
            Self::F => 3,
            Self::G => 4,
            Self::A => 5,
            Self::B => 6,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx % 7 {
            0 => Self::C,
            1 => Self::D,
            2 => Self::E,
            3 => Self::F,
            4 => Self::G,
            5 => Self::A,
            _ => Self::B,
        }
    }

    /// 从当前音名开始，步进 n 步后的音名。
    pub fn step(&self, n: i8) -> Self {
        let new_idx = ((self.index() as i8 + n) % 7 + 7) % 7;
        Self::from_index(new_idx as u8)
    }
}

// ── 音级类 ────────────────────────────────────────────────

/// 音级类（Pitch Class），整数 0-11，对应十二平均律中的 12 个音。
///
/// 0=C, 1=C#/Db, 2=D, ..., 11=B。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PitchClass(pub u8);

impl PitchClass {
    pub fn new(pc: u8) -> Self {
        Self(pc % 12)
    }

    pub fn get(&self) -> u8 {
        self.0
    }

    pub fn transpose(&self, semitones: i8) -> Self {
        let s = (self.0 as i8 + semitones % 12 + 12) % 12;
        Self(s as u8)
    }

    /// 与另一个音级类的半音距离（正向）。
    pub fn interval_to(&self, other: Self) -> u8 {
        (((other.0 as i8 - self.0 as i8) % 12 + 12) % 12) as u8
    }

    /// 升号拼写表：返回最常用的 (NoteName, Accidental)。
    pub fn spell_sharp(&self) -> (NoteName, Accidental) {
        SHARP_SPELLING[self.0 as usize]
    }

    /// 降号拼写表。
    pub fn spell_flat(&self) -> (NoteName, Accidental) {
        FLAT_SPELLING[self.0 as usize]
    }
}

/// 升号偏好拼写表。
const SHARP_SPELLING: [(NoteName, Accidental); 12] = [
    (NoteName::C, Accidental::Natural),
    (NoteName::C, Accidental::Sharp),
    (NoteName::D, Accidental::Natural),
    (NoteName::D, Accidental::Sharp),
    (NoteName::E, Accidental::Natural),
    (NoteName::F, Accidental::Natural),
    (NoteName::F, Accidental::Sharp),
    (NoteName::G, Accidental::Natural),
    (NoteName::G, Accidental::Sharp),
    (NoteName::A, Accidental::Natural),
    (NoteName::A, Accidental::Sharp),
    (NoteName::B, Accidental::Natural),
];

/// 降号偏好拼写表。
const FLAT_SPELLING: [(NoteName, Accidental); 12] = [
    (NoteName::C, Accidental::Natural),
    (NoteName::D, Accidental::Flat),
    (NoteName::D, Accidental::Natural),
    (NoteName::E, Accidental::Flat),
    (NoteName::E, Accidental::Natural),
    (NoteName::F, Accidental::Natural),
    (NoteName::G, Accidental::Flat),
    (NoteName::G, Accidental::Natural),
    (NoteName::A, Accidental::Flat),
    (NoteName::A, Accidental::Natural),
    (NoteName::B, Accidental::Flat),
    (NoteName::B, Accidental::Natural),
];

// ── 音高 ──────────────────────────────────────────────────

/// 音高：音名 + 变音记号 + 可选八度。
///
/// 八度为 `None` 时表示未确定（和弦符号 / 音阶级数专用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    pub name: NoteName,
    pub acc: Accidental,
    pub octave: Option<u8>,
}

impl Pitch {
    pub fn new(name: NoteName, acc: Accidental, octave: Option<u8>) -> Self {
        Self { name, acc, octave }
    }

    /// 从字符快速构造（兼容旧 API）。
    pub fn from_char(c: char, acc: Accidental, octave: Option<u8>) -> Self {
        Self::new(NoteName::from_char(c).unwrap_or(NoteName::C), acc, octave)
    }

    /// 音级类半音值（0-11），不含八度。
    pub fn semitone(&self) -> i8 {
        self.name.base_semitone() + self.acc.semitone_offset()
    }

    /// 返回对应的 `PitchClass`。
    pub fn pitch_class(&self) -> PitchClass {
        PitchClass::new((((self.semitone() % 12) + 12) % 12) as u8)
    }

    /// 转调：返回新音高（不修改自身）。
    ///
    /// 正向转调偏好升号拼写，负向偏好降号拼写。
    pub fn transpose(&self, semitones: i8) -> Self {
        let new_pc = ((self.semitone() + semitones) % 12 + 12) % 12;
        let (name, acc) = if semitones < 0 {
            FLAT_SPELLING[new_pc as usize]
        } else {
            SHARP_SPELLING[new_pc as usize]
        };

        let octave = self.octave.map(|oct| {
            let total = self.semitone() as i16 + (oct as i16 + 1) * 12 + semitones as i16;
            ((total.div_euclid(12)) - 1).max(0) as u8
        });

        Self { name, acc, octave }
    }

    /// 从音级类构造音高（无八度），使用升号拼写。
    pub fn from_pitch_class_sharp(pc: PitchClass) -> Self {
        let (name, acc) = pc.spell_sharp();
        Self::new(name, acc, None)
    }

    /// 从音级类构造音高（无八度），使用降号拼写。
    pub fn from_pitch_class_flat(pc: PitchClass) -> Self {
        let (name, acc) = pc.spell_flat();
        Self::new(name, acc, None)
    }

    pub fn display(&self) -> String {
        let mut s = self.name.as_char().to_string();
        s.push_str(self.acc.as_display_str());
        if let Some(oct) = self.octave {
            s.push_str(&oct.to_string());
        }
        s
    }
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accidental_offset() {
        assert_eq!(Accidental::Sharp.semitone_offset(), 1);
        assert_eq!(Accidental::Flat.semitone_offset(), -1);
        assert_eq!(Accidental::DoubleSharp.semitone_offset(), 2);
        assert_eq!(Accidental::DoubleFlat.semitone_offset(), -2);
    }

    #[test]
    fn test_accidental_from_str() {
        assert_eq!(Accidental::from_str("#"), Some(Accidental::Sharp));
        assert_eq!(Accidental::from_str("b"), Some(Accidental::Flat));
        assert_eq!(Accidental::from_str("bb"), Some(Accidental::DoubleFlat));
        assert_eq!(Accidental::from_str("x"), Some(Accidental::DoubleSharp));
        assert_eq!(Accidental::from_str("="), Some(Accidental::Natural));
        assert_eq!(Accidental::from_str("z"), None);
    }

    #[test]
    fn test_note_name_base_semitone() {
        assert_eq!(NoteName::C.base_semitone(), 0);
        assert_eq!(NoteName::E.base_semitone(), 4);
        assert_eq!(NoteName::B.base_semitone(), 11);
    }

    #[test]
    fn test_note_name_step() {
        assert_eq!(NoteName::C.step(2), NoteName::E);
        assert_eq!(NoteName::G.step(3), NoteName::C);
        assert_eq!(NoteName::B.step(1), NoteName::C);
    }

    #[test]
    fn test_pitch_class_transpose() {
        assert_eq!(PitchClass::new(0).transpose(4), PitchClass::new(4));
        assert_eq!(PitchClass::new(11).transpose(2), PitchClass::new(1));
        assert_eq!(PitchClass::new(0).transpose(-1), PitchClass::new(11));
    }

    #[test]
    fn test_pitch_semitone() {
        let c = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        assert_eq!(c.semitone(), 0);

        let cs = Pitch::new(NoteName::C, Accidental::Sharp, Some(4));
        assert_eq!(cs.semitone(), 1);

        let df = Pitch::new(NoteName::D, Accidental::Flat, Some(4));
        assert_eq!(df.semitone(), 1);
    }

    #[test]
    fn test_pitch_class() {
        let c = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        assert_eq!(c.pitch_class(), PitchClass::new(0));

        let gs = Pitch::new(NoteName::G, Accidental::Sharp, None);
        assert_eq!(gs.pitch_class(), PitchClass::new(8));
    }

    #[test]
    fn test_transpose_positive() {
        let c = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        let d = c.transpose(2);
        assert_eq!(d.name, NoteName::D);
        assert_eq!(d.acc, Accidental::Natural);
        assert_eq!(d.octave, Some(4));
    }

    #[test]
    fn test_transpose_negative() {
        let f = Pitch::new(NoteName::F, Accidental::Natural, Some(4));
        let e = f.transpose(-1);
        assert_eq!(e.name, NoteName::E);
        assert_eq!(e.acc, Accidental::Natural);
    }

    #[test]
    fn test_transpose_cross_octave() {
        let b = Pitch::new(NoteName::B, Accidental::Natural, Some(4));
        let c = b.transpose(1);
        assert_eq!(c.name, NoteName::C);
        assert_eq!(c.octave, Some(5));
    }

    #[test]
    fn test_transpose_no_octave() {
        let c = Pitch::new(NoteName::C, Accidental::Natural, None);
        let cs = c.transpose(1);
        assert_eq!(cs.name, NoteName::C);
        assert_eq!(cs.acc, Accidental::Sharp);
        assert_eq!(cs.octave, None);
    }

    #[test]
    fn test_display() {
        assert_eq!(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)).display(),
            "C4"
        );
        assert_eq!(
            Pitch::new(NoteName::F, Accidental::Sharp, Some(5)).display(),
            "F#5"
        );
        assert_eq!(
            Pitch::new(NoteName::B, Accidental::Flat, Some(3)).display(),
            "Bb3"
        );
        assert_eq!(
            Pitch::new(NoteName::E, Accidental::Natural, None).display(),
            "E"
        );
    }
}
