//! 音程计算模块 — 支持复合音程、转位与协和性判定。
//!
//! 音程由度数（degree）和质量（quality）组成。
//! 度数 1-8 为单八度内音程，9-15 为复合音程（9=复合二度，…，15=双八度）。

use super::pitch::Pitch;

/// 音程质量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalQuality {
    Perfect,
    Major,
    Minor,
    Augmented,
    Diminished,
    DoublyAugmented,
    DoublyDiminished,
}

impl IntervalQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Perfect => "P",
            Self::Major => "M",
            Self::Minor => "m",
            Self::Augmented => "A",
            Self::Diminished => "d",
            Self::DoublyAugmented => "AA",
            Self::DoublyDiminished => "dd",
        }
    }

    /// 是否为纯音程度数可用（1, 4, 5, 8 及其复合）。
    fn is_perfect_quality_for(simple_degree: u8) -> bool {
        matches!(simple_degree, 1 | 4 | 5 | 8)
    }
}

/// 音程：度数 + 质量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval {
    /// 音程度数（1=一度, …, 8=八度, 9=九度, …, 15=双八度）。
    pub degree: u8,
    pub quality: IntervalQuality,
}

impl Interval {
    pub fn new(degree: u8, quality: IntervalQuality) -> Self {
        Self { degree, quality }
    }

    /// 简化度数（1-8），复合音程返回其单八度等价。
    pub fn simple_degree(&self) -> u8 {
        ((self.degree - 1) % 7) + 1
    }

    /// 八度偏移量（0=单八度内, 1=跨一个八度, 2=跨两个八度）。
    pub fn octave_offset(&self) -> u8 {
        (self.degree - 1) / 7
    }

    /// 是否为复合音程（度数 > 8）。
    pub fn is_compound(&self) -> bool {
        self.degree > 8
    }

    /// 单八度内音程的半音数。
    fn simple_base_semitone(simple_degree: u8) -> i8 {
        match simple_degree {
            1 => 0,
            2 => 2,
            3 => 4,
            4 => 5,
            5 => 7,
            6 => 9,
            7 => 11,
            8 => 12,
            _ => 0,
        }
    }

    /// 音程的总半音数。
    pub fn semitones(&self) -> i8 {
        let simple = self.simple_degree();
        let base = Self::simple_base_semitone(simple);
        let octave = self.octave_offset() as i8 * 12;
        let offset = match self.quality {
            IntervalQuality::Perfect | IntervalQuality::Major => 0,
            IntervalQuality::Minor => -1,
            IntervalQuality::Augmented => 1,
            IntervalQuality::Diminished => {
                if IntervalQuality::is_perfect_quality_for(simple) { -1 } else { -2 }
            }
            IntervalQuality::DoublyAugmented => 2,
            IntervalQuality::DoublyDiminished => {
                if IntervalQuality::is_perfect_quality_for(simple) { -2 } else { -3 }
            }
        };
        base + octave + offset
    }

    /// 从半音数和度数推断音程。
    ///
    /// `semitones` 为总半音数（含八度偏移），`degree` 为完整度数。
    pub fn from_semitones(semitones: i8, degree: u8) -> Self {
        let simple = ((degree - 1) % 7) + 1;
        let octave = (degree - 1) / 7;
        let base = Self::simple_base_semitone(simple) + octave as i8 * 12;
        let diff = semitones - base;

        let is_perfect = IntervalQuality::is_perfect_quality_for(simple);
        let quality = if is_perfect {
            match diff {
                -2 => IntervalQuality::DoublyDiminished,
                -1 => IntervalQuality::Diminished,
                0 => IntervalQuality::Perfect,
                1 => IntervalQuality::Augmented,
                2 => IntervalQuality::DoublyAugmented,
                _ => IntervalQuality::Perfect,
            }
        } else {
            match diff {
                ..=-3 => IntervalQuality::DoublyDiminished,
                -2 => IntervalQuality::Diminished,
                -1 => IntervalQuality::Minor,
                0 => IntervalQuality::Major,
                1 => IntervalQuality::Augmented,
                2.. => IntervalQuality::DoublyAugmented,
            }
        };
        Self { degree, quality }
    }

