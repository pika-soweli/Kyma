//! 二进制中间表示 (Bin Musi IR) — 编码器 / 解码器。
//!
//! ## .bm 格式 v1
//!
//! ```text
//! Header:
//!   magic: [u8; 4] = b"BMIR"
//!   version: u16 LE = 1
//!   flags: u8  (bit0=title, bit1=key, bit2=tempo, bit3=time)
//!   [title]     string
//!   [global_key]  Key
//!   [global_tempo] u16 LE
//!   [global_time]  u8 beats, u8 beat_value
//!   track_count: u16 LE
//!
//! Track:  name(string) has_inst(u8) [inst(u8)] section_count(u16) [Section]*
//! Section: name(string) repeat(u8) measure_count(u16) [Measure]*
//! Measure: event_count(u16) [Event]*
//! Event:   tag(u8) [Note|Rest|Chord]
//!
//! Pitch:  note_name(u8) accidental(u8) has_octave(u8) [octave(u8)]
//! Key:    root(Pitch) scale_type(u8)
//! string: len(u16 LE) + UTF-8 bytes
//! ```

use sonus::{
    Accidental, AlterType, Chord, ChordAlterItem, ChordQuality, ChordSymbol,
    Duration, InstrumentKind, Key, Measure, MeasureEvent, Note, NoteKind,
    NoteName, Pitch, ScaleType, Score, Section, Tempo, TimeSig, Track,
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
    ALL_SCALE_TYPES.iter().position(|&s| s == st).unwrap() as u8
}

fn u8_to_scale(v: u8) -> ScaleType {
    ALL_SCALE_TYPES[(v as usize) % 30]
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

fn u8_to_acc(v: u8) -> Accidental {
    match v % 5 {
        0 => Accidental::Natural,
        1 => Accidental::Sharp,
        2 => Accidental::DoubleSharp,
        3 => Accidental::Flat,
        _ => Accidental::DoubleFlat,
    }
}

fn quality_to_u8(q: ChordQuality) -> u8 {
    match q {
        ChordQuality::Maj => 0,
        ChordQuality::Min => 1,
        ChordQuality::Dim => 2,
        ChordQuality::Aug => 3,
        ChordQuality::Sus2 => 4,
        ChordQuality::Sus4 => 5,
        ChordQuality::Power => 6,
    }
}

fn u8_to_quality(v: u8) -> ChordQuality {
    match v % 7 {
        0 => ChordQuality::Maj,
        1 => ChordQuality::Min,
        2 => ChordQuality::Dim,
        3 => ChordQuality::Aug,
        4 => ChordQuality::Sus2,
        5 => ChordQuality::Sus4,
        _ => ChordQuality::Power,
    }
}

fn alter_to_u8(a: AlterType) -> u8 {
    match a {
        AlterType::Sharp => 0,
        AlterType::Flat => 1,
        AlterType::Add => 2,
        AlterType::No => 3,
    }
}

fn u8_to_alter(v: u8) -> AlterType {
    match v % 4 {
        0 => AlterType::Sharp,
        1 => AlterType::Flat,
        2 => AlterType::Add,
        _ => AlterType::No,
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

// ── 二进制读取器 ──────────────────────────────────────────

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8, IrError> {
        if self.pos >= self.buf.len() {
            return Err(IrError::UnexpectedEof);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, IrError> {
        if self.pos + 2 > self.buf.len() {
            return Err(IrError::UnexpectedEof);
        }
        let v = u16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, IrError> {
        let len = self.u16()? as usize;
        if self.pos + len > self.buf.len() {
            return Err(IrError::UnexpectedEof);
        }
        let s = std::str::from_utf8(&self.buf[self.pos..self.pos + len])
            .map_err(|_| IrError::InvalidUtf8)?
            .to_string();
        self.pos += len;
        Ok(s)
    }

    fn pitch(&mut self) -> Result<Pitch, IrError> {
        let name = NoteName::from_index(self.u8()?);
        let acc = u8_to_acc(self.u8()?);
        let has_oct = self.u8()?;
        let octave = if has_oct != 0 {
            Some(self.u8()?)
        } else {
            None
        };
        Ok(Pitch::new(name, acc, octave))
    }

    fn key(&mut self) -> Result<Key, IrError> {
        let root = self.pitch()?;
        let st = u8_to_scale(self.u8()?);
        Ok(Key::new(root, st))
    }

    fn duration(&mut self) -> Result<Duration, IrError> {
        let base = self.u16()? as u32;
        let dotted = self.u8()? != 0;
        Ok(Duration::new(base, dotted))
    }
}

/// IR 错误。
#[derive(Debug, Clone)]
pub enum IrError {
    BadMagic,
    UnsupportedVersion(u16),
    UnexpectedEof,
    InvalidUtf8,
    BadEventTag(u8),
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "bad magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {}", v),
            Self::UnexpectedEof => write!(f, "unexpected end of data"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8"),
            Self::BadEventTag(t) => write!(f, "bad event tag: {}", t),
        }
    }
}

impl std::error::Error for IrError {}

// ── 公共 API ──────────────────────────────────────────────

/// 魔数。
const MAGIC: &[u8; 4] = b"BMIR";
/// 当前版本。
const VERSION: u16 = 1;

/// 将 Score 编码为 .bm 二进制。
pub fn encode(score: &Score) -> Vec<u8> {
    let mut w = Writer::new();

    // Header
    w.buf.extend_from_slice(MAGIC);
    w.u16(VERSION);

    // Flags
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
    w.u8(flags);

    // Optional fields
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

    // Tracks
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
                w.u8(1); // Rest
                w.duration(&note.duration);
            } else {
                w.u8(0); // Note
                if let NoteKind::Normal(pitch) = &note.kind {
                    w.pitch(pitch);
                }
                w.duration(&note.duration);
                w.u8(note.velocity());
            }
        }
        MeasureEvent::Chord(chord) => {
            w.u8(2); // Chord
            encode_chord(w, chord);
        }
        MeasureEvent::Control(_) => {
            // Cycle 0: 控制事件暂不编码
            w.u8(3);
        }
    }
}

