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

    /// 计算 MIDI 音符值（0-127）。
    ///
    /// MIDI 标准：C4 = 60。若八度为 `None` 则返回 `None`。
    ///
    /// 公式：`midi = (octave + 1) * 12 + semitone`
    pub fn to_midi(&self) -> Option<u8> {
        let octave = self.octave?;
        let midi = (octave as i16 + 1) * 12 + self.semitone() as i16;
        if (0..=127).contains(&midi) {
            Some(midi as u8)
        } else {
            None
        }
    }

    /// 在指定调性上下文中，为无变音记号的音高应用调号变音。
    ///
    /// 若音高已有显式变音记号（非 Natural），则不修改。
    pub fn apply_key_signature(
        &self,
        key_root: NoteName,
        key_mode: Option<crate::rmt::scale::Mode>,
    ) -> Self {
        if self.acc != Accidental::Natural {
            return *self;
        }

        // 使用 Key 结构获取调号的音阶，查找当前音名对应的变音记号
        use crate::key::Key;
        use crate::scale::ScaleType;

        // 根据 key_mode 确定 ScaleType
        let scale_type = match key_mode {
            Some(crate::rmt::scale::Mode::Ionian) | None => ScaleType::Major,
            Some(crate::rmt::scale::Mode::Aeolian) => ScaleType::Minor,
            Some(crate::rmt::scale::Mode::Dorian) => ScaleType::Dorian,
            Some(crate::rmt::scale::Mode::Mixolydian) => ScaleType::Mixolydian,
            _ => ScaleType::Major, // 默认大调
        };

        let root_pitch = Pitch::new(key_root, Accidental::Natural, None);
        let key = Key::new(root_pitch, scale_type);
        let scale_pitches = key.scale();

        // 查找当前音名在音阶中的变音记号
        let key_acc = scale_pitches
            .iter()
            .find(|p| p.name == self.name)
            .map(|p| p.acc)
            .unwrap_or(Accidental::Natural);

        Self {
            name: self.name,
            acc: key_acc,
            octave: self.octave,
        }
    }
}

// ── rust-music-theory 互转 ────────────────────────────────

use crate::rmt;

impl From<NoteName> for rmt::note::NoteLetter {
    fn from(n: NoteName) -> Self {
        match n {
            NoteName::C => rmt::note::NoteLetter::C,
            NoteName::D => rmt::note::NoteLetter::D,
            NoteName::E => rmt::note::NoteLetter::E,
            NoteName::F => rmt::note::NoteLetter::F,
            NoteName::G => rmt::note::NoteLetter::G,
            NoteName::A => rmt::note::NoteLetter::A,
            NoteName::B => rmt::note::NoteLetter::B,
        }
    }
}

impl From<rmt::note::NoteLetter> for NoteName {
    fn from(n: rmt::note::NoteLetter) -> Self {
        match n {
            rmt::note::NoteLetter::C => NoteName::C,
            rmt::note::NoteLetter::D => NoteName::D,
            rmt::note::NoteLetter::E => NoteName::E,
            rmt::note::NoteLetter::F => NoteName::F,
            rmt::note::NoteLetter::G => NoteName::G,
            rmt::note::NoteLetter::A => NoteName::A,
            rmt::note::NoteLetter::B => NoteName::B,
        }
    }
}

/// 将 rmt `PitchSymbol` 拆解为 sonus 的 `(NoteName, Accidental)`。
pub fn pitch_symbol_to_name_acc(ps: rmt::note::PitchSymbol) -> (NoteName, Accidental) {
    use rmt::note::PitchSymbol::*;
    match ps {
        Bs => (NoteName::B, Accidental::Sharp),
        C => (NoteName::C, Accidental::Natural),
        Cs => (NoteName::C, Accidental::Sharp),
        Db => (NoteName::D, Accidental::Flat),
        D => (NoteName::D, Accidental::Natural),
        Ds => (NoteName::D, Accidental::Sharp),
        Eb => (NoteName::E, Accidental::Flat),
        E => (NoteName::E, Accidental::Natural),
        Es => (NoteName::E, Accidental::Sharp),
        F => (NoteName::F, Accidental::Natural),
        Fs => (NoteName::F, Accidental::Sharp),
        Gb => (NoteName::G, Accidental::Flat),
        G => (NoteName::G, Accidental::Natural),
        Gs => (NoteName::G, Accidental::Sharp),
        Ab => (NoteName::A, Accidental::Flat),
        A => (NoteName::A, Accidental::Natural),
        As => (NoteName::A, Accidental::Sharp),
        Bb => (NoteName::B, Accidental::Flat),
        B => (NoteName::B, Accidental::Natural),
        Cb => (NoteName::C, Accidental::Flat),
        Fb => (NoteName::F, Accidental::Flat),
    }
}

