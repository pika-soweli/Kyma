//! 递归下降语法分析器 — Token 流 → sonus::Score（新语法 v2）。
//!
//! ## 语法
//!
//! ```text
//! score     := header* track*
//! header    := '@' ident '(' header_value ')'
//! header_value := string                    ; @title("...")
//!             | pitch ',' scale_type        ; @key(C, major)
//!             | int                         ; @tempo(120)
//!             | int '/' int                 ; @time(4/4)
//!             | int                         ; @dur(4)
//! track     := 'track' string ident? '{' section* '}'
//! section   := 'section' string repeat? '{' measure* '}'
//! repeat    := 'repeat' '(' int ')'
//! measure   := event* ('|' event*)*
//! event     := note | rest | chord | tie
//! note      := pitch duration?
//! rest      := 'R' duration?
//! chord     := '[' pitch chord_desc? ('/' pitch)? ']' duration?
//! tie       := event '~'                    ; 连音线（Cycle 0: 消费 token）
//! duration  := ':N' | ':N.'
//! ```
//!
//! 若音符/和弦/休止符不带 `duration`，使用 `@dur` 设置的默认时值
//!（未设置时默认为四分音符 `-4`）。

use crate::lexer::CompileError;
use crate::token::*;
use sonus::{
    Accidental, Chord, ChordAlterItem, ChordQuality, ChordSymbol, AlterType,
    Duration, InstrumentKind, Key, Measure, MeasureEvent, Note, NoteName,
    Pitch, ScaleType, Score, Section, Tempo, TimeSig, Track,
};

/// 全部 30 种音阶类型，用于字符串 ↔ 枚举转换。
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

fn scale_type_from_str(s: &str) -> Option<ScaleType> {
    ALL_SCALE_TYPES.iter().find(|&&st| st.as_str() == s).copied()
}