    /// 转位：返回转位后的新音程。
    ///
    /// 简单音程转位规律：度数之和 = 9，大↔小，增↔减，纯不变。
    /// 复合音程先化为简单音程再转位。
    pub fn invert(&self) -> Self {
        let simple = self.simple_degree();
        let new_simple = 9 - simple;
        let new_degree = new_simple + self.octave_offset() * 7;

        let new_quality = match self.quality {
            IntervalQuality::Perfect => IntervalQuality::Perfect,
            IntervalQuality::Major => IntervalQuality::Minor,
            IntervalQuality::Minor => IntervalQuality::Major,
            IntervalQuality::Augmented => IntervalQuality::Diminished,
            IntervalQuality::Diminished => IntervalQuality::Augmented,
            IntervalQuality::DoublyAugmented => IntervalQuality::DoublyDiminished,
            IntervalQuality::DoublyDiminished => IntervalQuality::DoublyAugmented,
        };
        Self::new(new_degree, new_quality)
    }

    /// 是否为协和音程（纯一度、纯八度、纯四度、纯五度、大三度、小三度、大六度、小六度）。
    pub fn is_consonant(&self) -> bool {
        let simple = self.simple_degree();
        match (simple, self.quality) {
            (1, IntervalQuality::Perfect) => true,
            (3, IntervalQuality::Major | IntervalQuality::Minor) => true,
            (4, IntervalQuality::Perfect) => true,
            (5, IntervalQuality::Perfect) => true,
            (6, IntervalQuality::Major | IntervalQuality::Minor) => true,
            (8, IntervalQuality::Perfect) => true,
            _ => false,
        }
    }

    /// 是否为不协和音程。
    pub fn is_dissonant(&self) -> bool {
        !self.is_consonant()
    }

    pub fn display(&self) -> String {
        format!("{}{}", self.quality.as_str(), self.degree)
    }

    // ── 常用音程快捷构造 ──

    pub fn unison() -> Self { Self::new(1, IntervalQuality::Perfect) }
    pub fn minor_second() -> Self { Self::new(2, IntervalQuality::Minor) }
    pub fn major_second() -> Self { Self::new(2, IntervalQuality::Major) }
    pub fn minor_third() -> Self { Self::new(3, IntervalQuality::Minor) }
    pub fn major_third() -> Self { Self::new(3, IntervalQuality::Major) }
    pub fn perfect_fourth() -> Self { Self::new(4, IntervalQuality::Perfect) }
    pub fn tritone() -> Self { Self::new(4, IntervalQuality::Augmented) }
    pub fn perfect_fifth() -> Self { Self::new(5, IntervalQuality::Perfect) }
    pub fn minor_sixth() -> Self { Self::new(6, IntervalQuality::Minor) }
    pub fn major_sixth() -> Self { Self::new(6, IntervalQuality::Major) }
    pub fn minor_seventh() -> Self { Self::new(7, IntervalQuality::Minor) }
    pub fn major_seventh() -> Self { Self::new(7, IntervalQuality::Major) }
    pub fn octave() -> Self { Self::new(8, IntervalQuality::Perfect) }
    pub fn minor_ninth() -> Self { Self::new(9, IntervalQuality::Minor) }
    pub fn major_ninth() -> Self { Self::new(9, IntervalQuality::Major) }
    pub fn perfect_eleventh() -> Self { Self::new(11, IntervalQuality::Perfect) }
    pub fn major_thirteenth() -> Self { Self::new(13, IntervalQuality::Major) }
}

// ── rust-music-theory 互转 ────────────────────────────────

use crate::rmt;

impl From<IntervalQuality> for rmt::interval::Quality {
    fn from(q: IntervalQuality) -> Self {
        match q {
            IntervalQuality::Perfect => rmt::interval::Quality::Perfect,
            IntervalQuality::Major => rmt::interval::Quality::Major,
            IntervalQuality::Minor => rmt::interval::Quality::Minor,
            IntervalQuality::Augmented | IntervalQuality::DoublyAugmented => {
                rmt::interval::Quality::Augmented
            }
            IntervalQuality::Diminished | IntervalQuality::DoublyDiminished => {
                rmt::interval::Quality::Diminished
            }
        }
    }
}

impl From<rmt::interval::Quality> for IntervalQuality {
    fn from(q: rmt::interval::Quality) -> Self {
        match q {
            rmt::interval::Quality::Perfect => IntervalQuality::Perfect,
            rmt::interval::Quality::Major => IntervalQuality::Major,
            rmt::interval::Quality::Minor => IntervalQuality::Minor,
            rmt::interval::Quality::Augmented => IntervalQuality::Augmented,
            rmt::interval::Quality::Diminished => IntervalQuality::Diminished,
        }
    }
}

