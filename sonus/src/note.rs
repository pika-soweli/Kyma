//! 音符 — 零 MIDI 耦合。
//!
//! 音符 = 音高 + 时值 + 力度 + 轨道。

use super::pitch::Pitch;
use super::duration::Duration;

/// 音符类型：正常音符或休止符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Normal(Pitch),
    Rest,
}

/// 音符实体。
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub kind: NoteKind,
    pub duration: Duration,
    velocity: u8,
    pub track_id: usize,
}

impl Note {
    pub fn new_note(pitch: Pitch, duration: Duration, track_id: usize) -> Self {
        Self {
            kind: NoteKind::Normal(pitch),
            duration,
            velocity: 100,
            track_id,
        }
    }

    pub fn new_rest(duration: Duration, track_id: usize) -> Self {
        Self {
            kind: NoteKind::Rest,
            duration,
            velocity: 0,
            track_id,
        }
    }

    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    pub fn set_velocity(&mut self, vel: u8) {
        self.velocity = vel.min(127);
    }

    pub fn is_rest(&self) -> bool {
        matches!(self.kind, NoteKind::Rest)
    }

    pub fn pitch(&self) -> Option<&Pitch> {
        match &self.kind {
            NoteKind::Normal(p) => Some(p),
            NoteKind::Rest => None,
        }
    }

    /// 转调：按半音数偏移音高（休止符不受影响）。
    pub fn transpose(&mut self, semitones: i8) {
        if let NoteKind::Normal(ref mut pitch) = self.kind {
            *pitch = pitch.transpose(semitones);
        }
    }

    pub fn display(&self) -> String {
        let head = match &self.kind {
            NoteKind::Normal(p) => p.display(),
            NoteKind::Rest => "R".to_string(),
        };
        format!("{}{}", head, self.duration.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pitch::{NoteName, Accidental};

    #[test]
    fn test_new_note() {
        let pitch = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        let note = Note::new_note(pitch, Duration::quarter(), 0);
        assert!(!note.is_rest());
        assert_eq!(note.velocity(), 100);
        assert_eq!(note.display(), "C4-4");
    }

    #[test]
    fn test_new_rest() {
        let note = Note::new_rest(Duration::quarter(), 0);
        assert!(note.is_rest());
        assert_eq!(note.velocity(), 0);
        assert_eq!(note.display(), "R-4");
    }

    #[test]
    fn test_set_velocity() {
        let mut note = Note::new_rest(Duration::quarter(), 0);
        note.set_velocity(200);
        assert_eq!(note.velocity(), 127);
    }

    #[test]
    fn test_transpose() {
        let pitch = Pitch::new(NoteName::C, Accidental::Natural, Some(4));
        let mut note = Note::new_note(pitch, Duration::quarter(), 0);
        note.transpose(2);
        assert_eq!(note.pitch().unwrap().name, NoteName::D);
    }

    #[test]
    fn test_rest_transpose() {
        let mut note = Note::new_rest(Duration::quarter(), 0);
        note.transpose(5);
        assert!(note.is_rest());
    }
}
