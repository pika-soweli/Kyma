//! 和弦模块 — 品质 / Lead-sheet 符号 / 多候选识别。
//!
//! ## 结构
//!
//! | 子模块 | 职责 |
//! |--------|------|
//! | `quality` | 三和弦品质（7 种） |
//! | `symbol` | Lead-sheet 和弦符号 + 和弦实体 |
//! | `detect` | 从音级类列表识别和弦（多候选） |

pub mod quality;
pub mod symbol;
pub mod detect;

pub use quality::*;
pub use symbol::*;
pub use detect::*;
