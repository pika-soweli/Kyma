//! 独立解析器 — 每种语法结构对应一个 struct，统一实现 [`ParseRule`] trait。

use super::{ParseContext, ParseRule, parse_scale_type};
use crate::lexer::CompileError;
use crate::token::*;
use sonus::{
    Accidental, AlterType, Chord, ChordAlterItem, ChordQuality, ChordSymbol,
    Duration, InstrumentKind, Key, LocalControl, Measure, MeasureEvent, Note,
    PedalKind, Pitch, Score, Section, Tempo, TimeSig, Track, Tuplet,
};

// ════════════════════════════════════════════════════════════
//  事件级解析器
// ════════════════════════════════════════════════════════════

/// 音符解析器 — `pitch duration?`，含连音线 `~` 合并。
pub struct NoteParser;

impl ParseRule<Note> for NoteParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Note, CompileError> {
        let pitch = ctx.parse_pitch()?;
        let duration = ctx.parse_optional_duration();
        let mut note = Note::new_note(pitch, duration, 0);

        while matches!(ctx.peek().kind, TokenKind::Tilde) {
            ctx.advance();
            let next = {
                let p = ctx.parse_pitch()?;
                let d = ctx.parse_optional_duration();
                Note::new_note(p, d, 0)
            };
            if let (Some(p1), Some(p2)) = (note.pitch(), next.pitch()) {
                if p1 == p2 {
                    note.duration = Duration::new(
                        note.duration.base,
                        note.duration.dotted || next.duration.dotted,
                    );
                }
            }
        }
        Ok(note)
    }
}

/// 休止符解析器 — `R duration?`。
pub struct RestParser;

impl ParseRule<Note> for RestParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Note, CompileError> {
        ctx.advance(); // 'R'
        let duration = ctx.parse_optional_duration();
        Ok(Note::new_rest(duration, 0))
    }
}

/// 和弦解析器 — `[ pitch chord_desc? ('/' pitch)? ] duration?`。
pub struct ChordParser;

impl ParseRule<Chord> for ChordParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Chord, CompileError> {
        ctx.expect(&TokenKind::LBracket)?;
        let root = ctx.parse_pitch()?;

        let mut quality: Option<ChordQuality> = None;
        let mut extension: Option<u32> = None;
        let mut major_seventh = false;
        let mut alters: Vec<ChordAlterItem> = Vec::new();

        loop {
            match &ctx.peek().kind.clone() {
                TokenKind::RBracket | TokenKind::Slash => break,

                TokenKind::Ident(s) => {
                    let s = s.clone();
                    if s == "maj" || s == "major" {
                        ctx.advance();
                        if let TokenKind::IntLit(n) = &ctx.peek().kind {
                            if [6u32, 7, 9, 11, 13].contains(n) {
                                major_seventh = true;
                                extension = Some(*n);
                                quality = Some(ChordQuality::Maj);
                                ctx.advance();
                                continue;
                            }
                        }
                        quality = Some(ChordQuality::Maj);
                        continue;
                    }
                    if let Some(q) = ChordQuality::from_str(&s) {
                        quality = Some(q);
                        ctx.advance();
                        if q == ChordQuality::Sus4 {
                            if let TokenKind::IntLit(n) = &ctx.peek().kind {
                                if *n == 2 {
                                    quality = Some(ChordQuality::Sus2);
                                    ctx.advance();
                                } else if *n == 4 {
                                    ctx.advance();
                                }
                            }
                        }
                        continue;
                    }
                    if s == "add" {
                        ctx.advance();
                        let n = ctx.expect_int()?;
                        alters.push(ChordAlterItem { alter_type: AlterType::Add, number: n });
                        continue;
                    }
                    if s == "no" {
                        ctx.advance();
                        let n = ctx.expect_int()?;
                        alters.push(ChordAlterItem { alter_type: AlterType::No, number: n });
                        continue;
                    }
                    return Err(ctx.err(&format!("unexpected identifier in chord: {}", s)));
                }

                TokenKind::IntLit(n) => {
                    let n = *n;
                    if n == 5 && quality.is_none() && extension.is_none() {
                        quality = Some(ChordQuality::Power);
                        ctx.advance();
                    } else if [6u32, 7, 9, 11, 13].contains(&n) {
                        extension = Some(n);
                        ctx.advance();
                    } else {
                        return Err(ctx.err(&format!("unexpected number in chord: {}", n)));
                    }
                }

                TokenKind::Accidental(acc) => {
                    let acc = *acc;
                    ctx.advance();
                    let n = ctx.expect_int()?;
                    let alter_type = match acc {
                        Accidental::Sharp | Accidental::DoubleSharp => AlterType::Sharp,
                        Accidental::Flat | Accidental::DoubleFlat => AlterType::Flat,
                        _ => return Err(ctx.err("invalid accidental in chord alter")),
                    };
                    alters.push(ChordAlterItem { alter_type, number: n });
                }

                _ => return Err(ctx.err("unexpected token in chord descriptor")),
            }
        }

