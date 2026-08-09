//! 词法分析器 — .tm 文本 → Token 流（新语法 v2）。
//!
//! ## 词法规则
//!
//! | 输入模式 | Token |
//! |----------|-------|
//! | `@` | `At`（头部指令前缀） |
//! | `;` | 注释（跳过至行尾） |
//! | `A`-`G`（单字符） | `NoteName` |
//! | `A`-`G` 后跟字母/变音 | `Ident`（解析器拆分） |
//! | `R`（单字符） | `Rest` |
//! | `b` / `bb` / `#` / `x` / `=` | `Accidental` |
//! | `:N` / `:N.` | `Duration`（时值覆盖） |
//! | `:` (非数字) | `Colon`（tuplet 比例用） |
//! | `~` | `Tilde`（连音线） |
//! | `0`-`9` | `IntLit` |
//! | `"…"` | `StringLit` |
//! | `{ } | [ ] / ( ) ,` | 标点 |
//! | 其他字母序列 | `Ident`（检查关键字表） |

use crate::token::*;
use sonus::{Accidental, NoteName};

/// 编译错误。
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, col {}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for CompileError {}

/// 词法分析器。
pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    // ── 字符操作 ──

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                // `;` 行注释（新语法）
                Some(';') => {
                    while self.peek().map_or(false, |c| c != '\n') {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn read_alpha(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphabetic() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn read_digits(&mut self) -> u32 {
        let mut n: u32 = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                n = n * 10 + (c as u32 - b'0' as u32);
                self.advance();
            } else {
                break;
            }
        }
        n
    }

    fn read_string(&mut self) -> Result<String, CompileError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // skip opening "
        let mut s = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(s);
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => s.push(c),
                        None => {
                            return Err(CompileError {
                                message: "unterminated string literal".into(),
                                line,
                                col,
                            });
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
                None => {
                    return Err(CompileError {
                        message: "unterminated string literal".into(),
                        line,
                        col,
                    });
                }
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, CompileError> {
        self.skip_ws_and_comments();

        let line = self.line;
        let col = self.col;

        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(Token::new(TokenKind::Eof, line, col)),
        };

        // ── 标点 ──
        match c {
            '@' => { self.advance(); return Ok(Token::new(TokenKind::At, line, col)); }
            '{' => { self.advance(); return Ok(Token::new(TokenKind::LBrace, line, col)); }
            '}' => { self.advance(); return Ok(Token::new(TokenKind::RBrace, line, col)); }
            '|' => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    return Ok(Token::new(TokenKind::ColonColon, line, col));
                }
                return Ok(Token::new(TokenKind::Pipe, line, col));
            }
            '[' => { self.advance(); return Ok(Token::new(TokenKind::LBracket, line, col)); }
            ']' => { self.advance(); return Ok(Token::new(TokenKind::RBracket, line, col)); }
            '/' => { self.advance(); return Ok(Token::new(TokenKind::Slash, line, col)); }
            '(' => { self.advance(); return Ok(Token::new(TokenKind::LParen, line, col)); }
            ')' => { self.advance(); return Ok(Token::new(TokenKind::RParen, line, col)); }
            ',' => { self.advance(); return Ok(Token::new(TokenKind::Comma, line, col)); }
            '~' => { self.advance(); return Ok(Token::new(TokenKind::Tilde, line, col)); }
            '"' => {
                let s = self.read_string()?;
                return Ok(Token::new(TokenKind::StringLit(s), line, col));
            }
            '#' => { self.advance(); return Ok(Token::new(TokenKind::Accidental(Accidental::Sharp), line, col)); }
            '=' => { self.advance(); return Ok(Token::new(TokenKind::Accidental(Accidental::Natural), line, col)); }
            _ => {}
        }

        // ── 时值 :N / :N. / :（Colon）──
        if c == ':' {
            self.advance();
            // : 后面跟数字 → Duration
            if self.peek().map_or(false, |d| d.is_ascii_digit()) {
                let base = self.read_digits();
                // 检查附点
                let dotted = if self.peek() == Some('.') {
                    self.advance();
                    true
                } else {
                    false
                };
                return Ok(Token::new(TokenKind::Duration { base, dotted }, line, col));
            }
            // : 后面非数字 → Colon（tuplet 比例等）
            return Ok(Token::new(TokenKind::Colon, line, col));
        }

        // ── 数字 → IntLit ──
        if c.is_ascii_digit() {
            let n = self.read_digits();
            return Ok(Token::new(TokenKind::IntLit(n), line, col));
        }

        // ── R → Rest（单字符，不跟字母）──
        if c == 'R' && !self.peek2().map_or(false, |d| d.is_alphabetic() || d == '_') {
            self.advance();
            return Ok(Token::new(TokenKind::Rest, line, col));
        }

        // ── A-G → NoteName 或 Ident ──
        if "CDEFGAB".contains(c) {
            if self.peek2().map_or(false, |d| d.is_alphabetic() || d == '_') {
                let s = self.read_alpha();
                return Ok(Token::new(TokenKind::Ident(s), line, col));
            }
            self.advance();
            let name = NoteName::from_char(c).unwrap();
            return Ok(Token::new(TokenKind::NoteName(name), line, col));
        }

        // ── b → DoubleFlat / Flat / Ident ──
        if c == 'b' {
            if self.peek2() == Some('b') {
                self.advance();
                self.advance();
                return Ok(Token::new(TokenKind::Accidental(Accidental::DoubleFlat), line, col));
            }
            if self.peek2().map_or(false, |d| d.is_alphabetic() || d == '_') {
                let s = self.read_alpha();
                return Ok(Token::new(TokenKind::Ident(s), line, col));
            }
            self.advance();
            return Ok(Token::new(TokenKind::Accidental(Accidental::Flat), line, col));
        }

        // ── x → DoubleSharp / Ident ──
        if c == 'x' {
            if self.peek2().map_or(false, |d| d.is_alphabetic() || d == '_') {
                let s = self.read_alpha();
                return Ok(Token::new(TokenKind::Ident(s), line, col));
            }
            self.advance();
            return Ok(Token::new(TokenKind::Accidental(Accidental::DoubleSharp), line, col));
        }

        // ── 其他字母 → Ident（检查关键字）──
        if c.is_alphabetic() || c == '_' {
            let s = self.read_alpha();
            if let Some(kw) = keyword_lookup(&s) {
                return Ok(Token::new(kw, line, col));
            }
            return Ok(Token::new(TokenKind::Ident(s), line, col));
        }

        Err(CompileError {
            message: format!("unexpected character: '{}'", c),
            line,
            col,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<TokenKind> {
        Lexer::new(input)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_at_header() {
        assert_eq!(
            lex("@title(\"Hello\")"),
            vec![
                TokenKind::At,
                TokenKind::Ident("title".into()),
                TokenKind::LParen,
                TokenKind::StringLit("Hello".into()),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_semicolon_comment() {
        assert_eq!(
            lex("; this is a comment\n@tempo(120)"),
            vec![
                TokenKind::At,
                TokenKind::Ident("tempo".into()),
                TokenKind::LParen,
                TokenKind::IntLit(120),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_note_default_duration() {
        assert_eq!(
            lex("C4"),
            vec![
                TokenKind::NoteName(NoteName::C),
                TokenKind::IntLit(4),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_duration_override() {
        assert_eq!(
            lex("C4:2"),
            vec![
                TokenKind::NoteName(NoteName::C),
                TokenKind::IntLit(4),
                TokenKind::Duration { base: 2, dotted: false },
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dotted_duration() {
        assert_eq!(
            lex("C4:4."),
            vec![
                TokenKind::NoteName(NoteName::C),
                TokenKind::IntLit(4),
                TokenKind::Duration { base: 4, dotted: true },
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_rest() {
        assert_eq!(
            lex("R"),
            vec![TokenKind::Rest, TokenKind::Eof]
        );
    }

    #[test]
    fn test_rest_with_duration() {
        assert_eq!(
            lex("R:2"),
            vec![
                TokenKind::Rest,
                TokenKind::Duration { base: 2, dotted: false },
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_sharp() {
        assert_eq!(
            lex("C#4"),
            vec![
                TokenKind::NoteName(NoteName::C),
                TokenKind::Accidental(Accidental::Sharp),
                TokenKind::IntLit(4),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_tie() {
        assert_eq!(
            lex("C4 ~ C4"),
            vec![
                TokenKind::NoteName(NoteName::C),
                TokenKind::IntLit(4),
                TokenKind::Tilde,
                TokenKind::NoteName(NoteName::C),
                TokenKind::IntLit(4),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_chord_brackets() {
        assert_eq!(
            lex("[C maj 7]"),
            vec![
                TokenKind::LBracket,
                TokenKind::NoteName(NoteName::C),
                TokenKind::Ident("maj".into()),
                TokenKind::IntLit(7),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_compact_chord() {
        assert_eq!(
            lex("[Cmaj7]"),
            vec![
                TokenKind::LBracket,
                TokenKind::Ident("Cmaj".into()),
                TokenKind::IntLit(7),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_key_with_comma() {
        assert_eq!(
            lex("@key(C, major)"),
            vec![
                TokenKind::At,
                TokenKind::Ident("key".into()),
                TokenKind::LParen,
                TokenKind::NoteName(NoteName::C),
                TokenKind::Comma,
                TokenKind::Ident("major".into()),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_structural_keywords() {
        assert_eq!(
            lex("track section voice let"),
            vec![
                TokenKind::Track,
                TokenKind::Section,
                TokenKind::Voice,
                TokenKind::Let,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_double_flat() {
        assert_eq!(
            lex("Cbb4"),
            vec![
                TokenKind::Ident("Cbb".into()),
                TokenKind::IntLit(4),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_colon_alone() {
        // 新语法中 : 后跟数字始终为 Duration，仅裸 : 为 Colon
        assert_eq!(
            lex(":"),
            vec![TokenKind::Colon, TokenKind::Eof]
        );
    }
}
