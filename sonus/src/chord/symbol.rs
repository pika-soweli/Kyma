//! Lead-sheet 和弦符号与和弦实体。
//!
//! ## 符号结构
//!
//! `ChordSymbol = root + quality + extension + alterations`
//!
//! 例：`Cmaj7#5` = root=C, quality=Maj, extension=maj7, alter=#5

use super::quality::ChordQuality;
use super::super::pitch::{Accidental, Pitch};
use super::super::duration::Duration;

// ── 变更类型 ──────────────────────────────────────────────

/// 和弦音变更类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlterType {
    Sharp,
    Flat,
    Add,
    No,
}

impl AlterType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sharp" | "#" => Some(Self::Sharp),
            "flat" | "b" => Some(Self::Flat),
            "add" => Some(Self::Add),
            "no" => Some(Self::No),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sharp => "#",
            Self::Flat => "b",
            Self::Add => "add",
            Self::No => "no",
        }
    }
}

/// 和弦变更项：变更类型 + 度数。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChordAlterItem {
    pub alter_type: AlterType,
    pub number: u32,
}

// ── 和弦符号 ──────────────────────────────────────────────

/// Lead-sheet 和弦符号。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChordSymbol {
    pub root: Pitch,
    pub quality: Option<ChordQuality>,
    /// 扩展音度数：6, 7, 9, 11, 13。
    pub base_number: Option<u32>,
    /// 是否为大七度（true=maj7, false=属七）。
    pub major_seventh: bool,
    pub alters: Vec<ChordAlterItem>,
}

impl ChordSymbol {
    pub fn new(root: Pitch) -> Self {
        Self {
            root,
            quality: None,
            base_number: None,
            major_seventh: false,
            alters: Vec::new(),
        }
    }

    pub fn with_quality(mut self, q: ChordQuality) -> Self {
        self.quality = Some(q);
        self
    }

    pub fn with_extension(mut self, num: u32, major_seventh: bool) -> Self {
        self.base_number = Some(num);
        self.major_seventh = major_seventh;
        self
    }

    /// 返回和弦包含的音级类列表（0-11，含 slash bass）。
    ///
    /// 这是纯领域运算，不涉及 MIDI。
    pub fn pitch_classes(&self) -> Vec<u8> {
        let root_semi = self.root.semitone();
        let mut pcs: Vec<u8> = Vec::new();

        // 1. 三和弦音程
        if let Some(q) = &self.quality {
            for &interval in q.intervals() {
                push_pc(&mut pcs, root_semi, interval);
            }
        } else {
            push_pc(&mut pcs, root_semi, 0);
        }

        // 2. 扩展音
        if let Some(num) = self.base_number {
            if num == 6 {
                push_pc(&mut pcs, root_semi, 9); // 大六度
            }
            if num >= 7 {
                let seventh = if self.major_seventh { 11 } else { 10 };
                push_pc(&mut pcs, root_semi, seventh);
            }
            if num >= 9 {
                push_pc(&mut pcs, root_semi, 2); // 大九度 (14 mod 12)
            }
            if num >= 11 {
                push_pc(&mut pcs, root_semi, 5); // 纯十一度 (17 mod 12)
            }
            if num >= 13 {
                push_pc(&mut pcs, root_semi, 9); // 大十三度 (21 mod 12)
            }
        }

        // 3. 变更
        for alter in &self.alters {
            let degree_semi = degree_to_semitone(alter.number);
            let natural_pc = pc_of(root_semi, degree_semi);
            let altered_pc = match alter.alter_type {
                AlterType::Sharp => pc_of(root_semi, degree_semi + 1),
                AlterType::Flat => pc_of(root_semi, degree_semi - 1),
                AlterType::Add | AlterType::No => natural_pc,
            };

            match alter.alter_type {
                AlterType::No => {
                    pcs.retain(|&p| p != natural_pc);
                }
                AlterType::Sharp | AlterType::Flat => {
                    pcs.retain(|&p| p != natural_pc);
                    if !pcs.contains(&altered_pc) {
                        pcs.push(altered_pc);
                    }
                }
                AlterType::Add => {
                    if !pcs.contains(&natural_pc) {
                        pcs.push(natural_pc);
                    }
                }
            }
        }

        pcs.sort();
        pcs
    }