        let mut slash_bass: Option<Pitch> = None;
        let mut inversion: u8 = 0;

        if matches!(ctx.peek().kind, TokenKind::Slash) {
            ctx.advance();
            match &ctx.peek().kind {
                TokenKind::IntLit(n) if [1u32, 2, 3].contains(n) => {
                    inversion = *n as u8;
                    ctx.advance();
                }
                _ => {
                    slash_bass = Some(ctx.parse_pitch()?);
                }
            }
        }

        ctx.expect(&TokenKind::RBracket)?;
        let duration = ctx.parse_optional_duration();

        let mut symbol = ChordSymbol::new(root);
        if let Some(q) = quality {
            symbol = symbol.with_quality(q);
        }
        if let Some(ext) = extension {
            symbol = symbol.with_extension(ext, major_seventh);
        }
        symbol.alters = alters;

        let chord = if inversion > 0 {
            Chord::new_with_inversion(symbol, inversion, duration, 0)
        } else if let Some(bass) = slash_bass {
            Chord::new_slash(symbol, bass, duration, 0)
        } else {
            Chord::new_normal(symbol, duration, 0)
        };
        Ok(chord)
    }
}

/// 装饰音解析器 — `grace(pitch duration?)`。
pub struct GraceParser;

impl ParseRule<Note> for GraceParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Note, CompileError> {
        ctx.advance(); // 'grace'
        ctx.expect(&TokenKind::LParen)?;
        let pitch = ctx.parse_pitch()?;
        let duration = ctx.parse_optional_duration();
        ctx.expect(&TokenKind::RParen)?;
        Ok(Note::new_note(pitch, duration, 0))
    }
}

/// 连音符解析器 — `N:M { event* }`。
pub struct TupletParser;

impl TupletParser {
    /// 前瞻判断当前是否为连音符开头：`IntLit Duration{..} LBrace`。
    pub fn is_tuplet_start(ctx: &ParseContext) -> bool {
        matches!(ctx.peek().kind, TokenKind::IntLit(_))
            && matches!(
                ctx.tokens.get(ctx.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Duration { dotted: false, .. })
            )
            && matches!(
                ctx.tokens.get(ctx.pos + 2).map(|t| &t.kind),
                Some(TokenKind::LBrace)
            )
    }
}

impl ParseRule<Tuplet> for TupletParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Tuplet, CompileError> {
        let num = ctx.expect_int()?;
        let den = match &ctx.peek().kind {
            TokenKind::Duration { base, dotted: false } => {
                let b = *base;
                ctx.advance();
                b
            }
            _ => return Err(ctx.err("expected ':N' ratio in tuplet")),
        };
        ctx.expect(&TokenKind::LBrace)?;

        let mut tuplet = Tuplet::new((num, den));
        let dispatcher = EventDispatcher;

        loop {
            match &ctx.peek().kind {
                TokenKind::RBrace => {
                    ctx.advance();
                    break;
                }
                TokenKind::Eof => return Err(ctx.err("unexpected EOF in tuplet")),
                _ => {
                    let event = dispatcher.try_parse(ctx)?;
                    tuplet.push_event(event);
                }
            }
        }
        Ok(tuplet)
    }
}

/// 局部控制事件解析器 — `@cresc` / `@key(...)` / `@tempo(...)` 等。
pub struct ControlParser;

impl ParseRule<LocalControl> for ControlParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<LocalControl, CompileError> {
        ctx.advance(); // '@'

