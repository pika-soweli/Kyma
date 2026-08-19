//! `.bm` 二进制 → `PerformScore` 读取器。
//!
//! 直接解析二进制格式为面向播放的轻量类型，
//! 不经过任何符号化音乐理论模型。

use crate::{
    PerfControl, PerfDuration, PerfEvent, PerformMeasure, PerformScore,
    PerformSection, PerformTrack,
};

/// IR 读取错误。
#[derive(Debug, Clone)]
pub enum ReadError {
    UnexpectedEof,
    InvalidUtf8,
    BadMagic,
    UnsupportedVersion(u16),
    BadEventTag(u8),
    BadControlTag(u8),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of data"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8"),
            Self::BadMagic => write!(f, "bad magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {}", v),
            Self::BadEventTag(t) => write!(f, "bad event tag: {}", t),
            Self::BadControlTag(t) => write!(f, "bad control tag: {}", t),
        }
    }
}

impl std::error::Error for ReadError {}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8, ReadError> {
        if self.pos >= self.buf.len() {
            return Err(ReadError::UnexpectedEof);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, ReadError> {
        if self.pos + 2 > self.buf.len() {
            return Err(ReadError::UnexpectedEof);
        }
        let v = u16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, ReadError> {
        let len = self.u16()? as usize;
        if self.pos + len > self.buf.len() {
            return Err(ReadError::UnexpectedEof);
        }
        let s = std::str::from_utf8(&self.buf[self.pos..self.pos + len])
            .map_err(|_| ReadError::InvalidUtf8)?
            .to_string();
        self.pos += len;
        Ok(s)
    }

    fn duration(&mut self) -> Result<PerfDuration, ReadError> {
        let base = self.u16()? as u32;
        let dotted = self.u8()? != 0;
        Ok(PerfDuration { base, dotted })
    }

    fn skip_pitch(&mut self) -> Result<(), ReadError> {
        self.u8()?; // note_name
        self.u8()?; // accidental
        let has_oct = self.u8()?;
        if has_oct != 0 {
            self.u8()?; // octave
        }
        Ok(())
    }

    fn skip_key(&mut self) -> Result<(), ReadError> {
        self.skip_pitch()?;
        self.u8()?; // scale_type
        Ok(())
    }
}

const MAGIC: &[u8; 4] = b"BMIR";
const VERSION: u16 = 2;

/// 从 `.bm` 二进制读取 `PerformScore`。
pub fn read(bytes: &[u8]) -> Result<PerformScore, ReadError> {
    let mut r = Reader::new(bytes);

    if bytes.len() < 6 {
        return Err(ReadError::UnexpectedEof);
    }
    if &bytes[0..4] != MAGIC {
        return Err(ReadError::BadMagic);
    }
    r.pos = 4;

    let version = r.u16()?;
    if version != VERSION {
        return Err(ReadError::UnsupportedVersion(version));
    }

    let flags = r.u8()?;
    let mut title = None;
    let mut global_tempo: u16 = 120;
    let mut global_time = None;

    if flags & 1 != 0 {
        title = Some(r.string()?);
    }
    if flags & 2 != 0 {
        r.skip_key()?;
    }
    if flags & 4 != 0 {
        global_tempo = r.u16()?;
    }
    if flags & 8 != 0 {
        let beats = r.u8()?;
        let beat_value = r.u8()?;
        global_time = Some((beats, beat_value));
    }
    if flags & 16 != 0 {
        r.u8()?; // default_dur base
        r.u8()?; // default_dur dotted
    }

    let track_count = r.u16()?;
    let mut tracks = Vec::with_capacity(track_count as usize);
    for _ in 0..track_count {
        tracks.push(read_track(&mut r)?);
    }

    Ok(PerformScore {
        title,
        global_tempo,
        global_time,
        tracks,
    })
}

fn read_track(r: &mut Reader) -> Result<PerformTrack, ReadError> {
    let name = r.string()?;
    let has_inst = r.u8()?;
    let instrument = if has_inst != 0 {
        Some(r.u8()?)
    } else {
        None
    };

    let section_count = r.u16()?;
    let mut sections = Vec::with_capacity(section_count as usize);
    for _ in 0..section_count {
        sections.push(read_section(r)?);
    }

    Ok(PerformTrack {
        name,
        instrument,
        sections,
    })
}

fn read_section(r: &mut Reader) -> Result<PerformSection, ReadError> {
    let name = r.string()?;
    let repeat = r.u8()?;
    let measure_count = r.u16()?;
    let mut measures = Vec::with_capacity(measure_count as usize);
    for _ in 0..measure_count {
        measures.push(read_measure(r)?);
    }
    Ok(PerformSection {
        name,
        repeat,
        measures,
    })
}

fn read_measure(r: &mut Reader) -> Result<PerformMeasure, ReadError> {
    let event_count = r.u16()?;
    let mut events = Vec::with_capacity(event_count as usize);
    for _ in 0..event_count {
        events.push(read_event(r)?);
    }
    Ok(PerformMeasure { events })
}

fn read_event(r: &mut Reader) -> Result<PerfEvent, ReadError> {
    let tag = r.u8()?;
    match tag {
        0 => {
            let midi = r.u8()?;
            let duration = r.duration()?;
            let velocity = r.u8()?;
            Ok(PerfEvent::Note {
                midi,
                duration,
                velocity,
            })
        }
        1 => {
            let duration = r.duration()?;
            Ok(PerfEvent::Rest { duration })
        }
        2 => {
            let midi_count = r.u8()?;
            let mut midis = Vec::with_capacity(midi_count as usize);
            for _ in 0..midi_count {
                midis.push(r.u8()?);
            }
            let duration = r.duration()?;
            let velocity = r.u8()?;
            Ok(PerfEvent::Chord {
                midis,
                duration,
                velocity,
            })
        }
        3 => Ok(PerfEvent::Control(read_control(r)?)),
        4 => {
            let num = r.u8()?;
            let den = r.u8()?;
            let event_count = r.u16()?;
            let mut events = Vec::with_capacity(event_count as usize);
            for _ in 0..event_count {
                events.push(read_event(r)?);
            }
            Ok(PerfEvent::Tuplet {
                ratio: (num, den),
                events,
            })
        }
        5 => {
            let midi = r.u8()?;
            let duration = r.duration()?;
            let velocity = r.u8()?;
            Ok(PerfEvent::Grace {
                midi,
                duration,
                velocity,
            })
        }
        _ => Err(ReadError::BadEventTag(tag)),
    }
}

fn read_control(r: &mut Reader) -> Result<PerfControl, ReadError> {
    let ctrl_type = r.u8()?;
    match ctrl_type {
        0 => {
            let root = r.u8()?;
            r.u8()?; // acc (skip for perform)
            let has_oct = r.u8()?;
            if has_oct != 0 {
                r.u8()?; // octave (skip)
            }
            let scale_type = r.u8()?;
            Ok(PerfControl::Key { root, scale_type })
        }
        1 => {
            let bpm = r.u16()?;
            Ok(PerfControl::Tempo(bpm))
        }
        2 => {
            let beats = r.u8()?;
            let beat_value = r.u8()?;
            Ok(PerfControl::TimeSig { beats, beat_value })
        }
        3 => {
            let p = r.u8()?;
            Ok(PerfControl::PedalOn(p))
        }
        4 => {
            let p = r.u8()?;
            Ok(PerfControl::PedalOff(p))
        }
        5 => {
            let v = r.u8()?;
            Ok(PerfControl::Volume(v))
        }
        6 => {
            let s = r.string()?;
            Ok(PerfControl::DynamicMark(s))
        }
        _ => Err(ReadError::BadControlTag(ctrl_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bad_magic() {
        let result = read(b"XXXX\x02\x00\x00");
        assert!(matches!(result, Err(ReadError::BadMagic)));
    }

    #[test]
    fn test_read_too_short() {
        let result = read(b"BM");
        assert!(matches!(result, Err(ReadError::UnexpectedEof)));
    }

    #[test]
    fn test_read_unsupported_version() {
        let result = read(b"BMIR\x99\x00\x00");
        assert!(matches!(result, Err(ReadError::UnsupportedVersion(0x99))));
    }

    #[test]
    fn test_read_empty_score() {
        // Minimal valid .bm: magic + version(2) + flags(0) + track_count(0)
        let bytes = b"BMIR\x02\x00\x00\x00\x00";
        let score = read(bytes).unwrap();
        assert_eq!(score.title, None);
        assert_eq!(score.global_tempo, 120); // default
        assert_eq!(score.tracks.len(), 0);
    }
}
