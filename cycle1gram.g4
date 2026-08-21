grammar Cycle1Gram;

// ── tok-musi 新语法 v2 — ANTLR4 形式化描述 ──────────────────────────────
// 对应 opus/src/parser.rs + opus/src/lexer.rs 的实际实现。

// ═══════════════════════════════════════════════════════════════════════════
// 词法规则（Lexer Rules，大写起始）
// ═══════════════════════════════════════════════════════════════════════════

WS
    : [ \t\r\n]+ -> skip
    ;

COMMENT
    : ';' ~[\r\n]* -> skip
    ;

// ── @ 头部指令前缀 ──
AT : '@' ;

// ── 音乐记号 ──
NOTE_NAME
    : [A-G]                // C D E F G A B
    ;

ACCIDENTAL
    : 'bb'                 // 重降
    | '##'                 // 重升
    | 'b'                  // 降号
    | '#'                  // 升号
    | '='                  // 自然
    ;

REST : 'R' ;

// ── 时值：:N 或 :N.（如 :4 :2. :8）──
// ANTLR4 lexer 最长匹配：DURATION (':'+DIGIT+{'.'?}) 优先于 COLON (':')
// tuplet ratio "3:2" 实际 token 流为 INTEGER DURATION（见注释）
DURATION
    : ':' DIGIT+ '.'?
    ;

// ── 连音线 ──
TILDE : '~' ;

// ── 数字 ──
fragment DIGIT : [0-9] ;

INTEGER
    : DIGIT+
    ;

// ── 字符串字面量 ──
STRING
    : '"' (~["\r\n])* '"'
    ;

// ── 括号和标点 ──
LPAREN  : '(' ;
RPAREN  : ')' ;
LBRACE  : '{' ;
RBRACE  : '}' ;
LBRACKET: '[' ;
RBRACKET: ']' ;
PIPE    : '|' ;
COMMA   : ',' ;
SLASH   : '/' ;
COLON   : ':' ;
          // 单独 ':' 无数字时产生（正常语法中极少用）
          // tuplet ratio "3:2" 在 lexer 里输出 INTEGER DURATION(2)，非 COLON

// ── 关键字和标识符 ──
KEYWORD
    : 'track'
    | 'section'
    | 'voice'
    | 'let'
    | 'repeat'
    | 'grace'
    ;

IDENT
    : [A-Za-z_][A-Za-z0-9_]*
    ;

// ═══════════════════════════════════════════════════════════════════════════
// 语法规则（Parser Rules，小写起始）
// ═══════════════════════════════════════════════════════════════════════════

// ── 顶层：头部 + 音轨 ──
score
    : header* track* EOF
    ;

// ── @ 头部指令 ──
header
    : AT IDENT LPAREN header_value RPAREN
    ;

header_value
    : STRING                                                         # HeaderTitle
    | pitch COMMA scale_type                                         # HeaderKey
    | INTEGER                                                        # HeaderTempo
    | INTEGER SLASH INTEGER                                          # HeaderTime
    | INTEGER                                                        # HeaderDur
    ;

// ── 音轨 ──
track
    : 'track' STRING instrument? LBRACE section* RBRACE
    ;

instrument
    : IDENT
    ;

// ── 段落 ──
section
    : 'section' STRING repeat? LBRACE measure* RBRACE
    ;

repeat
    : 'repeat' LPAREN INTEGER RPAREN
    ;

// ── 小节 ──
measure
    : event (PIPE event)*
    ;

// ── 事件 ──
event
    : note TILDE                             # TieEvent     // 连音线（parser 合并后一音符时值）
    | note                                   # NoteEvent
    | REST duration?                         # RestEvent
    | chord duration?                        # ChordEvent
    | AT control_name LPAREN control_arg* RPAREN   # ControlEvent
    | 'grace' LPAREN note RPAREN             # GraceEvent
    | INTEGER DURATION LBRACE event+ RBRACE  # TupletEvent  // "3:2 { ... }"
    ;

// ── 音符 ──
note
    : pitch duration?
    ;

pitch
    : NOTE_NAME ACCIDENTAL? INTEGER?           // "C#4" 拆为 NOTE_NAME('+) INTLIT
    | IDENT                                    // "Csharp" → parser 拆分首字符
    ;

duration
    : DURATION
    ;

// ── 和弦 ──
chord
    : LBRACKET chord_root chord_desc* (SLASH pitch)? RBRACKET
    ;

chord_root
    : pitch
    ;

chord_desc
    : IDENT             // maj / minor / min / dim / aug / sus / power …
    | INTEGER           // 6 / 7 / 9 / 11 / 13 …
    | ACCIDENTAL INTEGER // #5 / b7 / bb3 … 和弦变音
    | 'add' INTEGER     // add2 / add4 …
    | 'no' INTEGER      // no3 / no5 …
    ;

// ── 音阶类型（@key / 局部 @key 的 scale_type）──
scale_type
    : IDENT
    ;

// ── 局部控制指令名 ──
control_name
    : IDENT
    ;

control_arg
    : pitch
    | INTEGER
    | IDENT
    ;

// ═══════════════════════════════════════════════════════════════════════════
// 语法要点说明
// ═══════════════════════════════════════════════════════════════════════════
//
// 1. pitch 规则中 NOTE_NAME ACCIDENTAL? INTEGER? 对应 lex("C#4")
//    → [NOTE_NAME(C), ACCIDENTAL(#), INTEGER(4)]
//    IDENT 分支对应 lex("Csharp") → [IDENT("Csharp")]，parser 做拆分
//
// 2. tuplet "3:2 { C4 E4 G4 }"
//    lexer 输出: [INTEGER(3), DURATION(:2), LBRACE, ...]
//    即 INTEGER DURATION LBRACE event+ RBRACE
//
// 3. 连音线 C4 ~ C4
//    grammar 用 TieEvent 捕捉 ~，parser 将两音符合并为一个（同 pitch 时值相加）
//
// 4. 时值缺失时使用 @dur 默认值（parser 层逻辑，grammar 无法表达）
//
// ── 示例对照 ──
//
//   @title("Canon in D")
//   @tempo(120)
//   @time(4/4)
//   @dur(4)
//
//   track "melody" piano {
//     section "A" repeat(2) {
//       G4 E4 C4 D4 | G4 E4 C4 D4 |
//       [G maj 7] [Cmaj7] [Dmaj7] [G7] |
//     }
//     section "B" {
//       R:2 | C4 ~ D4 |
//       grace(E5) F#5:8 |
//       3:2 { G4 E4 C4 } |
//       @key(D, major) |
//       @tempo(100) |
//     }
//   }
//
//   track "bass" contrabass {
//     section "A" { D2:1 | ... }
//   }