        let name = match &ctx.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(ctx.err("expected control name after '@'")),
        };
        ctx.advance();

        match name.as_str() {
            "cresc" => return Ok(LocalControl::DynamicMark("cresc".into())),
            "decresc" => return Ok(LocalControl::DynamicMark("decresc".into())),
            "rit" => return Ok(LocalControl::DynamicMark("rit".into())),
            "accel" => return Ok(LocalControl::DynamicMark("accel".into())),
            "fermata" => return Ok(LocalControl::DynamicMark("fermata".into())),
            _ => {}
        }

        ctx.expect(&TokenKind::LParen)?;

        let ctrl = match name.as_str() {
            "key" => {
                let root = ctx.parse_pitch()?;
                ctx.expect(&TokenKind::Comma)?;
                let (scale_type, _direction) = parse_scale_type(ctx)?;
                LocalControl::LocalKey(Key::new(root, scale_type))
            }
            "tempo" => {
                let bpm = ctx.expect_int()? as u16;
                LocalControl::LocalTempo(Tempo::new(bpm))
            }
            "time" => {
                let beats = ctx.expect_int()?;
                ctx.expect(&TokenKind::Slash)?;
                let beat_value = ctx.expect_int()?;
                LocalControl::LocalTime(TimeSig::new(beats, beat_value))
            }
            "pedal" => {
                let kind_str = match &ctx.peek().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(ctx.err("expected pedal kind")),
                };
                ctx.advance();
                let kind = pedal_kind_from_str(&kind_str)
                    .ok_or_else(|| ctx.err(&format!("unknown pedal kind: {}", kind_str)))?;
                ctx.expect(&TokenKind::Comma)?;
                let state = match &ctx.peek().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(ctx.err("expected 'on' or 'off'")),
                };
                ctx.advance();
                match state.as_str() {
                    "on" => LocalControl::PedalOn(kind),
                    "off" => LocalControl::PedalOff(kind),
                    _ => return Err(ctx.err("expected 'on' or 'off'")),
                }
            }
            "dyn" => {
                let dyn_str = match &ctx.peek().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(ctx.err("expected dynamic mark")),
                };
                ctx.advance();
                LocalControl::DynamicMark(dyn_str)
            }
            "vol" => {
                let v = ctx.expect_int()? as u8;
                LocalControl::Volume(v)
            }
            _ => return Err(ctx.err(&format!("unknown control: @{}", name))),
        };

        ctx.expect(&TokenKind::RParen)?;
        Ok(ctrl)
    }
}

/// 事件分发器 — 根据当前 token 分派到对应的子解析器。
pub struct EventDispatcher;

impl ParseRule<MeasureEvent> for EventDispatcher {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<MeasureEvent, CompileError> {
        match &ctx.peek().kind {
            TokenKind::Rest => {
                let note = RestParser.try_parse(ctx)?;
                Ok(MeasureEvent::Note(note))
            }
            TokenKind::LBracket => {
                let chord = ChordParser.try_parse(ctx)?;
                Ok(MeasureEvent::Chord(chord))
            }
            TokenKind::NoteName(_) | TokenKind::Ident(_) => {
                let note = NoteParser.try_parse(ctx)?;
                Ok(MeasureEvent::Note(note))
            }
            TokenKind::At => {
                let ctrl = ControlParser.try_parse(ctx)?;
                Ok(MeasureEvent::Control(ctrl))
            }
            TokenKind::Grace => {
                let note = GraceParser.try_parse(ctx)?;
                Ok(MeasureEvent::Grace(note))
            }
            TokenKind::IntLit(_) => {
                if TupletParser::is_tuplet_start(ctx) {
                    let tuplet = TupletParser.try_parse(ctx)?;
                    Ok(MeasureEvent::Tuplet(tuplet))
                } else {
                    Err(ctx.err("expected note, rest, or chord"))
                }
            }
            _ => Err(ctx.err("expected note, rest, chord, @control, grace, or tuplet")),
        }
    }
}

// ════════════════════════════════════════════════════════════
//  结构级解析器
// ════════════════════════════════════════════════════════════

/// 头部指令解析器 — `@title("...")` / `@key(...)` / `@tempo(...)` 等。
pub struct HeaderParser;

impl HeaderParser {
    pub fn try_parse(ctx: &mut ParseContext, score: &mut Score) -> Result<(), CompileError> {
        ctx.advance(); // '@'

        let name = match &ctx.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(ctx.err("expected header name after '@'")),
        };
        ctx.advance();

        ctx.expect(&TokenKind::LParen)?;