    pub fn display(&self) -> String {
        let mut s = self.root.display();

        if let Some(q) = &self.quality {
            s.push_str(q.as_display_str());
        }

        if let Some(num) = self.base_number {
            if num >= 7 && self.major_seventh {
                match self.quality {
                    Some(ChordQuality::Maj) | None => s.push_str("maj"),
                    _ => s.push_str("(maj"),
                }
            }
            s.push_str(&num.to_string());
            if num >= 7 && self.major_seventh {
                if !matches!(self.quality, Some(ChordQuality::Maj) | None) {
                    s.push(')');
                }
            }
        }

        for alter in &self.alters {
            s.push_str(&format!("{}{}", alter.alter_type.as_str(), alter.number));
        }

        s
    }
}

// ── 和弦实体 ──────────────────────────────────────────────

/// 和弦：符号 + 可选 slash bass + 转位 + 时值 + 轨道。
#[derive(Debug, Clone, PartialEq)]
pub struct Chord {
    pub symbol: ChordSymbol,
    pub slash_bass: Option<Pitch>,
    /// 转位：0 = 原位，1 = 第一转位，2 = 第二转位 …
    pub inversion: u8,
    pub duration: Duration,
    pub track_id: usize,
    velocity: u8,
}

impl Chord {
    pub fn new_normal(symbol: ChordSymbol, duration: Duration, track_id: usize) -> Self {
        Self { symbol, slash_bass: None, inversion: 0, duration, track_id, velocity: 100 }
    }

    pub fn new_slash(
        symbol: ChordSymbol,
        bass: Pitch,
        duration: Duration,
        track_id: usize,
    ) -> Self {
        Self { symbol, slash_bass: Some(bass), inversion: 0, duration, track_id, velocity: 100 }
    }

    /// 创建带转位的和弦。
    pub fn new_with_inversion(
        symbol: ChordSymbol,
        inversion: u8,
        duration: Duration,
        track_id: usize,
    ) -> Self {
        Self { symbol, slash_bass: None, inversion, duration, track_id, velocity: 100 }
    }

    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    pub fn set_velocity(&mut self, vel: u8) {
        self.velocity = vel.min(127);
    }

    /// 返回和弦所有音级类（含 slash bass）。
    pub fn pitch_classes(&self) -> Vec<u8> {
        let mut pcs = self.symbol.pitch_classes();
        if let Some(bass) = &self.slash_bass {
            let bass_pc = (((bass.semitone() % 12) + 12) % 12) as u8;
            if !pcs.contains(&bass_pc) {
                pcs.insert(0, bass_pc);
            }
        }
        pcs.sort();
        pcs
    }

    /// 转调：按半音数偏移和弦根音（原地修改）。
    pub fn transpose(&mut self, semitones: i8) {
        self.symbol.root = self.symbol.root.transpose(semitones);
        if let Some(ref mut bass) = self.slash_bass {
            *bass = bass.transpose(semitones);
        }
    }

    pub fn display(&self) -> String {
        let mut buf = format!("[{}", self.symbol.display());
        if self.inversion > 0 {
            buf.push_str(&format!("/{}", self.inversion));
        } else if let Some(bass) = &self.slash_bass {
            buf.push_str(&format!("/{}", bass.display()));
        }
        buf.push_str(&format!("]{}", self.duration.display()));
        buf
    }

    /// 通过 rust-music-theory 计算和弦的实际音符（含转位、slash bass）。
    ///
    /// 利用 rmt 0.4 的 `Chord::from_string` 解析和弦字符串，
    /// 并通过 `bass` 字段处理 slash chord。
    pub fn notes(&self, _octave: u8) -> Option<Vec<Pitch>> {
        use crate::rmt::note::Notes;
        let mut rmt_chord = self.symbol.to_rmt_chord()?;
        if self.inversion > 0 {
            rmt_chord.inversion = self.inversion;
        }
        if let Some(ref bass) = self.slash_bass {
            rmt_chord.bass = Some(rmt::note::Pitch::from(*bass));
        }
        let rmt_notes = rmt_chord.notes();
        Some(rmt_notes.into_iter().map(|n| {
            let rmt_pitch = n.pitch;
            Pitch::from(rmt_pitch)
        }).collect())
    }

