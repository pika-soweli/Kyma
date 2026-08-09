//! # sonus — 纯乐理领域模型
//!
//! 零 MIDI 耦合的音高、音程、音阶、调式、和弦、时值、乐器与乐谱结构。
//!
//! ## 模块层级
//!
//! | 层级 | 模块 | 说明 |
//! |------|------|------|
//! | 基础 | `pitch` | 音名 / 音级类 / 变音记号 / 音高 |
//! | 基础 | `duration` `tempo` | 时值与节拍 |
//! | 理论 | `pcset` | 音级集合（Forte 集合论） |
//! | 理论 | `interval` | 音程（含复合音程、转位、协和性） |
//! | 音阶 | `scale` `key` | 数据驱动音阶字典 + 调式 |
//! | 和弦 | `chord` | 和弦品质 / Lead-sheet 符号 / 多候选识别 |
//! | 领域 | `note` `instrument` `score` | 音符 / 乐器 / 乐谱结构 |

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
pub mod tuplet;
pub mod grace_note;
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
pub use tuplet::*;
pub use grace_note::*;
pub use score::*;
