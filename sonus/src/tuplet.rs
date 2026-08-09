//! 连音符 — 以 N 个音符替代 M 个标准时值。
//!
//! 语法：`3:2 {C4:8 D4:8 E4:8}` 表示三个八分音符替代两个八分音符（即三连音）。

use super::note::Note;

/// 连音符：N 个音符（由 tuplet_n 指定）在 M 拍（由 tuplet_m 指定）内演奏。
///
/// tuplet_n / tuplet_m 为比率，如 3:2 表示三连音（3个音符占2拍的时值）。
/// tuplet_notes 为连音符内的音符序列。
#[derive(Debug, Clone, PartialEq)]
pub struct Tuplet {
    pub tuplet_n: u32,
    pub tuplet_m: u32,
    pub notes: Vec<Note>,
}

impl Tuplet {
    pub fn new(tuplet_n: u32, tuplet_m: u32, notes: Vec<Note>) -> Self {
        Self {
            tuplet_n,
            tuplet_m,
            notes,
        }
    }

    /// 连音符的总标准时值（以全音符为单位）。
    /// 即 tuplet_m 拍的标准时值总和。
    pub fn base_duration(&self) -> f32 {
        if self.notes.is_empty() {
            return 0.0;
        }
        let beat_value = self.notes[0].duration.base as f32;
        let unit = 4.0 / beat_value;
        unit * self.tuplet_m as f32
    }

    /// 每个音符的实际时值（以全音符为单位）。
    pub fn note_duration(&self) -> f32 {
        self.base_duration() / self.tuplet_n as f32
    }

    pub fn display(&self) -> String {
        let inner: Vec<String> = self.notes.iter().map(|n| n.display()).collect();
        format!("{}:{}{{{}}}", self.tuplet_n, self.tuplet_m, inner.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::Note;
    use crate::pitch::{Accidental, NoteName, Pitch};
    use crate::duration::Duration;

    fn note(c: char, octave: u8, dur: Duration) -> Note {
        let name = match c {
            'C' => NoteName::C,
            'D' => NoteName::D,
            'E' => NoteName::E,
            'F' => NoteName::F,
            'G' => NoteName::G,
            'A' => NoteName::A,
            'B' => NoteName::B,
            _ => panic!("unknown note name"),
        };
        Note::new_note(Pitch::new(name, Accidental::Natural, Some(octave)), dur, 0)
    }

    #[test]
    fn test_triplet_durations() {
        let notes = vec![
            note('C', 4, Duration::eighth()),
            note('D', 4, Duration::eighth()),
            note('E', 4, Duration::eighth()),
        ];
        let tuplet = Tuplet::new(3, 2, notes);

        // 3个八分音符替代2个八分音符（即一个四分音符的时值）
        assert!((tuplet.base_duration() - 1.0).abs() < 1e-6);
        assert!((tuplet.note_duration() - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_quadruplet() {
        let notes = vec![
            note('C', 4, Duration::sixteenth()),
            note('D', 4, Duration::sixteenth()),
            note('E', 4, Duration::sixteenth()),
            note('F', 4, Duration::sixteenth()),
        ];
        let tuplet = Tuplet::new(4, 2, notes);

        // 4个十六分音符替代2个十六分音符（即一个八分音符的时值）
        assert!((tuplet.base_duration() - 0.5).abs() < 1e-6);
        assert!((tuplet.note_duration() - 0.5 / 4.0).abs() < 1e-6);
    }
}