fn encode_chord(w: &mut Writer, chord: &Chord) {
    // Root pitch
    w.pitch(&chord.symbol.root);

    // Quality
    match &chord.symbol.quality {
        Some(q) => {
            w.u8(1);
            w.u8(quality_to_u8(*q));
        }
        None => w.u8(0),
    }

    // Extension
    match chord.symbol.base_number {
        Some(n) => {
            w.u8(1);
            w.u8(n as u8);
        }
        None => w.u8(0),
    }

    // Major seventh
    w.u8(if chord.symbol.major_seventh { 1 } else { 0 });

    // Slash bass
    match &chord.slash_bass {
        Some(bass) => {
            w.u8(1);
            w.pitch(bass);
        }
        None => w.u8(0),
    }

    // Alters
    w.u8(chord.symbol.alters.len() as u8);
    for alter in &chord.symbol.alters {
        w.u8(alter_to_u8(alter.alter_type));
        w.u8(alter.number as u8);
    }

    // Duration & velocity
    w.duration(&chord.duration);
    w.u8(chord.velocity());
}

/// 从 .bm 二进制解码为 Score。
pub fn decode(bytes: &[u8]) -> Result<Score, IrError> {
    let mut r = Reader::new(bytes);

    // Magic
    if bytes.len() < 6 {
        return Err(IrError::UnexpectedEof);
    }
    if &bytes[0..4] != MAGIC {
        return Err(IrError::BadMagic);
    }
    r.pos = 4;

    let version = r.u16()?;
    if version != VERSION {
        return Err(IrError::UnsupportedVersion(version));
    }

    let flags = r.u8()?;
    let mut score = Score::empty();

    if flags & 1 != 0 {
        score.set_title(r.string()?);
    }
    if flags & 2 != 0 {
        score.set_global_key(r.key()?);
    }
    if flags & 4 != 0 {
        score.set_global_tempo(Tempo::new(r.u16()?));
    }
    if flags & 8 != 0 {
        let beats = r.u8()? as u32;
        let beat_value = r.u8()? as u32;
        score.set_global_time(TimeSig::new(beats, beat_value));
    }

    let track_count = r.u16()?;
    for i in 0..track_count as usize {
        let track = decode_track(&mut r, i)?;
        score.push_track(track);
    }

    Ok(score)
}

