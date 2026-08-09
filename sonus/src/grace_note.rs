//! 装饰音 — 极短时值的辅助音符，不占用正式拍值。
//!
//! 语法：`grace(C#5:8)` 或 `grace(C#5:16)`

use super::note::Note;

/// 装饰音：在正式音符之前快速演奏的辅助音符。
#[derive(Debug, Clone, PartialEq)]
pub struct GraceNote {
    pub note: Note,
}

impl GraceNote {
    pub fn new(note: Note) -> Self {
        Self { note }
    }

    /// 装饰音的标准时值（为正式音符时值的 1/4，上限八分音符）。
    pub fn duration(&self) -> super::duration::Duration {
        let base = self.note.duration.base.max(8);
        super::duration::Duration::new(base, false)
    }

    pub fn display(&self) -> String {
        format!("grace({})", self.note.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::Note;
    use crate::pitch::{Accidental, NoteName, Pitch};
    use crate::duration::Duration;

    #[test]
    fn test_grace_note_creation() {
        let note = Note::new_note(
            Pitch::new(NoteName::C, Accidental::Sharp, Some(5)),
            Duration::eighth(),
            64,
        );
        let grace = GraceNote::new(note);
        assert_eq!(grace.duration(), Duration::eighth());
    }

    #[test]
    fn test_grace_note_quarter_base() {
        let note = Note::new_note(
            Pitch::new(NoteName::D, Accidental::Natural, Some(4)),
            Duration::quarter(),
            64,
        );
        let grace = GraceNote::new(note);
        // 四分音符为基时，装饰音取 1/8 时值
        assert_eq!(grace.duration(), Duration::eighth());
    }
}