impl From<Interval> for rmt::interval::Interval {
    fn from(i: Interval) -> Self {
        let simple = i.simple_degree();
        let number = match simple {
            1 => rmt::interval::Number::Unison,
            2 => rmt::interval::Number::Second,
            3 => rmt::interval::Number::Third,
            4 => rmt::interval::Number::Fourth,
            5 => rmt::interval::Number::Fifth,
            6 => rmt::interval::Number::Sixth,
            7 => rmt::interval::Number::Seventh,
            8 => rmt::interval::Number::Octave,
            _ => rmt::interval::Number::Unison,
        };
        let semi = i.semitones().max(0) as u8;
        rmt::interval::Interval::from_semitone(semi).unwrap_or_else(|_| {
            rmt::interval::Interval {
                semitone_count: semi,
                quality: i.quality.into(),
                number,
                step: None,
            }
        })
    }
}

impl From<rmt::interval::Interval> for Interval {
    fn from(i: rmt::interval::Interval) -> Self {
        let degree = match i.number {
            rmt::interval::Number::Unison => 1,
            rmt::interval::Number::Second => 2,
            rmt::interval::Number::Third => 3,
            rmt::interval::Number::Fourth => 4,
            rmt::interval::Number::Fifth => 5,
            rmt::interval::Number::Sixth => 6,
            rmt::interval::Number::Seventh => 7,
            rmt::interval::Number::Octave => 8,
        };
        Interval::from_semitones(i.semitone_count as i8, degree)
    }
}

// ── 自由函数 ──────────────────────────────────────────────

/// 计算两个音高之间的音程（仅取音级部分，忽略八度）。
pub fn interval_between(a: &Pitch, b: &Pitch) -> Interval {
    let degree = ((b.name.index() as i8 - a.name.index() as i8 + 7) % 7 + 1) as u8;
    let semi_diff = ((b.semitone() - a.semitone()) % 12 + 12) % 12;
    Interval::from_semitones(semi_diff, degree)
}

/// 计算两个完整音高（含八度）之间的音程。
pub fn interval_between_pitches(a: &Pitch, b: &Pitch) -> Interval {
    let a_oct = a.octave.unwrap_or(4) as i16;
    let b_oct = b.octave.unwrap_or(4) as i16;

    let total_letter_steps = b.name.index() as i16 - a.name.index() as i16 + (b_oct - a_oct) * 7;
    let degree = (total_letter_steps + 1) as u8;

    let a_total = a.semitone() as i16 + (a_oct + 1) * 12;
    let b_total = b.semitone() as i16 + (b_oct + 1) * 12;
    let semi_diff = (b_total - a_total) as i8;

    Interval::from_semitones(semi_diff, degree)
}