fn decode_track(r: &mut Reader, track_id: usize) -> Result<Track, IrError> {
    let name = r.string()?;
    let mut track = Track::new(name, track_id);

    let has_inst = r.u8()?;
    if has_inst != 0 {
        let idx = r.u8()?;
        track.set_instrument(InstrumentKind::from_index(idx));
    }

    let section_count = r.u16()?;
    for _ in 0..section_count {
        track.push_section(decode_section(r)?);
    }

    Ok(track)
}

fn decode_section(r: &mut Reader) -> Result<Section, IrError> {
    let name = r.string()?;
    let mut section = Section::new(name);

    let repeat = r.u8()?;
    if repeat > 0 {
        section.set_repeat(repeat as u32);
    }

    let measure_count = r.u16()?;
    for i in 0..measure_count as u32 {
        section.push_measure(decode_measure(r, i)?);
    }

    Ok(section)
}

fn decode_measure(r: &mut Reader, index: u32) -> Result<Measure, IrError> {
    let mut measure = Measure::new(index);
    let event_count = r.u16()?;
    for _ in 0..event_count {
        let tag = r.u8()?;
        match tag {
            0 => {
                // Note
                let pitch = r.pitch()?;
                let duration = r.duration()?;
                let velocity = r.u8()?;
                let mut note = Note::new_note(pitch, duration, 0);
                note.set_velocity(velocity);
                measure.push_event(MeasureEvent::Note(note));
            }
            1 => {
                // Rest
                let duration = r.duration()?;
                measure.push_event(MeasureEvent::Note(Note::new_rest(duration, 0)));
            }
            2 => {
                // Chord
                let chord = decode_chord(r)?;
                measure.push_event(MeasureEvent::Chord(chord));
            }
            3 => {
                // Control — Cycle 0 跳过
            }
            _ => return Err(IrError::BadEventTag(tag)),
        }
    }
    Ok(measure)
}