// ── Parser ────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// 默认时值（由 @dur 设置，未设置时为四分音符）。
    default_duration: Duration,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            default_duration: Duration::quarter(),
        }
    }

    pub fn parse(mut self) -> Result<Score, CompileError> {
        let mut score = Score::empty();
        let mut track_count = 0usize;

        loop {
            match &self.peek().kind {
                TokenKind::Eof => break,
                TokenKind::At => self.parse_header(&mut score)?,
                TokenKind::Track => {
                    let track = self.parse_track(track_count)?;
                    score.push_track(track);
                    track_count += 1;
                }
                _ => return Err(self.err("expected '@header' or 'track'")),
            }
        }
        Ok(score)
    }

    // ── Token 操作 ──

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn replace_current(&mut self, kind: TokenKind) {
        if self.pos < self.tokens.len() {
            let line = self.tokens[self.pos].line;
            let col = self.tokens[self.pos].col;
            self.tokens[self.pos] = Token::new(kind, line, col);
        }
    }

    fn err(&self, msg: &str) -> CompileError {
        let t = self.peek();
        CompileError {
            message: msg.to_string(),
            line: t.line,
            col: t.col,
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<(), CompileError> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!(
                "expected {:?}, found {:?}",
                kind, self.peek().kind
            )))
        }
    }

    fn expect_int(&mut self) -> Result<u32, CompileError> {
        match &self.peek().kind {
            TokenKind::IntLit(n) => {
                let n = *n;
                self.advance();
                Ok(n)
            }
            _ => Err(self.err("expected integer")),
        }
    }

    // ── @ Headers ──

    fn parse_header(&mut self, score: &mut Score) -> Result<(), CompileError> {
        self.advance(); // '@'

        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.err("expected header name after '@'")),
        };
        self.advance();

        self.expect(&TokenKind::LParen)?;

        match name.as_str() {
            "title" => {
                match &self.peek().kind {
                    TokenKind::StringLit(s) => {
                        score.set_title(s.clone());
                        self.advance();
                    }
                    _ => return Err(self.err("expected string in @title()")),
                }
            }
            "key" => {
                let root = self.parse_pitch()?;
                self.expect(&TokenKind::Comma)?;
                let scale_type = self.parse_scale_type()?;
                score.set_global_key(Key::new(root, scale_type));
            }
            "tempo" => {
                let bpm = self.expect_int()? as u16;
                score.set_global_tempo(Tempo::new(bpm));
            }
            "time" => {
                let beats = self.expect_int()?;
                self.expect(&TokenKind::Slash)?;
                let beat_value = self.expect_int()?;
                score.set_global_time(TimeSig::new(beats, beat_value));
            }
            "dur" => {
                let base = self.expect_int()?;
                self.default_duration = Duration::new(base, false);
            }
            _ => return Err(self.err(&format!("unknown header: @{}", name))),
        }

        self.expect(&TokenKind::RParen)?;
        Ok(())
    }

    // ── Track ──

    fn parse_track(&mut self, track_id: usize) -> Result<Track, CompileError> {
        self.advance(); // 'track'

        let name = match &self.peek().kind {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(self.err("expected track name string")),
        };

        let mut track = Track::new(name, track_id);

        // 可选乐器名
        if let TokenKind::Ident(s) = &self.peek().kind {
            if let Some(inst) = InstrumentKind::from_str(s) {
                track.set_instrument(inst);
                self.advance();
            }
        }

        self.expect(&TokenKind::LBrace)?;

        loop {
            match &self.peek().kind {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Section => {
                    let section = self.parse_section()?;
                    track.push_section(section);
                }
                _ => return Err(self.err("expected 'section' or '}'")),
            }
        }

        Ok(track)
    }

    // ── Section ──

    fn parse_section(&mut self) -> Result<Section, CompileError> {
        self.advance(); // 'section'

        let name = match &self.peek().kind {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(self.err("expected section name string")),
        };

        let mut section = Section::new(name);

        // 可选 repeat(X)
        if matches!(self.peek().kind, TokenKind::Repeat) {
            self.advance(); // 'repeat'
            self.expect(&TokenKind::LParen)?;
            let times = match &self.peek().kind {
                TokenKind::IntLit(n) => {
                    let n = *n;
                    self.advance();
                    n
                }
                _ => return Err(self.err("expected repeat count integer")),
            };
            self.expect(&TokenKind::RParen)?;
            section.set_repeat(times);
        }

        self.expect(&TokenKind::LBrace)?;

        let mut measure_idx: u32 = 0;
        let mut current_measure = Measure::new(measure_idx);

        loop {
            match &self.peek().kind {
                TokenKind::RBrace => {
                    if !current_measure.events.is_empty() {
                        section.push_measure(current_measure);
                    }
                    self.advance();
                    break;
                }
                TokenKind::Pipe => {
                    section.push_measure(current_measure);
                    measure_idx += 1;
                    current_measure = Measure::new(measure_idx);
                    self.advance();
                }
                TokenKind::Eof => {
                    return Err(self.err("unexpected EOF in section"));
                }
                _ => {
                    let event = self.parse_event()?;
                    current_measure.push_event(event);
                }
            }
        }

        Ok(section)
    }

    // ── Event ──

    fn parse_event(&mut self) -> Result<MeasureEvent, CompileError> {
        match &self.peek().kind {
            TokenKind::Rest => {
                self.advance();
                let duration = self.parse_optional_duration();
                Ok(MeasureEvent::Note(Note::new_rest(duration, 0)))
            }
            TokenKind::LBracket => {
                let chord = self.parse_chord()?;
                Ok(MeasureEvent::Chord(chord))
            }
            TokenKind::NoteName(_) | TokenKind::Ident(_) => {
                let note = self.parse_note()?;
                // 连音线：消耗 ~ 及后续同音高音符（Cycle 0: 合并时值）
                let mut note = note;
                while matches!(self.peek().kind, TokenKind::Tilde) {
                    self.advance(); // '~'
                    let next = self.parse_note()?;
                    // 简单连音：合并时值
                    if let (Some(p1), Some(p2)) = (note.pitch(), next.pitch()) {
                        if p1 == p2 {
                            note.duration = Duration::new(
                                note.duration.base,
                                note.duration.dotted || next.duration.dotted,
                            );
                            // 连音不产生新事件，仅延长时值
                        }
                    }
                }
                Ok(MeasureEvent::Note(note))
            }
            _ => Err(self.err("expected note, rest, or chord")),
        }
    }

    fn parse_note(&mut self) -> Result<Note, CompileError> {
        let pitch = self.parse_pitch()?;
        let duration = self.parse_optional_duration();
        Ok(Note::new_note(pitch, duration, 0))
    }

    // ── Pitch ──

    fn parse_pitch(&mut self) -> Result<Pitch, CompileError> {
        let (name, acc_from_split) = match &self.peek().kind.clone() {
            TokenKind::NoteName(n) => {
                self.advance();
                (*n, None)
            }
            TokenKind::Ident(s) if is_pitch_start(s) => {
                let (name, acc) = self.split_pitch_ident(s)?;
                (name, Some(acc))
            }
            _ => return Err(self.err("expected pitch")),
        };

        let acc = match (acc_from_split, &self.peek().kind) {
            (Some(a), _) => a,
            (None, TokenKind::Accidental(a)) => {
                let a = *a;
                self.advance();
                a
            }
            (None, _) => Accidental::Natural,
        };

        let octave = match &self.peek().kind {
            TokenKind::IntLit(n) if *n < 10 => {
                let n = *n as u8;
                self.advance();
                Some(n)
            }
            _ => None,
        };

        Ok(Pitch::new(name, acc, octave))
    }

    fn split_pitch_ident(&mut self, s: &str) -> Result<(NoteName, Accidental), CompileError> {
        let first = s.chars().next().unwrap();
        let name = NoteName::from_char(first)
            .ok_or_else(|| self.err("invalid pitch"))?;
        let rest = &s[first.len_utf8()..];

        let (acc, remaining) = if rest.starts_with("bb") {
            (Accidental::DoubleFlat, &rest[2..])
        } else if rest.starts_with('b') {
            (Accidental::Flat, &rest[1..])
        } else if rest.starts_with('x') {
            (Accidental::DoubleSharp, &rest[1..])
        } else {
            (Accidental::Natural, rest)
        };

        if remaining.is_empty() {
            self.advance();
        } else {
            self.replace_current(TokenKind::Ident(remaining.to_string()));
        }

        Ok((name, acc))
    }

    // ── Duration ──

    /// 解析可选的时值标记 `:N` 或 `:N.`。
    /// 若不存在，返回默认时值。
    fn parse_optional_duration(&mut self) -> Duration {
        match &self.peek().kind {
            TokenKind::Duration { base, dotted } => {
                let d = Duration::new(*base, *dotted);
                self.advance();
                d
            }
            _ => self.default_duration,
        }
    }

    // ── Chord ──

    fn parse_chord(&mut self) -> Result<Chord, CompileError> {
        self.expect(&TokenKind::LBracket)?;

        let root = self.parse_pitch()?;

        let mut quality: Option<ChordQuality> = None;
        let mut extension: Option<u32> = None;
        let mut major_seventh = false;
        let mut alters: Vec<ChordAlterItem> = Vec::new();

        loop {
            match &self.peek().kind.clone() {
                TokenKind::RBracket | TokenKind::Slash => break,

                TokenKind::Ident(s) => {
                    let s = s.clone();

                    if s == "maj" || s == "major" {
                        self.advance();
                        if let TokenKind::IntLit(n) = &self.peek().kind {
                            if [6u32, 7, 9, 11, 13].contains(n) {
                                major_seventh = true;
                                extension = Some(*n);
                                quality = Some(ChordQuality::Maj);
                                self.advance();
                                continue;
                            }
                        }
                        quality = Some(ChordQuality::Maj);
                        continue;
                    }

                    if let Some(q) = ChordQuality::from_str(&s) {
                        quality = Some(q);
                        self.advance();
                        if q == ChordQuality::Sus4 {
                            if let TokenKind::IntLit(n) = &self.peek().kind {
                                if *n == 2 {
                                    quality = Some(ChordQuality::Sus2);
                                    self.advance();
                                } else if *n == 4 {
                                    self.advance();
                                }
                            }
                        }
                        continue;
                    }

                    if s == "add" {
                        self.advance();
                        let n = self.expect_int()?;
                        alters.push(ChordAlterItem {
                            alter_type: AlterType::Add,
                            number: n,
                        });
                        continue;
                    }

                    if s == "no" {
                        self.advance();
                        let n = self.expect_int()?;
                        alters.push(ChordAlterItem {
                            alter_type: AlterType::No,
                            number: n,
                        });
                        continue;
                    }

                    return Err(self.err(&format!("unexpected identifier in chord: {}", s)));
                }

                TokenKind::IntLit(n) => {
                    let n = *n;
                    if n == 5 && quality.is_none() && extension.is_none() {
                        quality = Some(ChordQuality::Power);
                        self.advance();
                    } else if [6u32, 7, 9, 11, 13].contains(&n) {
                        extension = Some(n);
                        self.advance();
                    } else {
                        return Err(self.err(&format!("unexpected number in chord: {}", n)));
                    }
                }

                TokenKind::Accidental(acc) => {
                    let acc = *acc;
                    self.advance();
                    let n = self.expect_int()?;
                    let alter_type = match acc {
                        Accidental::Sharp | Accidental::DoubleSharp => AlterType::Sharp,
                        Accidental::Flat | Accidental::DoubleFlat => AlterType::Flat,
                        _ => return Err(self.err("invalid accidental in chord alter")),
                    };
                    alters.push(ChordAlterItem {
                        alter_type,
                        number: n,
                    });
                }

                _ => return Err(self.err("unexpected token in chord descriptor")),
            }
        }

        // Slash bass
        let slash_bass = if matches!(self.peek().kind, TokenKind::Slash) {
            self.advance();
            Some(self.parse_pitch()?)
        } else {
            None
        };

        self.expect(&TokenKind::RBracket)?;

        // 和弦时值：可选，默认取 @dur
        let duration = self.parse_optional_duration();

        let mut symbol = ChordSymbol::new(root);
        if let Some(q) = quality {
            symbol = symbol.with_quality(q);
        }
        if let Some(ext) = extension {
            symbol = symbol.with_extension(ext, major_seventh);
        }
        symbol.alters = alters;

        let chord = match slash_bass {
            Some(bass) => Chord::new_slash(symbol, bass, duration, 0),
            None => Chord::new_normal(symbol, duration, 0),
        };

        Ok(chord)
    }

    // ── Scale Type ──

    fn parse_scale_type(&mut self) -> Result<ScaleType, CompileError> {
        match &self.peek().kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                if let Some(st) = scale_type_from_str(&s) {
                    self.advance();
                    Ok(st)
                } else {
                    Err(self.err(&format!("unknown scale type: {}", s)))
                }
            }
            _ => Err(self.err("expected scale type identifier")),
        }
    }
}

