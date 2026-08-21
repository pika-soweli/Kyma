//! 递归下降语法分析器 — Token 流 → sonus::Score（新语法 v2）。
//!
//! ## 架构
//!
//! ```text
//! Parser (编排)
//!   ├── ParseContext (共享状态: tokens / pos / default_duration)
//!   └── rules::* (各独立解析器 struct，统一实现 ParseRule trait)
//!         ├── HeaderParser    — @title / @key / @tempo / @time / @dur
//!         ├── TrackParser     — track "name" instrument? { section* }
//!         ├── SectionParser   — section "name" repeat(N)? { measure* }
//!         ├── EventDispatcher — 按 token 分派到下列子解析器
//!         ├── NoteParser      — pitch duration? (含连音线 ~)
//!         ├── RestParser      — R duration?
//!         ├── ChordParser     — [ pitch desc? (/ pitch)? ] duration?
//!         ├── GraceParser     — grace(pitch duration?)
//!         ├── TupletParser    — N:M { event* }
//!         └── ControlParser   — @cresc / @key(...) / @pedal(...) ...
//! ```
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
//! tie       := event '~'                    ; 连音线（合并时值）
//! duration  := ':N' | ':N.'
//! ```

pub mod rules;

use crate::lexer::CompileError;
use crate::token::*;
use sonus::{
    rmt, Accidental, Duration, NoteName, Pitch, ScaleDirection, ScaleType, Score,
};
use rules::{HeaderParser, TrackParser};

// ── ParseRule trait ────────────────────────────────────────

/// 解析规则 trait — 每种语法结构对应一个实现。
///
/// 实现者为独立的 unit struct（如 [`NoteParser`](rules::NoteParser)），
/// 通过 `try_parse` 从共享的 [`ParseContext`] 中消费 token 并产出结果。
pub trait ParseRule<T> {
    fn try_parse(&self, ctx: &mut ParseContext) -> Result<T, CompileError>;
}

// ── ParseContext ───────────────────────────────────────────

/// 解析共享上下文 — 持有 token 流、位置指针、默认时值与调性状态。
///
/// 提供 token 操作（peek / advance / expect）和通用子解析
/// （pitch / duration / scale_type）供各 [`ParseRule`] 实现使用。
pub struct ParseContext {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) default_duration: Duration,
    /// 当前调号根音（来自 @key 头部或 @key(...) 控制命令）。
    pub(crate) key_root: Option<NoteName>,
    /// 当前调式（Ionian/Dorian/...）。
    pub(crate) key_mode: Option<rmt::scale::Mode>,
}

impl ParseContext {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            default_duration: Duration::quarter(),
            key_root: None,
            key_mode: None,
        }
    }

    pub fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    pub fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    pub fn replace_current(&mut self, kind: TokenKind) {
        if self.pos < self.tokens.len() {
            let line = self.tokens[self.pos].line;
            let col = self.tokens[self.pos].col;
            self.tokens[self.pos] = Token::new(kind, line, col);
        }
    }

    pub fn err(&self, msg: &str) -> CompileError {
        let t = self.peek();
        CompileError { message: msg.to_string(), line: t.line, col: t.col }
    }

    pub fn expect(&mut self, kind: &TokenKind) -> Result<(), CompileError> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!("expected {:?}, found {:?}", kind, self.peek().kind)))
        }
    }

    pub fn expect_int(&mut self) -> Result<u32, CompileError> {
        match &self.peek().kind {
            TokenKind::IntLit(n) => {
                let n = *n;
                self.advance();
                Ok(n)
            }
            _ => Err(self.err("expected integer")),
        }
    }

    /// 解析可选时值标记 `:N` / `:N.`，不存在则返回默认时值。
    pub fn parse_optional_duration(&mut self) -> Duration {
        match &self.peek().kind {
            TokenKind::Duration { base, dotted } => {
                let d = Duration::new(*base, *dotted);
                self.advance();
                d
            }
            _ => self.default_duration,
        }
    }

    /// 解析音高字面量（NoteName / Ident 拆分 / Accidental / octave）。
    pub fn parse_pitch(&mut self) -> Result<Pitch, CompileError> {
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

        let pitch = Pitch::new(name, acc, octave);
        Ok(self.apply_key_signature(pitch))
    }

    /// 为音高应用当前调号的变音记号。
    ///
    /// 若音高已有显式变音记号（非 Natural），则不修改。
    /// 若调号未设置，则返回原音高。
    pub fn apply_key_signature(&self, pitch: Pitch) -> Pitch {
        match (self.key_root, self.key_mode) {
            (Some(root), mode) => pitch.apply_key_signature(root, mode),
            _ => pitch,
        }
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
}

// ── 共享辅助 ──────────────────────────────────────────────

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

