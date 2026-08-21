//! 乐谱结构 — 纯领域模型。
//!
//! 层级：`Score` → `Track` → `Section` → `Measure` → `MeasureEvent`

use super::chord::Chord;
use super::note::Note;
use super::key::Key;
use super::tempo::Tempo;
use super::instrument::InstrumentKind;
use super::duration::Duration;

/// 拍号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSig {
    pub beats_per_bar: u32,
    pub beat_value: u32,
}

impl TimeSig {
    pub fn new(beats_per_bar: u32, beat_value: u32) -> Self {
        Self {
            beats_per_bar: beats_per_bar.max(1),
            beat_value: beat_value.max(1),
        }
    }

    pub fn global(beats_per_bar: u32, beat_value: u32) -> Self {
        Self::new(beats_per_bar, beat_value)
    }

    pub fn local(beats_per_bar: u32, beat_value: u32) -> Self {
        Self::new(beats_per_bar, beat_value)
    }

    /// 一小节的相对时值（以全音符 = 1.0 为单位）。
    pub fn bar_total_value(&self) -> f32 {
        let beat_unit = 1.0 / self.beat_value as f32;
        beat_unit * self.beats_per_bar as f32
    }

    /// 一小节相当于多少个四分音符。
    pub fn bar_quarter_notes(&self) -> f32 {
        self.bar_total_value() * 4.0
    }

    pub fn display(&self) -> String {
        format!("time({}/{})", self.beats_per_bar, self.beat_value)
    }
}

/// 踏板类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PedalKind {
    Sustain,
    Soft,
    Sostenuto,
}

/// 局部控制事件。
#[derive(Debug, Clone, PartialEq)]
pub enum LocalControl {
    LocalKey(Key),
    LocalTempo(Tempo),
    LocalTime(TimeSig),
    PedalOn(PedalKind),
    PedalOff(PedalKind),
    /// 显式音量（0-127）。
    Volume(u8),
    /// 动态记号（p / mp / mf / f / crescendo / decrescendo …）。
    DynamicMark(String),
}

/// 连音符。
#[derive(Debug, Clone)]
pub struct Tuplet {
    /// 比例，如 (3, 2) 表示三连音（3 个音占 2 个音的时值）。
    pub ratio: (u32, u32),
    pub events: Vec<MeasureEvent>,
}

impl Tuplet {
    pub fn new(ratio: (u32, u32)) -> Self {
        assert!(ratio.0 > 0, "tuplet numerator must be > 0");
        assert!(ratio.1 > 0, "tuplet denominator must be > 0");
        Self { ratio, events: Vec::new() }
    }

    pub fn push_event(&mut self, event: MeasureEvent) {
        self.events.push(event);
    }
}

/// 小节事件。
#[derive(Debug, Clone)]
pub enum MeasureEvent {
    Note(Note),
    Chord(Chord),
    Control(LocalControl),
    Tuplet(Tuplet),
    /// 装饰音（时值极短，不占拍）。
    Grace(Note),
}

/// 小节。
#[derive(Debug, Clone)]
pub struct Measure {
    pub events: Vec<MeasureEvent>,
    pub index: u32,
}

impl Measure {
    pub fn new(index: u32) -> Self {
        Self { events: Vec::new(), index }
    }

    pub fn push_event(&mut self, event: MeasureEvent) {
        self.events.push(event);
    }
}

/// 段落。
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub repeat_times: Option<u32>,
    pub measures: Vec<Measure>,
}

impl Section {
    pub fn new(name: String) -> Self {
        Self { name, repeat_times: None, measures: Vec::new() }
    }

    pub fn set_repeat(&mut self, times: u32) {
        self.repeat_times = Some(times);
    }

    pub fn push_measure(&mut self, measure: Measure) {
        self.measures.push(measure);
    }
}

/// 轨道。
#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub track_id: usize,
    pub instrument: Option<InstrumentKind>,
    pub sections: Vec<Section>,
}

impl Track {
    pub fn new(name: String, track_id: usize) -> Self {
        Self { name, track_id, instrument: None, sections: Vec::new() }
    }

    pub fn set_instrument(&mut self, inst: InstrumentKind) {
        self.instrument = Some(inst);
    }

    pub fn push_section(&mut self, sec: Section) {
        self.sections.push(sec);
    }
}

