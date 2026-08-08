//! # opus — toki-musi 编译器
//!
//! 将 `.tm` (toki-musi) 源文件编译为 `.bm` (Bin Musi IR) 二进制格式。
//!
//! ## 编译流水线
//!
//! ```text
//! .tm 文本 → Lexer → Token 流 → Parser → sonus::Score → ir::encode → .bm 二进制
//! ```
//!
//! ## 用法
//!
//! ```sh
//! opus input.tm              # 输出 input.bm
//! opus input.tm -o out.bm    # 指定输出路径
//! opus --decode input.bm     # 解码 .bm 并打印 Score 结构
//! ```

pub mod token;
pub mod lexer;
pub mod parser;
pub mod ir;

pub use lexer::CompileError;
pub use ir::IrError;

/// 完整编译流水线：`.tm` 文本 → `Score`。
pub fn compile_to_score(input: &str) -> Result<sonus::Score, CompileError> {
    let tokens = lexer::Lexer::new(input).tokenize()?;
    let score = parser::Parser::new(tokens).parse()?;
    Ok(score)
}

/// 完整编译流水线：`.tm` 文本 → `.bm` 二进制字节。
pub fn compile(input: &str) -> Result<Vec<u8>, CompileError> {
    let score = compile_to_score(input)?;
    Ok(ir::encode(&score))
}