        match name.as_str() {
            "title" => {
                match &ctx.peek().kind {
                    TokenKind::StringLit(s) => {
                        score.set_title(s.clone());
                        ctx.advance();
                    }
                    _ => return Err(ctx.err("expected string in @title()")),
                }
            }
            "key" => {
                let root = ctx.parse_pitch()?;
                ctx.expect(&TokenKind::Comma)?;
                let (scale_type, _direction) = parse_scale_type(ctx)?;
                score.set_global_key(Key::new(root, scale_type));
            }
            "tempo" => {
                let bpm = ctx.expect_int()? as u16;
                score.set_global_tempo(Tempo::new(bpm));
            }
            "time" => {
                let beats = ctx.expect_int()?;
                ctx.expect(&TokenKind::Slash)?;
                let beat_value = ctx.expect_int()?;
                score.set_global_time(TimeSig::new(beats, beat_value));
            }
            "dur" => {
                let base = ctx.expect_int()?;
                let d = Duration::new(base, false);
                ctx.default_duration = d;
                score.default_duration = d;
            }
            _ => return Err(ctx.err(&format!("unknown header: @{}", name))),
        }

        ctx.expect(&TokenKind::RParen)?;
        Ok(())
    }
}

/// 段落解析器 — `section "name" repeat(N)? { measure* }`。
pub struct SectionParser;

impl ParseRule<Section> for SectionParser {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<Section, CompileError> {
        ctx.advance(); // 'section'

        let name = match &ctx.peek().kind {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                ctx.advance();
                s
            }
            _ => return Err(ctx.err("expected section name string")),
        };

        let mut section = Section::new(name);

        if matches!(ctx.peek().kind, TokenKind::Repeat) {
            ctx.advance();
            ctx.expect(&TokenKind::LParen)?;
            let times = match &ctx.peek().kind {
                TokenKind::IntLit(n) => {
                    let n = *n;
                    ctx.advance();
                    n
                }
                _ => return Err(ctx.err("expected repeat count integer")),
            };
            ctx.expect(&TokenKind::RParen)?;
            section.set_repeat(times);
        }

        ctx.expect(&TokenKind::LBrace)?;

        let mut measure_idx: u32 = 0;
        let mut current_measure = Measure::new(measure_idx);
        let dispatcher = EventDispatcher;

        loop {
            match &ctx.peek().kind {
                TokenKind::RBrace => {
                    if !current_measure.events.is_empty() {
                        section.push_measure(current_measure);
                    }
                    ctx.advance();
                    break;
                }
                TokenKind::Pipe => {
                    section.push_measure(current_measure);
                    measure_idx += 1;
                    current_measure = Measure::new(measure_idx);
                    ctx.advance();
                }
                TokenKind::Eof => {
                    return Err(ctx.err("unexpected EOF in section"));
                }
                _ => {
                    let event = dispatcher.try_parse(ctx)?;
                    current_measure.push_event(event);
                }
            }
        }

        Ok(section)
    }
}

/// 音轨解析器 — `track "name" instrument? { section* }`。
pub struct TrackParser;

impl TrackParser {
    pub fn try_parse(ctx: &mut ParseContext, track_id: usize) -> Result<Track, CompileError> {
        ctx.advance(); // 'track'

        let name = match &ctx.peek().kind {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                ctx.advance();
                s
            }
            _ => return Err(ctx.err("expected track name string")),
        };

        let mut track = Track::new(name, track_id);

        if let TokenKind::Ident(s) = &ctx.peek().kind {
            if let Some(inst) = InstrumentKind::from_str(s) {
                track.set_instrument(inst);
                ctx.advance();
            }
        }

        ctx.expect(&TokenKind::LBrace)?;

        let section_parser = SectionParser;

        loop {
            match &ctx.peek().kind {
                TokenKind::RBrace => {
                    ctx.advance();
                    break;
                }
                TokenKind::Section => {
                    let section = section_parser.try_parse(ctx)?;
                    track.push_section(section);
                }
                _ => return Err(ctx.err("expected 'section' or '}'")),
            }
        }

