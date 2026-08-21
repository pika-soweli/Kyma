//! 二进制中间表示 (Bin Musi IR) — 编码器。
//!
//! 将 `sonus::Score` 编码为 `.bm` 二进制格式。
//! 播放由 `perform` 模块直接读取 `.bm` 文件完成。
//!
//! ## .bm 格式 v2
//!
//! ```text
//! Header:
//!   magic: [u8; 4] = b"BMIR"
//!   version: u16 LE = 2
//!   flags: u8  (bit0=title, bit1=key, bit2=tempo, bit3=time, bit4=default_dur)
//!   [title]     string
//!   [global_key]  Key
//!   [global_tempo] u16 LE
//!   [global_time]  u8 beats, u8 beat_value
//!   track_count: u16 LE
//!
//! Track:  name(string) has_inst(u8) [inst(u8)] section_count(u16) [Section]*
//! Section: name(string) repeat(u8) measure_count(u16) [Measure]*
//! Measure: event_count(u16) [Event]*
//! Event:   tag(u8) [Note|Rest|Chord|Control|Tuplet|Grace]
//!
//! Note:   midi(u8) duration(base:u16, dotted:u8) velocity(u8)
//! Rest:   duration(base:u16, dotted:u8)
//! Chord:  midi_count(u8) [midi(u8)]* duration(base:u16, dotted:u8) velocity(u8)
//! Grace:  midi(u8) duration(base:u16, dotted:u8) velocity(u8)
//!
//! Key:    root(Pitch) scale_type(u8)
//! Pitch:  note_name(u8) accidental(u8) has_octave(u8) [octave(u8)]
//! string: len(u16 LE) + UTF-8 bytes
//! ```

use sonus::{
    Accidental, Chord, Duration, Key,
    LocalControl, Measure, MeasureEvent, PedalKind, Pitch, ScaleType, Score,
    Section, Track,
};

// ── 枚举 ↔ u8 转换 ────────────────────────────────────────

const ALL_SCALE_TYPES: [ScaleType; 30] = [
    ScaleType::Major, ScaleType::Dorian, ScaleType::Phrygian, ScaleType::Lydian,
    ScaleType::Mixolydian, ScaleType::Minor, ScaleType::Locrian,
    ScaleType::HarmonicMinor, ScaleType::MelodicMinor,
    ScaleType::MajorPentatonic, ScaleType::MinorPentatonic,
    ScaleType::Blues,
    ScaleType::WholeTone, ScaleType::Chromatic, ScaleType::Octatonic,
    ScaleType::HungarianMinor, ScaleType::PhrygianDominant,
    ScaleType::NeapolitanMajor, ScaleType::NeapolitanMinor,
    ScaleType::Enigmatic, ScaleType::Oriental, ScaleType::HungarianGypsy,
    ScaleType::Romanian, ScaleType::Persian, ScaleType::Arabic, ScaleType::Byzantine,
    ScaleType::Egyptian, ScaleType::Hindu,
    ScaleType::Hirajoshi, ScaleType::Insen,
];

fn scale_to_u8(st: ScaleType) -> u8 {
    ALL_SCALE_TYPES.iter().position(|&s| s == st).unwrap_or(0) as u8
}

fn acc_to_u8(a: Accidental) -> u8 {
    match a {
        Accidental::Natural => 0,
        Accidental::Sharp => 1,
        Accidental::DoubleSharp => 2,
        Accidental::Flat => 3,
        Accidental::DoubleFlat => 4,
    }
}

fn pedal_to_u8(p: PedalKind) -> u8 {
    match p {
        PedalKind::Sustain => 0,
        PedalKind::Soft => 1,
        PedalKind::Sostenuto => 2,
    }
}

// ── 二进制写入器 ──────────────────────────────────────────

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.u16(bytes.len() as u16);
        self.buf.extend_from_slice(bytes);
    }

    fn pitch(&mut self, p: &Pitch) {
        self.u8(p.name.index());
        self.u8(acc_to_u8(p.acc));
        match p.octave {
            Some(oct) => {
                self.u8(1);
                self.u8(oct);
            }
            None => self.u8(0),
        }
    }

    fn key(&mut self, k: &Key) {
        self.pitch(&k.root);
        self.u8(scale_to_u8(k.scale_type));
    }

    fn duration(&mut self, d: &Duration) {
        self.u16(d.base as u16);
        self.u8(if d.dotted { 1 } else { 0 });
    }

    fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

// ── 公共 API ──────────────────────────────────────────────

const MAGIC: &[u8; 4] = b"BMIR";
const VERSION: u16 = 2;

