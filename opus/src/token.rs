//! Token 类型定义 — toki-musi 新语法 v2。
//!
//! 新语法关键变化：
//! - 头部用 `@keyword(value)` 语法
//! - `;` 为注释（非 `//`）
//! - 时值用 `:N` / `:N.`（非 `-N` / `.N`），可选（有 `@dur` 默认值）
//! - `~` 连音线
//! - `{ }` 用于 tuplet 分组

use sonus::{Accidental, NoteName};

/// Token 种类。
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── @ 前缀头部标记 ──
    At, // @

    // ── 结构关键字 ──
    Track,
    Section,
    Voice,
    Let,
    Repeat,
    Grace,

    // ── 字面量 ──
    StringLit(String),
    IntLit(u32),

    // ── 音乐记号 ──
    NoteName(NoteName),
    Accidental(Accidental),
    Rest,
    /// 时值：base = 分母 (1/2/4/8/16…)，dotted = 是否附点。
    /// 新语法：`:N` 非附点，`:N.` 附点。
    Duration { base: u32, dotted: bool },

    // ── 和弦 ──
    LBracket, // [
    RBracket, // ]
    Slash,    // /

    // ── 标识符 ──
    Ident(String),

    // ── 标点 ──
    LParen, // (
    RParen, // )
    Comma,  // ,
    LBrace, // {
    RBrace, // }
    Pipe,   // |
    Colon,  // : (用于 tuplet 比例 3:2，非时值上下文)
    Tilde,  // ~ 连音线

    // ── 特殊 ──
    Eof,
}

/// 带位置信息的 Token。
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Self { kind, line, col }
    }
}

/// 结构关键字表（非 @ 前缀的裸关键字）。
pub fn keyword_lookup(s: &str) -> Option<TokenKind> {
    match s {
        "track" => Some(TokenKind::Track),
        "section" => Some(TokenKind::Section),
        "voice" => Some(TokenKind::Voice),
        "let" => Some(TokenKind::Let),
        "repeat" => Some(TokenKind::Repeat),
        "grace" => Some(TokenKind::Grace),
        _ => None,
    }
}