        Ok(track)
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

fn pedal_kind_from_str(s: &str) -> Option<PedalKind> {
    match s {
        "sustain" => Some(PedalKind::Sustain),
        "soft" => Some(PedalKind::Soft),
        "sostenuto" => Some(PedalKind::Sostenuto),
        _ => None,
    }
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ParseContext;

    fn make_ctx(tokens: Vec<Token>) -> ParseContext {
        ParseContext::new(tokens)
    }

    #[test]
    fn test_note_parser_basic() {
        let tokens = vec![
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 1),
            Token::new(TokenKind::IntLit(4), 1, 2),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let note = NoteParser.try_parse(&mut ctx).unwrap();
        assert_eq!(note.pitch().unwrap().name, sonus::NoteName::C);
        assert_eq!(note.pitch().unwrap().octave, Some(4));
        assert_eq!(note.duration, Duration::quarter());
    }

    #[test]
    fn test_note_parser_with_duration() {
        let tokens = vec![
            Token::new(TokenKind::NoteName(sonus::NoteName::E), 1, 1),
            Token::new(TokenKind::IntLit(4), 1, 2),
            Token::new(TokenKind::Duration { base: 8, dotted: false }, 1, 3),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let note = NoteParser.try_parse(&mut ctx).unwrap();
        assert_eq!(note.duration, Duration::eighth());
    }

    #[test]
    fn test_rest_parser() {
        let tokens = vec![
            Token::new(TokenKind::Rest, 1, 1),
            Token::new(TokenKind::Duration { base: 2, dotted: false }, 1, 2),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let note = RestParser.try_parse(&mut ctx).unwrap();
        assert!(note.is_rest());
        assert_eq!(note.duration, Duration::half());
    }

    #[test]
    fn test_chord_parser_basic() {
        let tokens = vec![
            Token::new(TokenKind::LBracket, 1, 1),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 2),
            Token::new(TokenKind::Ident("maj".into()), 1, 4),
            Token::new(TokenKind::IntLit(7), 1, 8),
            Token::new(TokenKind::RBracket, 1, 9),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let chord = ChordParser.try_parse(&mut ctx).unwrap();
        assert_eq!(chord.symbol.root.name, sonus::NoteName::C);
        assert_eq!(chord.symbol.quality, Some(ChordQuality::Maj));
        assert_eq!(chord.symbol.base_number, Some(7));
        assert!(chord.symbol.major_seventh);
    }

    #[test]
    fn test_grace_parser() {
        let tokens = vec![
            Token::new(TokenKind::Grace, 1, 1),
            Token::new(TokenKind::LParen, 1, 6),
            Token::new(TokenKind::NoteName(sonus::NoteName::D), 1, 7),
            Token::new(TokenKind::IntLit(5), 1, 8),
            Token::new(TokenKind::RParen, 1, 9),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let note = GraceParser.try_parse(&mut ctx).unwrap();
        assert_eq!(note.pitch().unwrap().name, sonus::NoteName::D);
        assert_eq!(note.pitch().unwrap().octave, Some(5));
    }

    #[test]
    fn test_tuplet_parser() {
        let tokens = vec![
            Token::new(TokenKind::IntLit(3), 1, 1),
            Token::new(TokenKind::Duration { base: 2, dotted: false }, 1, 2),
            Token::new(TokenKind::LBrace, 1, 5),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 7),
            Token::new(TokenKind::IntLit(4), 1, 8),
            Token::new(TokenKind::RBrace, 1, 10),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let tuplet = TupletParser.try_parse(&mut ctx).unwrap();
        assert_eq!(tuplet.ratio, (3, 2));
        assert_eq!(tuplet.events.len(), 1);
    }

    #[test]
    fn test_control_parser_expression() {
        let tokens = vec![
            Token::new(TokenKind::At, 1, 1),
            Token::new(TokenKind::Ident("cresc".into()), 1, 2),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let ctrl = ControlParser.try_parse(&mut ctx).unwrap();
        assert!(matches!(ctrl, LocalControl::DynamicMark(ref s) if s == "cresc"));
    }

    #[test]
    fn test_control_parser_tempo() {
        let tokens = vec![
            Token::new(TokenKind::At, 1, 1),
            Token::new(TokenKind::Ident("tempo".into()), 1, 2),
            Token::new(TokenKind::LParen, 1, 7),
            Token::new(TokenKind::IntLit(90), 1, 8),
            Token::new(TokenKind::RParen, 1, 10),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let ctrl = ControlParser.try_parse(&mut ctx).unwrap();
        if let LocalControl::LocalTempo(t) = ctrl {
            assert_eq!(t.bpm(), 90);
        } else {
            panic!("expected LocalTempo");
        }
    }

    #[test]
    fn test_event_dispatcher_note() {
        let tokens = vec![
            Token::new(TokenKind::NoteName(sonus::NoteName::G), 1, 1),
            Token::new(TokenKind::IntLit(4), 1, 2),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let event = EventDispatcher.try_parse(&mut ctx).unwrap();
        assert!(matches!(event, MeasureEvent::Note(_)));
    }

    #[test]
    fn test_event_dispatcher_rest() {
        let tokens = vec![
            Token::new(TokenKind::Rest, 1, 1),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let event = EventDispatcher.try_parse(&mut ctx).unwrap();
        assert!(matches!(event, MeasureEvent::Note(_)));
    }

    #[test]
    fn test_section_parser() {
        let tokens = vec![
            Token::new(TokenKind::Section, 1, 1),
            Token::new(TokenKind::StringLit("A".into()), 1, 9),
            Token::new(TokenKind::LBrace, 1, 12),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 2, 1),
            Token::new(TokenKind::IntLit(4), 2, 2),
            Token::new(TokenKind::Pipe, 2, 4),
            Token::new(TokenKind::RBrace, 3, 1),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let section = SectionParser.try_parse(&mut ctx).unwrap();
        assert_eq!(section.name, "A");
        assert_eq!(section.measures.len(), 1);
    }

    #[test]
    fn test_section_parser_with_repeat() {
        let tokens = vec![
            Token::new(TokenKind::Section, 1, 1),
            Token::new(TokenKind::StringLit("A".into()), 1, 9),
            Token::new(TokenKind::Repeat, 1, 13),
            Token::new(TokenKind::LParen, 1, 19),
            Token::new(TokenKind::IntLit(3), 1, 20),
            Token::new(TokenKind::RParen, 1, 21),
            Token::new(TokenKind::LBrace, 1, 23),
            Token::new(TokenKind::RBrace, 2, 1),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let section = SectionParser.try_parse(&mut ctx).unwrap();
        assert_eq!(section.repeat_times, Some(3));
    }

    #[test]
    fn test_chord_parser_halfdim() {
        // [B halfdim 7]
        let tokens = vec![
            Token::new(TokenKind::LBracket, 1, 1),
            Token::new(TokenKind::NoteName(sonus::NoteName::B), 1, 2),
            Token::new(TokenKind::Ident("halfdim".into()), 1, 3),
            Token::new(TokenKind::IntLit(7), 1, 10),
            Token::new(TokenKind::RBracket, 1, 11),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let chord = ChordParser.try_parse(&mut ctx).unwrap();
        assert_eq!(chord.symbol.quality, Some(ChordQuality::HalfDim));
        assert_eq!(chord.symbol.base_number, Some(7));
    }

    #[test]
    fn test_chord_parser_dim() {
        // [C dim]
        let tokens = vec![
            Token::new(TokenKind::LBracket, 1, 1),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 2),
            Token::new(TokenKind::Ident("dim".into()), 1, 3),
            Token::new(TokenKind::RBracket, 1, 6),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let chord = ChordParser.try_parse(&mut ctx).unwrap();
        assert_eq!(chord.symbol.quality, Some(ChordQuality::Dim));
    }

    #[test]
    fn test_chord_parser_inversion() {
        // [Cmaj /2]
        let tokens = vec![
            Token::new(TokenKind::LBracket, 1, 1),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 2),
            Token::new(TokenKind::Ident("maj".into()), 1, 3),
            Token::new(TokenKind::Slash, 1, 6),
            Token::new(TokenKind::IntLit(2), 1, 7),
            Token::new(TokenKind::RBracket, 1, 8),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let chord = ChordParser.try_parse(&mut ctx).unwrap();
        assert_eq!(chord.inversion, 2);
        assert!(chord.slash_bass.is_none());
    }

    #[test]
    fn test_chord_parser_slash_bass_still_works() {
        // [C / E] — slash bass, not inversion
        let tokens = vec![
            Token::new(TokenKind::LBracket, 1, 1),
            Token::new(TokenKind::NoteName(sonus::NoteName::C), 1, 2),
            Token::new(TokenKind::Slash, 1, 3),
            Token::new(TokenKind::NoteName(sonus::NoteName::E), 1, 5),
            Token::new(TokenKind::RBracket, 1, 6),
            Token::new(TokenKind::Eof, 0, 0),
        ];
        let mut ctx = make_ctx(tokens);
        let chord = ChordParser.try_parse(&mut ctx).unwrap();
        assert_eq!(chord.inversion, 0);
        assert!(chord.slash_bass.is_some());
    }
}
