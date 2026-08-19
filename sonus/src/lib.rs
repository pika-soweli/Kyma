//! # sonus — 纯乐理领域模型
//!
//! 基于 `rust-music-theory` 开源乐理库构建，扩展双升降记号、30 种音阶、
//! Forte 集合论、Lead-sheet 和弦符号、和弦识别等高级功能。
//!
//! ## 模块层级
//!
//! | 层级 | 模块 | 说明 |
//! |------|------|------|
//! | 底层 | `rmt` | rust-music-theory 重导出（音高/音阶/和弦/音程基础模型） |
//! | 基础 | `pitch` | 音名 / 音级类 / 变音记号 / 音高（扩展双升降 + rmt 互转） |
//! | 基础 | `duration` `tempo` | 时值与节拍 |
//! | 理论 | `pcset` | 音级集合（Forte 集合论） |
//! | 理论 | `interval` | 音程（含复合音程、转位、协和性 + rmt 互转） |
//! | 音阶 | `scale` `key` | 30 种音阶字典 + 调式（rmt 覆盖范围内委托生成） |
//! | 和弦 | `chord` | 和弦品质 / Lead-sheet 符号 / 多候选识别（rmt 品质互转） |
//! | 领域 | `note` `instrument` `score` | 音符 / 乐器 / 乐谱结构

/// rust-music-theory 底层乐理库重导出。
///
/// 提供 `PitchSymbol`、`NoteLetter`、`Scale`、`Mode`、`Chord` 等基础类型。
/// sonus 的各模块在此基础上扩展，并提供双向 `From` 转换。
pub use rust_music_theory as rmt;

pub mod pitch;
pub mod pcset;
pub mod interval;
pub mod scale;
pub mod key;
pub mod chord;
pub mod duration;
pub mod tempo;
pub mod note;
pub mod instrument;
pub mod score;

pub use pitch::*;
pub use pcset::*;
pub use interval::*;
pub use scale::*;
pub use key::*;
pub use chord::*;
pub use duration::*;
pub use tempo::*;
pub use note::*;
pub use instrument::*;
pub use score::*;