/// 将 sonus 的 `(NoteName, Accidental)` 映射为 rmt `PitchSymbol`。
///
/// 双升 / 双降等 rmt 无法直接表示的变音会按音级类等价到升号拼写。
pub fn name_acc_to_pitch_symbol(name: NoteName, acc: Accidental) -> rmt::note::PitchSymbol {
    use rmt::note::PitchSymbol::*;
    match (name, acc) {
        (NoteName::B, Accidental::Sharp) => Bs,
        (NoteName::C, Accidental::Natural) => C,
        (NoteName::C, Accidental::Sharp) => Cs,
        (NoteName::D, Accidental::Flat) => Db,
        (NoteName::D, Accidental::Natural) => D,
        (NoteName::D, Accidental::Sharp) => Ds,
        (NoteName::E, Accidental::Flat) => Eb,
        (NoteName::E, Accidental::Natural) => E,
        (NoteName::E, Accidental::Sharp) => Es,
        (NoteName::F, Accidental::Natural) => F,
        (NoteName::F, Accidental::Sharp) => Fs,
        (NoteName::G, Accidental::Flat) => Gb,
        (NoteName::G, Accidental::Natural) => G,
        (NoteName::G, Accidental::Sharp) => Gs,
        (NoteName::A, Accidental::Flat) => Ab,
        (NoteName::A, Accidental::Natural) => A,
        (NoteName::A, Accidental::Sharp) => As,
        (NoteName::B, Accidental::Flat) => Bb,
        (NoteName::B, Accidental::Natural) => B,
        (NoteName::C, Accidental::Flat) => Cb,
        (NoteName::F, Accidental::Flat) => Fb,
        _ => {
            let pc = PitchClass::new(
                ((((name.base_semitone() + acc.semitone_offset()) % 12) + 12) % 12) as u8,
            );
            let (sn, sa) = pc.spell_sharp();
            name_acc_to_pitch_symbol(sn, sa)
        }
    }
}

impl From<Pitch> for rmt::note::Pitch {
    fn from(p: Pitch) -> Self {
        rmt::note::Pitch::new(p.name.into(), p.acc.semitone_offset() as i8)
    }
}

impl From<rmt::note::Pitch> for Pitch {
    fn from(p: rmt::note::Pitch) -> Self {
        let name: NoteName = p.letter.into();
        let acc = Accidental::from_offset(p.accidental as i8);
        Pitch::new(name, acc, None)
    }
}

impl From<PitchClass> for rmt::note::Pitch {
    fn from(pc: PitchClass) -> Self {
        let (name, acc) = pc.spell_sharp();
        rmt::note::Pitch::new(name.into(), acc.semitone_offset() as i8)
    }
}

// ── KeySignature 集成 ─────────────────────────────────────

/// 在指定调性上下文中，为音级类选择正确的等音拼写。
///
/// 利用 rmt 的 `KeySignature::get_preferred_spelling` 实现。
/// 例如在 F 大调中，音级类 5 (G) 保持 G，但在 Bb 大调中，
/// 音级类 10 会拼写为 Bb 而非 A#。
pub fn spell_in_key(pc: PitchClass, key_root: NoteName, key_mode: Option<rmt::scale::Mode>) -> (NoteName, Accidental) {
    let tonic = rmt::note::Pitch::new(key_root.into(), 0);
    let ks = match key_mode {
        Some(mode) => rmt::note::KeySignature::new_with_mode(tonic, Some(mode)),
        None => rmt::note::KeySignature::new(tonic),
    };
    let rmt_pitch: rmt::note::Pitch = pc.into();
    let symbol = ks.get_preferred_spelling(rmt_pitch);
    pitch_symbol_to_name_acc(symbol)
}

/// 通过 rmt 的 `Pitch::from_u8_with_scale_context` 进行调性感知的音高创建。
pub fn pitch_from_pc_with_context(
    pc_value: u8,
    _key_root: NoteName,
    key_mode: Option<rmt::scale::Mode>,
    direction: rmt::scale::Direction,
) -> Pitch {
    let p = rmt::note::Pitch::from_u8_with_scale_context(pc_value, key_mode, direction);
    Pitch::from(p)
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

    // ── MIDI 计算测试 ──

    #[test]
    fn test_to_midi_c4() {
        let c4 = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        assert_eq!(c4.to_midi(), Some(60));
    }

    #[test]
    fn test_to_midi_a4() {
        let a4 = Pitch::new(NoteName::A, Accidental::Natural, Some(4));
        assert_eq!(a4.to_midi(), Some(69));
    }

    #[test]
    fn test_to_midi_sharp() {
        let f_sharp = Pitch::new(NoteName::F, Accidental::Sharp, Some(4));
        assert_eq!(f_sharp.to_midi(), Some(66));
    }

    #[test]
    fn test_to_midi_flat() {
        let b_flat = Pitch::new(NoteName::B, Accidental::Flat, Some(3));
        assert_eq!(b_flat.to_midi(), Some(58));
    }

    #[test]
    fn test_to_midi_no_octave() {
        let c = Pitch::new(NoteName::C, Accidental::Natural, None);
        assert_eq!(c.to_midi(), None);
    }

    #[test]
    fn test_to_midi_out_of_range() {
        let low = Pitch::new(NoteName::C, Accidental::Natural, Some(0));
        assert_eq!(low.to_midi(), Some(12));
        let high = Pitch::new(NoteName::G, Accidental::Natural, Some(9));
        assert_eq!(high.to_midi(), Some(127));
    }

    // ── 调号应用测试 ──

    #[test]
    fn test_apply_key_signature_c_major() {
        // C 大调：无升降号，所有音保持 Natural
        let f = Pitch::new(NoteName::F, Accidental::Natural, Some(4));
        let result = f.apply_key_signature(NoteName::C, None);
        assert_eq!(result.acc, Accidental::Natural);
    }

    #[test]
    fn test_apply_key_signature_g_major() {
        // G 大调：F#
        let f = Pitch::new(NoteName::F, Accidental::Natural, Some(4));
        let result = f.apply_key_signature(NoteName::G, None);
        assert_eq!(result.acc, Accidental::Sharp);
    }

    #[test]
    fn test_apply_key_signature_explicit_acc_preserved() {
        // 已有显式变音记号的音高不被调号修改
        let f_flat = Pitch::new(NoteName::F, Accidental::Flat, Some(4));
        let result = f_flat.apply_key_signature(NoteName::G, None);
        assert_eq!(result.acc, Accidental::Flat);
    }
}