// ── 辅助 ──────────────────────────────────────────────────

fn is_pitch_start(s: &str) -> bool {
    s.chars()
        .next()
        .map(|c| "CDEFGAB".contains(c))
        .unwrap_or(false)
}

static EOF_TOKEN: Token = Token {
    kind: TokenKind::Eof,
    line: 0,
    col: 0,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Score {
        let tokens = Lexer::new(input).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    fn parse_err(input: &str) -> CompileError {
        let tokens = match Lexer::new(input).tokenize() {
            Ok(t) => t,
            Err(e) => return e,
        };
        Parser::new(tokens).parse().unwrap_err()
    }

    use crate::lexer::Lexer;

    // ── 头部测试 ──

    #[test]
    fn test_headers() {
        let score = parse("@title(\"Test\")\n@key(C, major)\n@tempo(120)\n@time(4/4)");
        assert_eq!(score.title, Some("Test".into()));
        assert!(score.global_key.is_some());
        assert_eq!(score.global_bpm(), 120);
        assert!(score.global_time.is_some());
    }

    #[test]
    fn test_dur_directive() {
        let score = parse("@dur(2)\ntrack \"t\" {\nsection \"s\" {\nC4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert_eq!(n.duration, Duration::half());
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn test_key_with_flat() {
        let score = parse("@key(Bb, major)");
        let key = score.global_key.unwrap();
        assert_eq!(key.root.name, NoteName::B);
        assert_eq!(key.root.acc, Accidental::Flat);
    }

    #[test]
    fn test_key_with_sharp() {
        let score = parse("@key(F#, minor)");
        let key = score.global_key.unwrap();
        assert_eq!(key.root.name, NoteName::F);
        assert_eq!(key.root.acc, Accidental::Sharp);
        assert_eq!(key.scale_type, ScaleType::Minor);
    }

    // ── 默认时值测试 ──

    #[test]
    fn test_default_duration_quarter() {
        let score = parse("track \"t\" {\nsection \"s\" {\nC4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert_eq!(n.duration, Duration::quarter());
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn test_duration_override() {
        let score = parse("track \"t\" {\nsection \"s\" {\nC4:2 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert_eq!(n.duration, Duration::half());
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn test_dotted_duration() {
        let score = parse("track \"t\" {\nsection \"s\" {\nC4:4. |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert!(n.duration.dotted);
            assert_eq!(n.duration.base, 4);
        } else {
            panic!("expected note");
        }
    }

    // ── 音轨/段落测试 ──

    #[test]
    fn test_track_with_notes() {
        let score = parse("track \"piano\" piano {\nsection \"A\" {\nC4 E4 G4 C5 |\n}\n}");
        assert_eq!(score.tracks.len(), 1);
        assert_eq!(score.tracks[0].name, "piano");
        assert!(score.tracks[0].instrument.is_some());
        assert_eq!(score.tracks[0].sections.len(), 1);
        assert_eq!(score.tracks[0].sections[0].measures.len(), 1);
        assert_eq!(score.tracks[0].sections[0].measures[0].events.len(), 4);
    }

    #[test]
    fn test_rest() {
        let score = parse("track \"t\" {\nsection \"s\" {\nR |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        assert_eq!(m.events.len(), 1);
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert!(n.is_rest());
        } else {
            panic!("expected rest");
        }
    }

    #[test]
    fn test_rest_with_duration() {
        let score = parse("track \"t\" {\nsection \"s\" {\nR:1 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert!(n.is_rest());
            assert_eq!(n.duration, Duration::whole());
        } else {
            panic!("expected rest");
        }
    }

    // ── 和弦测试 ──

    #[test]
    fn test_chord_basic() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C maj 7] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.root.name, NoteName::C);
            assert!(c.symbol.quality.is_some());
            assert_eq!(c.symbol.base_number, Some(7));
            assert!(c.symbol.major_seventh);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_compact_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[Cmaj7] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.root.name, NoteName::C);
            assert_eq!(c.symbol.base_number, Some(7));
            assert!(c.symbol.major_seventh);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_minor_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[Dm 7] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.root.name, NoteName::D);
            assert_eq!(c.symbol.quality, Some(ChordQuality::Min));
            assert_eq!(c.symbol.base_number, Some(7));
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_slash_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C maj 7 / G] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert!(c.slash_bass.is_some());
            assert_eq!(c.slash_bass.unwrap().name, NoteName::G);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_chord_alter() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C 7 #5] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.alters.len(), 1);
            assert_eq!(c.symbol.alters[0].alter_type, AlterType::Sharp);
            assert_eq!(c.symbol.alters[0].number, 5);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_chord_with_duration() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C maj]:2 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.duration, Duration::half());
        } else {
            panic!("expected chord");
        }
    }

    // ── 连音线测试 ──

    #[test]
    fn test_tie() {
        let score = parse("track \"t\" {\nsection \"s\" {\nC4 ~ C4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        // 连音后应只有一个事件（合并时值）
        assert_eq!(m.events.len(), 1);
        if let MeasureEvent::Note(n) = &m.events[0] {
            assert_eq!(n.pitch().unwrap().name, NoteName::C);
        } else {
            panic!("expected note");
        }
    }

    // ── 其他测试 ──

    #[test]
    fn test_multiple_measures() {
        let score = parse("track \"t\" {\nsection \"s\" {\nC4 |\nE4 |\nG4 |\n}\n}");
        assert_eq!(score.tracks[0].sections[0].measures.len(), 3);
    }

    #[test]
    fn test_sus_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C sus4] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.quality, Some(ChordQuality::Sus4));
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_sus2_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C sus 2] |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.quality, Some(ChordQuality::Sus2));
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_error_bad_token() {
        // 新语法中 @ 是合法 token，用 $ 测试非法字符
        let e = parse_err("track \"t\" {\nsection \"s\" {\n$ |\n}\n}");
        assert!(e.message.contains("unexpected character"));
    }

    #[test]
    fn test_comment_semicolon() {
        let score = parse("; comment line\n@tempo(120)\ntrack \"t\" {\nsection \"s\" {\nC4 |\n}\n}");
        assert_eq!(score.global_bpm(), 120);
    }

    #[test]
    fn test_section_repeat() {
        let score = parse("track \"t\" {\nsection \"A\" repeat(3) {\nC4 |\nE4 |\n}\n}");
        assert_eq!(score.tracks[0].sections.len(), 1);
        assert_eq!(score.tracks[0].sections[0].repeat_times, Some(3));
        assert_eq!(score.tracks[0].sections[0].measures.len(), 2);
    }

    #[test]
    fn test_section_repeat_no_value() {
        let e = parse_err("track \"t\" {\nsection \"A\" repeat() {\nC4 |\n}\n}");
        assert!(e.message.contains("expected repeat count integer"));
    }

    #[test]
    fn test_section_repeat_with_non_int() {
        let e = parse_err("track \"t\" {\nsection \"A\" repeat(A) {\nC4 |\n}\n}");
        assert!(e.message.contains("expected repeat count integer"));
    }

    #[test]
    fn test_section_repeat_omitted() {
        let score = parse("track \"t\" {\nsection \"A\" {\nC4 |\n}\n}");
        assert_eq!(score.tracks[0].sections[0].repeat_times, None);
    }
}