/// 乐谱。
#[derive(Debug, Clone)]
pub struct Score {
    pub title: Option<String>,
    pub global_key: Option<Key>,
    pub global_tempo: Option<Tempo>,
    pub global_time: Option<TimeSig>,
    /// 默认时值（由 @dur(N) 设置，未设置时为四分音符）。
    pub default_duration: Duration,
    pub tracks: Vec<Track>,
}

impl Score {
    pub fn empty() -> Self {
        Self {
            title: None,
            global_key: None,
            global_tempo: None,
            global_time: None,
            default_duration: Duration::quarter(),
            tracks: Vec::new(),
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    pub fn set_global_key(&mut self, key: Key) {
        self.global_key = Some(key);
    }

    pub fn set_global_tempo(&mut self, tempo: Tempo) {
        self.global_tempo = Some(tempo);
    }

    pub fn set_global_time(&mut self, sig: TimeSig) {
        self.global_time = Some(sig);
    }

    pub fn push_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// 默认力度。
    pub const DEFAULT_VELOCITY: u8 = 100;

    /// 全局 BPM（无设置时返回 120）。
    pub fn global_bpm(&self) -> u16 {
        self.global_tempo.as_ref().map(|t| t.bpm()).unwrap_or(120)
    }

    /// 收集所有音符（不展开段落重复，不递归 Tuplet）。
    pub fn all_notes(&self) -> Vec<&Note> {
        let mut notes = Vec::new();
        for track in &self.tracks {
            for section in &track.sections {
                for measure in &section.measures {
                    collect_notes(&measure.events, &mut notes);
                }
            }
        }
        notes
    }

    /// 收集所有和弦（不展开段落重复，不递归 Tuplet）。
    pub fn all_chords(&self) -> Vec<&Chord> {
        let mut chords = Vec::new();
        for track in &self.tracks {
            for section in &track.sections {
                for measure in &section.measures {
                    collect_chords(&measure.events, &mut chords);
                }
            }
        }
        chords
    }

    /// 整体转调：将所有音符/和弦按半音数偏移。
    pub fn transpose(&mut self, semitones: i8) {
        if let Some(ref mut key) = self.global_key {
            *key = key.transpose(semitones);
        }
        for track in &mut self.tracks {
            for section in &mut track.sections {
                for measure in &mut section.measures {
                    transpose_events(&mut measure.events, semitones);
                }
            }
        }
    }
}

// ── 递归辅助函数 ──────────────────────────────────────────

fn collect_notes<'a>(events: &'a [MeasureEvent], notes: &mut Vec<&'a Note>) {
    for event in events {
        match event {
            MeasureEvent::Note(n) | MeasureEvent::Grace(n) => notes.push(n),
            MeasureEvent::Tuplet(t) => collect_notes(&t.events, notes),
            _ => {}
        }
    }
}

fn collect_chords<'a>(events: &'a [MeasureEvent], chords: &mut Vec<&'a Chord>) {
    for event in events {
        match event {
            MeasureEvent::Chord(c) => chords.push(c),
            MeasureEvent::Tuplet(t) => collect_chords(&t.events, chords),
            _ => {}
        }
    }
}