    /// 计算和弦的 MIDI 音符值列表（0-127）。
    ///
    /// 返回和弦中所有音符的 MIDI 值。若和弦无法解析则返回 `None`。
    pub fn to_midi(&self, octave: u8) -> Option<Vec<u8>> {
        let pitches = self.notes(octave)?;
        let midis: Vec<u8> = pitches.iter().filter_map(|p| {
            let mut p_with_oct = *p;
            if p_with_oct.octave.is_none() {
                p_with_oct.octave = Some(octave);
            }
            p_with_oct.to_midi()
        }).collect();
        if midis.is_empty() {
            None
        } else {
            Some(midis)
        }
    }
}

// ── rust-music-theory 互转 ────────────────────────────────

use crate::rmt;

impl ChordSymbol {
    /// 转换为 rmt::chord::Chord — 优先使用 `Chord::from_string`。
    ///
    /// 构建 lead-sheet 字符串（如 "Cmaj7"、"Bbm"、"Gdim"），
    /// 委托 rmt 解析并计算音程。若 `from_string` 失败则回退到手动构建。
    ///
    /// alterations 不参与转换（rmt 不支持）。
    pub fn to_rmt_chord(&self) -> Option<rmt::chord::Chord> {
        let quality = self.quality?;

        // 构建 lead-sheet 字符串供 Chord::from_string 解析
        let s = self.to_rmt_string(quality);
        if let Ok(chord) = rmt::chord::Chord::from_string(&s) {
            // 验证 from_string 解析的品质与期望一致
            // （rmt 的 from_string 对 "7" 等后缀的默认品质可能不符预期）
            let expected_quality: rmt::chord::Quality = match (quality, self.base_number) {
                (ChordQuality::Dom, _) => rmt::chord::Quality::Dominant,
                _ => quality.into(),
            };
            if chord.quality == expected_quality {
                return Some(chord);
            }
        }

        // 回退：手动构建
        let rmt_pitch: rmt::note::Pitch = self.root.into();
        let rmt_quality = match (quality, self.base_number) {
            (ChordQuality::Dom, Some(7)) => rmt::chord::Quality::Dominant,
            (ChordQuality::Dom, Some(9)) => rmt::chord::Quality::Dominant,
            (ChordQuality::Dom, Some(11)) => rmt::chord::Quality::Dominant,
            (ChordQuality::Dom, Some(13)) => rmt::chord::Quality::Dominant,
            _ => quality.into(),
        };
        let number = self.to_rmt_number();
        Some(rmt::chord::Chord::new(rmt_pitch, rmt_quality, number))
    }

    /// 构建 rmt 可解析的 lead-sheet 字符串。
    fn to_rmt_string(&self, quality: ChordQuality) -> String {
        let mut s = String::new();

        // 根音
        s.push(self.root.name.as_char());
        match self.root.acc {
            Accidental::Sharp => s.push('#'),
            Accidental::Flat => s.push('b'),
            Accidental::DoubleSharp => s.push_str("##"),
            Accidental::DoubleFlat => s.push_str("bb"),
            Accidental::Natural => {}
        }

        // 品质
        match quality {
            ChordQuality::Min => s.push('m'),
            ChordQuality::Dim => s.push_str("dim"),
            ChordQuality::Aug => s.push_str("aug"),
            ChordQuality::Sus2 => s.push_str("sus2"),
            ChordQuality::Sus4 => s.push_str("sus4"),
            ChordQuality::HalfDim => s.push_str("m7b5"),
            ChordQuality::Dom => {} // 默认，由 extension 决定
            ChordQuality::Maj | ChordQuality::Power => {} // 默认
        }

        // 扩展
        match self.base_number {
            Some(7) if self.major_seventh => s.push_str("maj7"),
            Some(7) => s.push('7'),
            Some(9) if self.major_seventh => s.push_str("maj9"),
            Some(9) => s.push('9'),
            Some(11) => s.push_str("11"),
            Some(13) => s.push_str("13"),
            _ => {}
        }

        s
    }