/// 解析音阶类型 + 可选方向。
///
/// 语法：`scale_type` 或 `scale_type ',' direction`
/// direction := 'asc' | 'desc'
pub(crate) fn parse_scale_type(ctx: &mut ParseContext) -> Result<(ScaleType, Option<ScaleDirection>), CompileError> {
    let st = match &ctx.peek().kind {
        TokenKind::Ident(s) => {
            let s = s.clone();
            if let Some(st) = scale_type_from_str(&s) {
                ctx.advance();
                st
            } else {
                return Err(ctx.err(&format!("unknown scale type: {}", s)));
            }
        }
        _ => return Err(ctx.err("expected scale type identifier")),
    };

    let direction = if matches!(ctx.peek().kind, TokenKind::Comma) {
        if let Some(TokenKind::Ident(s)) = ctx.tokens.get(ctx.pos + 1).map(|t| &t.kind) {
            if let Some(dir) = ScaleDirection::from_str(s) {
                ctx.advance(); // ','
                ctx.advance(); // direction ident
                Some(dir)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok((st, direction))
}

fn scale_type_from_str(s: &str) -> Option<ScaleType> {
    ALL_SCALE_TYPES.iter().find(|&&st| st.as_str() == s).copied()
}

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

// ── Parser (主编排) ────────────────────────────────────────

/// 顶层解析器 — 持有 [`ParseContext`]，编排各 [`ParseRule`] 实现完成 Score 构建。
pub struct Parser {
    ctx: ParseContext,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { ctx: ParseContext::new(tokens) }
    }

    pub fn parse(mut self) -> Result<Score, CompileError> {
        let mut score = Score::empty();
        let mut track_count = 0usize;

        loop {
            match &self.ctx.peek().kind {
                TokenKind::Eof => break,
                TokenKind::At => HeaderParser::try_parse(&mut self.ctx, &mut score)?,
                TokenKind::Track => {
                    let track = TrackParser::try_parse(&mut self.ctx, track_count)?;
                    score.push_track(track);
                    track_count += 1;
                }
                _ => return Err(self.ctx.err("expected '@header' or 'track'")),
            }
        }
        Ok(score)
    }
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use sonus::{
        Accidental, AlterType, ChordQuality, Duration, LocalControl,
        MeasureEvent, NoteName, PedalKind, ScaleType,
    };

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

    // ── Cycle 1: 局部控制事件 ──

    #[test]
    fn test_local_key() {
        let score = parse("track \"t\" {\nsection \"s\" {\n@key(A, minor) C4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Control(LocalControl::LocalKey(k)) = &m.events[0] {
            assert_eq!(k.root.name, NoteName::A);
            assert_eq!(k.scale_type, ScaleType::Minor);
        } else {
            panic!("expected LocalKey control");
        }
    }

    #[test]
    fn test_local_tempo() {
        let score = parse("track \"t\" {\nsection \"s\" {\n@tempo(90) C4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Control(LocalControl::LocalTempo(t)) = &m.events[0] {
            assert_eq!(t.bpm(), 90);
        } else {
            panic!("expected LocalTempo control");
        }
    }

    #[test]
    fn test_local_time() {
        let score = parse("track \"t\" {\nsection \"s\" {\n@time(6/8) C4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Control(LocalControl::LocalTime(ts)) = &m.events[0] {
            assert_eq!(ts.beats_per_bar, 6);
            assert_eq!(ts.beat_value, 8);
        } else {
            panic!("expected LocalTime control");
        }
    }

    #[test]
    fn test_pedal_on_off() {
        let score = parse("track \"t\" {\nsection \"s\" {\n@pedal(sustain, on) C4 |\n@pedal(sustain, off) |\n}\n}");
        let m0 = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Control(LocalControl::PedalOn(p)) = &m0.events[0] {
            assert_eq!(*p, PedalKind::Sustain);
        } else {
            panic!("expected PedalOn");
        }
        let m1 = &score.tracks[0].sections[0].measures[1];
        if let MeasureEvent::Control(LocalControl::PedalOff(p)) = &m1.events[0] {
            assert_eq!(*p, PedalKind::Sustain);
        } else {
            panic!("expected PedalOff");
        }
    }

    #[test]
    fn test_dyn_and_vol() {
        let score = parse("track \"t\" {\nsection \"s\" {\n@dyn(f) @vol(80) C4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Control(LocalControl::DynamicMark(d)) = &m.events[0] {
            assert_eq!(d, "f");
        } else {
            panic!("expected DynamicMark");
        }
        if let MeasureEvent::Control(LocalControl::Volume(v)) = &m.events[1] {
            assert_eq!(*v, 80);
        } else {
            panic!("expected Volume");
        }
    }

    // ── Cycle 1: 表情记号（无括号）──

    #[test]
    fn test_expression_marks() {
        for mark in ["cresc", "decresc", "rit", "accel", "fermata"] {
            let src = format!("track \"t\" {{\nsection \"s\" {{\n@{mark} C4 |\n}}\n}}");
            let score = parse(&src);
            let m = &score.tracks[0].sections[0].measures[0];
            if let MeasureEvent::Control(LocalControl::DynamicMark(d)) = &m.events[0] {
                assert_eq!(d, mark);
            } else {
                panic!("expected DynamicMark for @{mark}");
            }
        }
    }

    // ── Cycle 1: 连音符 ──

    #[test]
    fn test_tuplet_basic() {
        let score = parse("track \"t\" {\nsection \"s\" {\n3:2 { C4 E4 G4 } |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        assert_eq!(m.events.len(), 1);
        if let MeasureEvent::Tuplet(t) = &m.events[0] {
            assert_eq!(t.ratio, (3, 2));
            assert_eq!(t.events.len(), 3);
        } else {
            panic!("expected Tuplet");
        }
    }

    #[test]
    fn test_tuplet_nested_events() {
        let score = parse("track \"t\" {\nsection \"s\" {\n3:2 { C4:8 E4:8 G4:8 } |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Tuplet(t) = &m.events[0] {
            if let MeasureEvent::Note(n) = &t.events[0] {
                assert_eq!(n.duration, Duration::eighth());
            } else {
                panic!("expected Note inside tuplet");
            }
        } else {
            panic!("expected Tuplet");
        }
    }

    // ── Cycle 1: 装饰音 ──

    #[test]
    fn test_grace_note() {
        let score = parse("track \"t\" {\nsection \"s\" {\ngrace(C5) D4:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        assert_eq!(m.events.len(), 2);
        if let MeasureEvent::Grace(n) = &m.events[0] {
            assert_eq!(n.pitch().unwrap().name, NoteName::C);
            assert_eq!(n.pitch().unwrap().octave, Some(5));
        } else {
            panic!("expected Grace");
        }
    }

    #[test]
    fn test_grace_with_duration() {
        let score = parse("track \"t\" {\nsection \"s\" {\ngrace(C5:8) D4:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Grace(n) = &m.events[0] {
            assert_eq!(n.duration, Duration::eighth());
        } else {
            panic!("expected Grace");
        }
    }

    // ── Cycle 1: 综合场景 ──

    #[test]
    fn test_mixed_cycle1_events() {
        let score = parse("track \"piano\" piano {\nsection \"A\" {\n@pedal(sustain, on) grace(C5) 3:2 { D4:8 E4:8 F4:8 } G4:4 @pedal(sustain, off) |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        assert_eq!(m.events.len(), 5);
        assert!(matches!(m.events[0], MeasureEvent::Control(LocalControl::PedalOn(_))));
        assert!(matches!(m.events[1], MeasureEvent::Grace(_)));
        assert!(matches!(m.events[2], MeasureEvent::Tuplet(_)));
        assert!(matches!(m.events[3], MeasureEvent::Note(_)));
        assert!(matches!(m.events[4], MeasureEvent::Control(LocalControl::PedalOff(_))));
    }

    // ── Cycle 2: rmt 深度集成 ──

    #[test]
    fn test_halfdim_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[B halfdim 7]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.quality, Some(ChordQuality::HalfDim));
            assert_eq!(c.symbol.base_number, Some(7));
        } else {
            panic!("expected Chord");
        }
    }

    #[test]
    fn test_dim_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C dim]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.quality, Some(ChordQuality::Dim));
        } else {
            panic!("expected Chord");
        }
    }

    #[test]
    fn test_chord_inversion() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C maj /2]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.inversion, 2);
            assert!(c.slash_bass.is_none());
        } else {
            panic!("expected Chord");
        }
    }

    #[test]
    fn test_key_with_direction() {
        let score = parse("@key(C, major, asc)\ntrack \"t\" {\nsection \"s\" {\nC4:4 |\n}\n}");
        assert_eq!(score.global_key.unwrap().scale_type, ScaleType::Major);
    }

    #[test]
    fn test_dom_chord() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[G dom 7]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.symbol.quality, Some(ChordQuality::Dom));
            assert_eq!(c.symbol.base_number, Some(7));
        } else {
            panic!("expected Chord");
        }
    }

    #[test]
    fn test_chord_inversion_first() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C maj /1]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.inversion, 1);
        } else {
            panic!("expected Chord");
        }
    }

    #[test]
    fn test_chord_inversion_third() {
        let score = parse("track \"t\" {\nsection \"s\" {\n[C7 /3]:4 |\n}\n}");
        let m = &score.tracks[0].sections[0].measures[0];
        if let MeasureEvent::Chord(c) = &m.events[0] {
            assert_eq!(c.inversion, 3);
        } else {
            panic!("expected Chord");
        }
    }
}