/// 将 Score 编码为 .bm 二进制。
pub fn encode(score: &Score) -> Vec<u8> {
    let mut w = Writer::new();

    w.buf.extend_from_slice(MAGIC);
    w.u16(VERSION);

    let mut flags: u8 = 0;
    if score.title.is_some() {
        flags |= 1;
    }
    if score.global_key.is_some() {
        flags |= 2;
    }
    if score.global_tempo.is_some() {
        flags |= 4;
    }
    if score.global_time.is_some() {
        flags |= 8;
    }
    if score.default_duration != Duration::quarter() {
        flags |= 16;
    }
    w.u8(flags);

    if let Some(ref title) = score.title {
        w.string(title);
    }
    if let Some(ref key) = score.global_key {
        w.key(key);
    }
    if let Some(ref tempo) = score.global_tempo {
        w.u16(tempo.bpm());
    }
    if let Some(ref time) = score.global_time {
        w.u8(time.beats_per_bar as u8);
        w.u8(time.beat_value as u8);
    }
    if flags & 16 != 0 {
        w.u8(score.default_duration.base as u8);
        w.u8(if score.default_duration.dotted { 1 } else { 0 });
    }

    w.u16(score.tracks.len() as u16);
    for track in &score.tracks {
        encode_track(&mut w, track);
    }

    w.into_bytes()
}

fn encode_track(w: &mut Writer, track: &Track) {
    w.string(&track.name);

    match &track.instrument {
        Some(inst) => {
            w.u8(1);
            w.u8(inst.index());
        }
        None => w.u8(0),
    }

    w.u16(track.sections.len() as u16);
    for section in &track.sections {
        encode_section(w, section);
    }
}

fn encode_section(w: &mut Writer, section: &Section) {
    w.string(&section.name);
    w.u8(section.repeat_times.unwrap_or(0) as u8);
    w.u16(section.measures.len() as u16);
    for measure in &section.measures {
        encode_measure(w, measure);
    }
}

fn encode_measure(w: &mut Writer, measure: &Measure) {
    w.u16(measure.events.len() as u16);
    for event in &measure.events {
        encode_event(w, event);
    }
}

fn encode_event(w: &mut Writer, event: &MeasureEvent) {
    match event {
        MeasureEvent::Note(note) => {
            if note.is_rest() {
                w.u8(1);
                w.duration(&note.duration);
            } else {
                w.u8(0);
                let midi = note.to_midi().expect("note pitch has no octave or is out of MIDI range");
                w.u8(midi);
                w.duration(&note.duration);
                w.u8(note.velocity());
            }
        }
        MeasureEvent::Chord(chord) => {
            w.u8(2);
            encode_chord(w, chord);
        }
        MeasureEvent::Control(ctrl) => {
            w.u8(3);
            encode_control(w, ctrl);
        }
        MeasureEvent::Tuplet(tuplet) => {
            w.u8(4);
            w.u8(tuplet.ratio.0 as u8);
            w.u8(tuplet.ratio.1 as u8);
            w.u16(tuplet.events.len() as u16);
            for e in &tuplet.events {
                encode_event(w, e);
            }
        }
        MeasureEvent::Grace(note) => {
            w.u8(5);
            let midi = note.to_midi().expect("grace note pitch has no octave or is out of MIDI range");
            w.u8(midi);
            w.duration(&note.duration);
            w.u8(note.velocity());
        }
    }
}

fn encode_control(w: &mut Writer, ctrl: &LocalControl) {
    match ctrl {
        LocalControl::LocalKey(k) => {
            w.u8(0);
            w.key(k);
        }
        LocalControl::LocalTempo(t) => {
            w.u8(1);
            w.u16(t.bpm());
        }
        LocalControl::LocalTime(ts) => {
            w.u8(2);
            w.u8(ts.beats_per_bar as u8);
            w.u8(ts.beat_value as u8);
        }
        LocalControl::PedalOn(p) => {
            w.u8(3);
            w.u8(pedal_to_u8(*p));
        }
        LocalControl::PedalOff(p) => {
            w.u8(4);
            w.u8(pedal_to_u8(*p));
        }
        LocalControl::Volume(v) => {
            w.u8(5);
            w.u8(*v);
        }
        LocalControl::DynamicMark(s) => {
            w.u8(6);
            w.string(s);
        }
    }
}