    /// 将 base_number 映射为 rmt::chord::Number。
    fn to_rmt_number(&self) -> rmt::chord::Number {
        match self.base_number {
            None => rmt::chord::Number::Triad,
            Some(6) => rmt::chord::Number::Triad,
            Some(7) if self.major_seventh => rmt::chord::Number::MajorSeventh,
            Some(7) => rmt::chord::Number::Seventh,
            Some(9) if self.major_seventh => rmt::chord::Number::Ninth,
            Some(9) => rmt::chord::Number::Ninth,
            Some(11) => rmt::chord::Number::Eleventh,
            Some(13) => rmt::chord::Number::Thirteenth,
            _ => rmt::chord::Number::Triad,
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

fn push_pc(pcs: &mut Vec<u8>, root_semi: i8, interval: i8) {
    let pc = pc_of(root_semi, interval);
    if !pcs.contains(&pc) {
        pcs.push(pc);
    }
}

fn pc_of(root_semi: i8, interval: i8) -> u8 {
    (((root_semi + interval) % 12 + 12) % 12) as u8
}

/// 度数 → 半音偏移（基于大音阶）。
fn degree_to_semitone(degree: u32) -> i8 {
    let octave = ((degree.saturating_sub(1)) / 7) as i8;
    let in_octave = ((degree.saturating_sub(1)) % 7) as u8;
    let base = match in_octave {
        0 => 0,  // 1度
        1 => 2,  // 2度
        2 => 4,  // 3度
        3 => 5,  // 4度
        4 => 7,  // 5度
        5 => 9,  // 6度
        6 => 11, // 7度
        _ => 0,
    };
    base + octave * 12
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::pitch::{NoteName, Accidental};

    fn pitch(name: NoteName, acc: Accidental) -> Pitch {
        Pitch::new(name, acc, None)
    }

    #[test]
    fn test_major_triad_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        assert_eq!(sym.pitch_classes(), vec![0, 4, 7]);
    }

    #[test]
    fn test_minor_triad_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Min);
        assert_eq!(sym.pitch_classes(), vec![0, 3, 7]);
    }