/// 按音程转调一个音高（保持音名关系，返回新 Pitch）。
pub fn transpose_pitch(pitch: &Pitch, interval: &Interval) -> Pitch {
    let new_name = pitch.name.step((interval.degree as i8 - 1) % 7);
    let target_semi = pitch.semitone() + interval.semitones();
    let base_semi = new_name.base_semitone();
    let diff = target_semi - base_semi;
    let new_acc = super::pitch::Accidental::from_offset(diff);

    let new_octave = pitch.octave.map(|oct| {
        let step = (interval.degree as i8 - 1) / 7;
        (oct as i16 + pitch.name.index() as i16 + step as i16 + (interval.semitones() as i16) / 12) as u8
    });

    Pitch::new(new_name, new_acc, new_octave)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pitch::{Accidental, NoteName};

    fn p(name: NoteName, acc: Accidental, oct: Option<u8>) -> Pitch {
        Pitch::new(name, acc, oct)
    }

    #[test]
    fn test_interval_semitones() {
        assert_eq!(Interval::unison().semitones(), 0);
        assert_eq!(Interval::minor_second().semitones(), 1);
        assert_eq!(Interval::major_second().semitones(), 2);
        assert_eq!(Interval::minor_third().semitones(), 3);
        assert_eq!(Interval::major_third().semitones(), 4);
        assert_eq!(Interval::perfect_fourth().semitones(), 5);
        assert_eq!(Interval::tritone().semitones(), 6);
        assert_eq!(Interval::perfect_fifth().semitones(), 7);
        assert_eq!(Interval::octave().semitones(), 12);
    }

    #[test]
    fn test_compound_interval_semitones() {
        assert_eq!(Interval::minor_ninth().semitones(), 13);
        assert_eq!(Interval::major_ninth().semitones(), 14);
        assert_eq!(Interval::perfect_eleventh().semitones(), 17);
        assert_eq!(Interval::major_thirteenth().semitones(), 21);
    }

    #[test]
    fn test_interval_from_semitones() {
        assert_eq!(Interval::from_semitones(0, 1), Interval::unison());
        assert_eq!(Interval::from_semitones(2, 2), Interval::major_second());
        assert_eq!(Interval::from_semitones(1, 2), Interval::minor_second());
        assert_eq!(Interval::from_semitones(4, 3), Interval::major_third());
        assert_eq!(Interval::from_semitones(3, 3), Interval::minor_third());
        assert_eq!(Interval::from_semitones(5, 4), Interval::perfect_fourth());
        assert_eq!(Interval::from_semitones(7, 5), Interval::perfect_fifth());
        assert_eq!(Interval::from_semitones(12, 8), Interval::octave());
    }

    #[test]
    fn test_compound_from_semitones() {
        assert_eq!(Interval::from_semitones(14, 9), Interval::major_ninth());
        assert_eq!(Interval::from_semitones(13, 9), Interval::minor_ninth());
    }

    #[test]
    fn test_inversion() {
        assert_eq!(Interval::major_third().invert(), Interval::minor_sixth());
        assert_eq!(Interval::perfect_fifth().invert(), Interval::perfect_fourth());
        assert_eq!(Interval::minor_seventh().invert(), Interval::major_second());
        assert_eq!(Interval::unison().invert(), Interval::octave());
    }

    #[test]
    fn test_consonance() {
        assert!(Interval::unison().is_consonant());
        assert!(Interval::major_third().is_consonant());
        assert!(Interval::perfect_fifth().is_consonant());
        assert!(Interval::minor_sixth().is_consonant());
        assert!(Interval::major_second().is_dissonant());
        assert!(Interval::tritone().is_dissonant());
        assert!(Interval::minor_seventh().is_dissonant());
    }

    #[test]
    fn test_interval_between() {
        let c = p(NoteName::C, Accidental::Natural, None);
        let e = p(NoteName::E, Accidental::Natural, None);
        let g = p(NoteName::G, Accidental::Natural, None);
        let f_sharp = p(NoteName::F, Accidental::Sharp, None);

        assert_eq!(interval_between(&c, &e), Interval::major_third());
        assert_eq!(interval_between(&c, &g), Interval::perfect_fifth());
        assert_eq!(interval_between(&c, &f_sharp), Interval::tritone());
    }

    #[test]
    fn test_interval_between_inverse() {
        let c = p(NoteName::C, Accidental::Natural, None);
        let g = p(NoteName::G, Accidental::Natural, None);
        assert_eq!(interval_between(&c, &g), Interval::perfect_fifth());
        assert_eq!(interval_between(&g, &c), Interval::perfect_fourth());
    }

    #[test]
    fn test_interval_display() {
        assert_eq!(Interval::major_third().display(), "M3");
        assert_eq!(Interval::perfect_fifth().display(), "P5");
        assert_eq!(Interval::minor_seventh().display(), "m7");
        assert_eq!(Interval::major_ninth().display(), "M9");
    }

    #[test]
    fn test_is_compound() {
        assert!(!Interval::octave().is_compound());
        assert!(Interval::major_ninth().is_compound());
        assert_eq!(Interval::major_ninth().simple_degree(), 2);
        assert_eq!(Interval::major_ninth().octave_offset(), 1);
    }

    #[test]
    fn test_interval_between_pitches() {
        let c4 = p(NoteName::C, Accidental::Natural, Some(4));
        let g4 = p(NoteName::G, Accidental::Natural, Some(4));
        let c5 = p(NoteName::C, Accidental::Natural, Some(5));

        assert_eq!(interval_between_pitches(&c4, &g4), Interval::perfect_fifth());
        assert_eq!(interval_between_pitches(&c4, &c5), Interval::octave());
    }
}