fn encode_chord(w: &mut Writer, chord: &Chord) {
    let midis = chord.to_midi(4).expect("chord cannot be resolved to valid MIDI notes");
    w.u8(midis.len() as u8);
    for midi in &midis {
        w.u8(*midi);
    }
    w.duration(&chord.duration);
    w.u8(chord.velocity());
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonus::{
        ChordQuality, ChordSymbol, InstrumentKind, Note, NoteName,
        Tempo, Tuplet,
    };
    use perform::reader;
    use perform::PerfEvent;

    fn encode_and_read(score: &Score) -> perform::PerformScore {
        let bytes = encode(score);
        reader::read(&bytes).unwrap()
    }

    #[test]
    fn test_encode_read_notes() {
        let mut score = Score::empty();
        score.set_title("Test".into());
        score.set_global_tempo(Tempo::new(120));

        let mut track = Track::new("piano".into(), 0);
        track.set_instrument(InstrumentKind::AcousticPiano);
        let mut section = Section::new("A".into());
        let mut m = Measure::new(0);
        m.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            Duration::quarter(),
            0,
        )));
        m.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            Duration::quarter(),
            0,
        )));
        m.push_event(MeasureEvent::Note(Note::new_rest(Duration::half(), 0)));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let perf = encode_and_read(&score);
        assert_eq!(perf.title, Some("Test".into()));
        assert_eq!(perf.global_tempo, 120);
        assert_eq!(perf.tracks.len(), 1);
        assert_eq!(perf.tracks[0].instrument, Some(0));

        let events = &perf.tracks[0].sections[0].measures[0].events;
        assert_eq!(events.len(), 3);
        match &events[0] {
            PerfEvent::Note { midi, velocity, .. } => {
                assert_eq!(*midi, 60);
                assert_eq!(*velocity, 100);
            }
            _ => panic!("expected Note"),
        }
        match &events[1] {
            PerfEvent::Note { midi, .. } => assert_eq!(*midi, 64),
            _ => panic!("expected Note"),
        }
        assert!(matches!(&events[2], PerfEvent::Rest { .. }));
    }

    #[test]
    fn test_encode_read_chord() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("A".into());
        let mut m = Measure::new(0);

        let sym = ChordSymbol::new(Pitch::new(NoteName::C, Accidental::Natural, None))
            .with_quality(ChordQuality::Maj);
        m.push_event(MeasureEvent::Chord(Chord::new_normal(sym, Duration::whole(), 0)));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let perf = encode_and_read(&score);
        match &perf.tracks[0].sections[0].measures[0].events[0] {
            PerfEvent::Chord { midis, .. } => {
                assert!(!midis.is_empty());
                assert_eq!(midis[0], 60);
            }
            _ => panic!("expected Chord"),
        }
    }

    #[test]
    fn test_encode_read_control() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("A".into());
        let mut m = Measure::new(0);

        m.push_event(MeasureEvent::Control(LocalControl::LocalTempo(Tempo::new(140))));
        m.push_event(MeasureEvent::Control(LocalControl::PedalOn(PedalKind::Sustain)));
        m.push_event(MeasureEvent::Control(LocalControl::Volume(80)));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let perf = encode_and_read(&score);
        let events = &perf.tracks[0].sections[0].measures[0].events;
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], PerfEvent::Control(perform::PerfControl::Tempo(140))));
        assert!(matches!(&events[1], PerfEvent::Control(perform::PerfControl::PedalOn(0))));
        assert!(matches!(&events[2], PerfEvent::Control(perform::PerfControl::Volume(80))));
    }

    #[test]
    fn test_encode_read_tuplet() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("A".into());
        let mut m = Measure::new(0);

        let mut tuplet = Tuplet::new((3, 2));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::C, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::D, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        tuplet.push_event(MeasureEvent::Note(Note::new_note(
            Pitch::new(NoteName::E, Accidental::Natural, Some(4)),
            Duration::eighth(),
            0,
        )));
        m.push_event(MeasureEvent::Tuplet(tuplet));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let perf = encode_and_read(&score);
        match &perf.tracks[0].sections[0].measures[0].events[0] {
            PerfEvent::Tuplet { ratio, events } => {
                assert_eq!(*ratio, (3, 2));
                assert_eq!(events.len(), 3);
            }
            _ => panic!("expected Tuplet"),
        }
    }

    #[test]
    fn test_encode_read_grace() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("A".into());
        let mut m = Measure::new(0);

        let grace_note = Note::new_note(
            Pitch::new(NoteName::D, Accidental::Natural, Some(5)),
            Duration::eighth(),
            0,
        );
        m.push_event(MeasureEvent::Grace(grace_note));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let perf = encode_and_read(&score);
        match &perf.tracks[0].sections[0].measures[0].events[0] {
            PerfEvent::Grace { midi, velocity, .. } => {
                assert_eq!(*midi, 74); // D5
                assert_eq!(*velocity, 100);
            }
            _ => panic!("expected Grace"),
        }
    }

    #[test]
    fn test_encode_read_empty() {
        let score = Score::empty();
        let perf = encode_and_read(&score);
        assert_eq!(perf.tracks.len(), 0);
    }
}