    #[test]
    fn test_dominant_7th_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, false);
        assert_eq!(sym.pitch_classes(), vec![0, 4, 7, 10]);
    }

    #[test]
    fn test_major_7th_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, true);
        assert_eq!(sym.pitch_classes(), vec![0, 4, 7, 11]);
    }

    #[test]
    fn test_minor_7th_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Min)
            .with_extension(7, false);
        assert_eq!(sym.pitch_classes(), vec![0, 3, 7, 10]);
    }

    #[test]
    fn test_6th_chord_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(6, false);
        assert_eq!(sym.pitch_classes(), vec![0, 4, 7, 9]);
    }

    #[test]
    fn test_9th_chord_pcs() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(9, false);
        assert_eq!(sym.pitch_classes(), vec![0, 2, 4, 7, 10]);
    }

    #[test]
    fn test_alter_sharp_5() {
        let mut sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, false);
        sym.alters.push(ChordAlterItem { alter_type: AlterType::Sharp, number: 5 });
        // #5: replace 7 with 8
        assert_eq!(sym.pitch_classes(), vec![0, 4, 8, 10]);
    }

    #[test]
    fn test_alter_flat_5() {
        let mut sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Min)
            .with_extension(7, false);
        sym.alters.push(ChordAlterItem { alter_type: AlterType::Flat, number: 5 });
        // b5: replace 7 with 6 → half-diminished
        assert_eq!(sym.pitch_classes(), vec![0, 3, 6, 10]);
    }

    #[test]
    fn test_alter_add_9() {
        let mut sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        sym.alters.push(ChordAlterItem { alter_type: AlterType::Add, number: 9 });
        assert_eq!(sym.pitch_classes(), vec![0, 2, 4, 7]);
    }

    #[test]
    fn test_alter_no_5() {
        let mut sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        sym.alters.push(ChordAlterItem { alter_type: AlterType::No, number: 5 });
        assert_eq!(sym.pitch_classes(), vec![0, 4]);
    }

    #[test]
    fn test_display_major() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        assert_eq!(sym.display(), "C");
    }

    #[test]
    fn test_display_minor_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Min)
            .with_extension(7, false);
        assert_eq!(sym.display(), "Cm7");
    }

    #[test]
    fn test_display_major_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, true);
        assert_eq!(sym.display(), "Cmaj7");
    }

    #[test]
    fn test_display_minor_major_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Min)
            .with_extension(7, true);
        assert_eq!(sym.display(), "Cm(maj7)");
    }

    #[test]
    fn test_chord_with_slash_bass() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        let chord = Chord::new_slash(
            sym,
            pitch(NoteName::G, Accidental::Natural),
            Duration::quarter(),
            0,
        );
        // C/G: C major with G bass → [0, 4, 7] + [7] = [0, 4, 7]
        assert_eq!(chord.pitch_classes(), vec![0, 4, 7]);
    }

    #[test]
    fn test_chord_transpose() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, false);
        let mut chord = Chord::new_normal(sym, Duration::quarter(), 0);
        chord.transpose(2);
        assert_eq!(chord.symbol.root, pitch(NoteName::D, Accidental::Natural));
    }

    #[test]
    fn test_chord_notes_via_rmt() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        let chord = Chord::new_normal(sym, Duration::quarter(), 0);
        let notes = chord.notes(4).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].name, NoteName::C);
        assert_eq!(notes[1].name, NoteName::E);
        assert_eq!(notes[2].name, NoteName::G);
    }

    #[test]
    fn test_chord_notes_with_inversion() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        let chord = Chord::new_with_inversion(sym, 1, Duration::quarter(), 0);
        let notes = chord.notes(4).unwrap();
        assert_eq!(notes.len(), 3);
        // First inversion: E should be the lowest note
        assert_eq!(notes[0].name, NoteName::E);
    }

    #[test]
    fn test_chord_notes_minor() {
        let sym = ChordSymbol::new(pitch(NoteName::A, Accidental::Natural))
            .with_quality(ChordQuality::Min);
        let chord = Chord::new_normal(sym, Duration::quarter(), 0);
        let notes = chord.notes(4).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].name, NoteName::A);
        assert_eq!(notes[1].name, NoteName::C);
        assert_eq!(notes[2].name, NoteName::E);
    }

    // ── Chord::from_string 路径验证 ──

    #[test]
    fn test_from_string_major_triad() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Major);
        assert_eq!(rmt_chord.number, rmt::chord::Number::Triad);
    }

    #[test]
    fn test_from_string_minor_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::A, Accidental::Natural))
            .with_quality(ChordQuality::Min)
            .with_extension(7, false);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Minor);
        assert_eq!(rmt_chord.number, rmt::chord::Number::Seventh);
    }

    #[test]
    fn test_from_string_dominant_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::G, Accidental::Natural))
            .with_quality(ChordQuality::Dom)
            .with_extension(7, false);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Dominant);
        assert_eq!(rmt_chord.number, rmt::chord::Number::Seventh);
    }

    #[test]
    fn test_from_string_major_7th() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, true);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Major);
        assert_eq!(rmt_chord.number, rmt::chord::Number::MajorSeventh);
    }

    #[test]
    fn test_from_string_dim() {
        let sym = ChordSymbol::new(pitch(NoteName::B, Accidental::Natural))
            .with_quality(ChordQuality::Dim);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Diminished);
    }

    #[test]
    fn test_from_string_halfdim() {
        let sym = ChordSymbol::new(pitch(NoteName::B, Accidental::Natural))
            .with_quality(ChordQuality::HalfDim)
            .with_extension(7, false);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::HalfDiminished);
        assert_eq!(rmt_chord.number, rmt::chord::Number::Seventh);
    }

    #[test]
    fn test_from_string_sus4() {
        let sym = ChordSymbol::new(pitch(NoteName::G, Accidental::Natural))
            .with_quality(ChordQuality::Sus4);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Suspended4);
    }

    #[test]
    fn test_from_string_flat_root() {
        let sym = ChordSymbol::new(pitch(NoteName::B, Accidental::Flat))
            .with_quality(ChordQuality::Maj);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.root.letter, rmt::note::NoteLetter::B);
        assert_eq!(rmt_chord.root.accidental, -1);
    }

    #[test]
    fn test_from_string_sharp_root() {
        let sym = ChordSymbol::new(pitch(NoteName::F, Accidental::Sharp))
            .with_quality(ChordQuality::Min);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.root.letter, rmt::note::NoteLetter::F);
        assert_eq!(rmt_chord.root.accidental, 1);
    }

    #[test]
    fn test_from_string_9th() {
        let sym = ChordSymbol::new(pitch(NoteName::C, Accidental::Natural))
            .with_quality(ChordQuality::Dom)
            .with_extension(9, false);
        let rmt_chord = sym.to_rmt_chord().unwrap();
        assert_eq!(rmt_chord.quality, rmt::chord::Quality::Dominant);
        assert_eq!(rmt_chord.number, rmt::chord::Number::Ninth);
    }
}