fn decode_chord(r: &mut Reader) -> Result<Chord, IrError> {
    let root = r.pitch()?;

    let has_quality = r.u8()?;
    let quality = if has_quality != 0 {
        Some(u8_to_quality(r.u8()?))
    } else {
        None
    };

    let has_extension = r.u8()?;
    let extension = if has_extension != 0 {
        Some(r.u8()? as u32)
    } else {
        None
    };

    let major_seventh = r.u8()? != 0;

    let has_slash = r.u8()?;
    let slash_bass = if has_slash != 0 {
        Some(r.pitch()?)
    } else {
        None
    };

    let alter_count = r.u8()?;
    let mut alters = Vec::with_capacity(alter_count as usize);
    for _ in 0..alter_count {
        let alter_type = u8_to_alter(r.u8()?);
        let number = r.u8()? as u32;
        alters.push(ChordAlterItem { alter_type, number });
    }

    let duration = r.duration()?;
    let velocity = r.u8()?;

    let mut symbol = ChordSymbol::new(root);
    if let Some(q) = quality {
        symbol = symbol.with_quality(q);
    }
    if let Some(ext) = extension {
        symbol = symbol.with_extension(ext, major_seventh);
    }
    symbol.alters = alters;

    let mut chord = match slash_bass {
        Some(bass) => Chord::new_slash(symbol, bass, duration, 0),
        None => Chord::new_normal(symbol, duration, 0),
    };
    chord.set_velocity(velocity);

    Ok(chord)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonus::{NoteName, Accidental, Pitch};

    fn roundtrip(score: &Score) -> Score {
        let bytes = encode(score);
        decode(&bytes).unwrap()
    }

    #[test]
    fn test_empty_score() {
        let score = Score::empty();
        let decoded = roundtrip(&score);
        assert_eq!(decoded.title, None);
        assert_eq!(decoded.tracks.len(), 0);
    }

    #[test]
    fn test_headers() {
        let mut score = Score::empty();
        score.set_title("Test Song".into());
        score.set_global_key(Key::major(Pitch::new(NoteName::C, Accidental::Natural, None)));
        score.set_global_tempo(Tempo::new(140));
        score.set_global_time(TimeSig::new(3, 4));

        let decoded = roundtrip(&score);
        assert_eq!(decoded.title, Some("Test Song".into()));
        assert!(decoded.global_key.is_some());
        assert_eq!(decoded.global_bpm(), 140);
        assert_eq!(decoded.global_time.unwrap().beats_per_bar, 3);
    }

    #[test]
    fn test_notes() {
        let mut score = Score::empty();
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
            Pitch::new(NoteName::F, Accidental::Sharp, Some(5)),
            Duration::dotted_eighth(),
            0,
        )));
        m.push_event(MeasureEvent::Note(Note::new_rest(Duration::half(), 0)));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let decoded = roundtrip(&score);
        assert_eq!(decoded.tracks.len(), 1);
        let t = &decoded.tracks[0];
        assert_eq!(t.name, "piano");
        assert!(t.instrument.is_some());
        assert_eq!(t.sections[0].measures[0].events.len(), 3);

        // Check first note
        if let MeasureEvent::Note(n) = &t.sections[0].measures[0].events[0] {
            assert_eq!(n.pitch().unwrap().name, NoteName::C);
            assert_eq!(n.pitch().unwrap().octave, Some(4));
            assert_eq!(n.duration, Duration::quarter());
        } else {
            panic!("expected note");
        }

        // Check second note (F#5 dotted eighth)
        if let MeasureEvent::Note(n) = &t.sections[0].measures[0].events[1] {
            assert_eq!(n.pitch().unwrap().name, NoteName::F);
            assert_eq!(n.pitch().unwrap().acc, Accidental::Sharp);
            assert_eq!(n.pitch().unwrap().octave, Some(5));
            assert!(n.duration.dotted);
            assert_eq!(n.duration.base, 8);
        } else {
            panic!("expected note");
        }

        // Check rest
        if let MeasureEvent::Note(n) = &t.sections[0].measures[0].events[2] {
            assert!(n.is_rest());
            assert_eq!(n.duration, Duration::half());
        } else {
            panic!("expected rest");
        }
    }

    #[test]
    fn test_chord() {
        let mut score = Score::empty();
        let mut track = Track::new("guitar".into(), 0);
        let mut section = Section::new("V".into());

        let sym = ChordSymbol::new(Pitch::new(NoteName::C, Accidental::Natural, None))
            .with_quality(ChordQuality::Maj)
            .with_extension(7, true);
        let chord = Chord::new_normal(sym, Duration::quarter(), 0);
        let mut m = Measure::new(0);
        m.push_event(MeasureEvent::Chord(chord));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let decoded = roundtrip(&score);
        let t = &decoded.tracks[0];
        if let MeasureEvent::Chord(c) = &t.sections[0].measures[0].events[0] {
            assert_eq!(c.symbol.root.name, NoteName::C);
            assert_eq!(c.symbol.quality, Some(ChordQuality::Maj));
            assert_eq!(c.symbol.base_number, Some(7));
            assert!(c.symbol.major_seventh);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_section_repeat() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("B".into());
        section.set_repeat(3);
        section.push_measure(Measure::new(0));
        track.push_section(section);
        score.push_track(track);

        let decoded = roundtrip(&score);
        assert_eq!(decoded.tracks[0].sections[0].repeat_times, Some(3));
    }

    #[test]
    fn test_bad_magic() {
        let result = decode(b"XXXX\x01\x00\x00");
        assert!(matches!(result, Err(IrError::BadMagic)));
    }

    #[test]
    fn test_slash_chord_roundtrip() {
        let mut score = Score::empty();
        let mut track = Track::new("t".into(), 0);
        let mut section = Section::new("S".into());

        let sym = ChordSymbol::new(Pitch::new(NoteName::D, Accidental::Natural, None))
            .with_quality(ChordQuality::Min)
            .with_extension(7, false);
        let chord = Chord::new_slash(
            sym,
            Pitch::new(NoteName::F, Accidental::Natural, None),
            Duration::half(),
            0,
        );
        let mut m = Measure::new(0);
        m.push_event(MeasureEvent::Chord(chord));
        section.push_measure(m);
        track.push_section(section);
        score.push_track(track);

        let decoded = roundtrip(&score);
        if let MeasureEvent::Chord(c) = &decoded.tracks[0].sections[0].measures[0].events[0] {
            assert!(c.slash_bass.is_some());
            assert_eq!(c.slash_bass.unwrap().name, NoteName::F);
            assert_eq!(c.symbol.quality, Some(ChordQuality::Min));
        } else {
            panic!("expected chord");
        }
    }
}