fn transpose_events(events: &mut [MeasureEvent], semitones: i8) {
    for event in events {
        match event {
            MeasureEvent::Note(n) | MeasureEvent::Grace(n) => n.transpose(semitones),
            MeasureEvent::Chord(c) => c.transpose(semitones),
            MeasureEvent::Tuplet(t) => transpose_events(&mut t.events, semitones),
            MeasureEvent::Control(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pitch::{NoteName, Accidental, Pitch};
    use super::super::chord::{ChordSymbol, ChordQuality};

    #[test]
    fn test_time_sig() {
        let ts = TimeSig::new(4, 4);
        assert_eq!(ts.bar_quarter_notes(), 4.0);

        let ts = TimeSig::new(3, 4);
        assert_eq!(ts.bar_quarter_notes(), 3.0);

        let ts = TimeSig::new(6, 8);
        assert!((ts.bar_quarter_notes() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_score_basic() {
        let mut score = Score::empty();
        score.set_title("Test".to_string());
        score.set_global_key(Key::major(Pitch::new(NoteName::C, Accidental::Natural, None)));
        score.set_global_tempo(Tempo::new(120));
        score.set_global_time(TimeSig::new(4, 4));

        assert_eq!(score.title, Some("Test".to_string()));
        assert!(score.global_key.is_some());
        assert_eq!(score.global_bpm(), 120);
    }

    #[test]
    fn test_score_transpose() {
        let mut score = Score::empty();
        score.set_global_key(Key::major(Pitch::new(NoteName::C, Accidental::Natural, None)));

        let mut track = Track::new("piano".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);
        measure.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            super::super::duration::Duration::quarter(),
            0,
        )));
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        score.transpose(2);

        let notes = score.all_notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch().unwrap().name, NoteName::D);
    }

    #[test]
    fn test_all_notes_and_chords() {
        let mut score = Score::empty();
        let mut track = Track::new("test".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);

        measure.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            super::super::duration::Duration::quarter(),
            0,
        )));
        measure.push_event(MeasureEvent::Note(Note::new_rest(
            super::super::duration::Duration::quarter(),
            0,
        )));

        let sym = super::super::chord::ChordSymbol::new(
            Pitch::new(NoteName::G, Accidental::Natural, None),
        )
        .with_quality(super::super::chord::ChordQuality::Maj);
        measure.push_event(MeasureEvent::Chord(Chord::new_normal(
            sym,
            super::super::duration::Duration::quarter(),
            0,
        )));

        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        assert_eq!(score.all_notes().len(), 2);
        assert_eq!(score.all_chords().len(), 1);
    }

    #[test]
    fn test_all_notes_in_tuplet() {
        let mut score = Score::empty();
        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);

        let mut tuplet = Tuplet::new((3, 2));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::G, Accidental::Natural, Some(4)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        measure.push_event(MeasureEvent::Tuplet(tuplet));
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        assert_eq!(score.all_notes().len(), 3);
    }

    #[test]
    fn test_all_notes_in_grace() {
        let mut score = Score::empty();
        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);

        measure.push_event(MeasureEvent::Grace(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(5)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        assert_eq!(score.all_notes().len(), 1);
    }

    #[test]
    fn test_transpose_recursive_tuplet() {
        let mut score = Score::empty();
        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);

        let mut tuplet = Tuplet::new((3, 2));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            super::super::duration::Duration::eighth(),
            0,
        )));
        measure.push_event(MeasureEvent::Tuplet(tuplet));
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        score.transpose(4);
        let notes = score.all_notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].pitch().unwrap().name, NoteName::E);
        assert_eq!(notes[1].pitch().unwrap().name, NoteName::G);
        assert_eq!(notes[1].pitch().unwrap().acc, Accidental::Sharp);
    }

    #[test]
    fn test_score_clone() {
        let mut score = Score::empty();
        score.set_title("Test".to_string());
        score.set_global_tempo(Tempo::new(120));

        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        let mut measure = Measure::new(0);
        measure.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            super::super::duration::Duration::quarter(),
            0,
        )));
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        let score2 = score.clone();
        assert_eq!(score2.title, Some("Test".to_string()));
        assert_eq!(score2.global_bpm(), 120);
        assert_eq!(score2.tracks.len(), 1);
    }

    #[test]
    fn test_chord_in_tuplet() {
        // 和弦能否嵌套在连音符内：解析 → Tuplet{Chord}
        let mut tuplet = Tuplet::new((3, 2));
        let sym = ChordSymbol::new(Pitch::new(NoteName::D, Accidental::Natural, None))
            .with_quality(ChordQuality::Maj);
        tuplet.push_event(MeasureEvent::Chord(Chord::new_normal(sym, Duration::eighth(), 0)));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::F, Accidental::Sharp, Some(4)),
            Duration::eighth(),
            0,
        )));

        let mut measure = Measure::new(0);
        measure.push_event(MeasureEvent::Tuplet(tuplet));
        assert_eq!(measure.events.len(), 1);

        // 检查 all_notes 能正确进入 tuplet 内部找和弦
        let mut score = Score::empty();
        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        section.push_measure(measure);
        track.push_section(section);
        score.push_track(track);

        let chords = score.all_chords();
        assert_eq!(chords.len(), 1, "and弦应能在 tuplet 内被找到");
        let chord = chords[0];
        assert_eq!(chord.symbol.root.name, NoteName::D);
        assert_eq!(chord.symbol.quality, Some(ChordQuality::Maj));
    }

    #[test]
    fn test_note_in_tuplet() {
        let mut tuplet = Tuplet::new((3, 2));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::G, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));

        let mut score = Score::empty();
        let mut track = Track::new("t".to_string(), 0);
        let mut section = Section::new("A".to_string());
        section.push_measure({
            let mut m = Measure::new(0);
            m.push_event(MeasureEvent::Tuplet(tuplet));
            m
        });
        track.push_section(section);
        score.push_track(track);

        assert_eq!(score.all_notes().len(), 3, "tuplet 内的三个音符都应被找到");
    }
}
