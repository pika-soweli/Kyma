//! # perform — .bm 演奏库
//!
//! 读取 `.bm` (Bin Musi IR) 二进制文件并通过 MIDI 播放。
//!
//! ## 架构
//!
//! ```text
//! .bm 二进制 → reader::read → PerformScore → player::play → MIDI 输出
//! ```
//!
//! `PerformScore` 是面向播放的轻量 IR，直接持有 MIDI 值，
//! 不依赖任何符号化音乐理论类型。

pub mod reader;
pub mod player;

use std::time::Duration;

/// 面向播放的轻量 IR — 直接持有 MIDI 值。
#[derive(Debug, Clone)]
pub struct PerformScore {
    pub title: Option<String>,
    pub global_tempo: u16,
    pub global_time: Option<(u8, u8)>,
    pub tracks: Vec<PerformTrack>,
}

#[derive(Debug, Clone)]
pub struct PerformTrack {
    pub name: String,
    pub instrument: Option<u8>,
    pub sections: Vec<PerformSection>,
}

#[derive(Debug, Clone)]
pub struct PerformSection {
    pub name: String,
    pub repeat: u8,
    pub measures: Vec<PerformMeasure>,
}

#[derive(Debug, Clone)]
pub struct PerformMeasure {
    pub events: Vec<PerfEvent>,
}

/// 播放事件。
#[derive(Debug, Clone)]
pub enum PerfEvent {
    /// 单音: midi, 时值, 力度
    Note {
        midi: u8,
        duration: PerfDuration,
        velocity: u8,
    },
    /// 休止: 时值
    Rest {
        duration: PerfDuration,
    },
    /// 和弦: midi 序列, 时值, 力度
    Chord {
        midis: Vec<u8>,
        duration: PerfDuration,
        velocity: u8,
    },
    /// 装饰音: midi, 时值, 力度
    Grace {
        midi: u8,
        duration: PerfDuration,
        velocity: u8,
    },
    /// 连音符: 比例, 事件列表
    Tuplet {
        ratio: (u8, u8),
        events: Vec<PerfEvent>,
    },
    /// 控制事件
    Control(PerfControl),
}

/// 播放时值。
#[derive(Debug, Clone, Copy)]
pub struct PerfDuration {
    pub base: u32,
    pub dotted: bool,
}

impl PerfDuration {
    /// 计算拍数（以全音符 = 1.0 为基准）。
    pub fn beats(&self) -> f64 {
        let base = self.base.max(1) as f64;
        let base = 4.0 / base;
        if self.dotted {
            base * 1.5
        } else {
            base
        }
    }

    /// 在指定 BPM 下转换为时间长度。
    pub fn to_duration(&self, bpm: u16) -> Duration {
        let beat_us = 60_000_000.0 / bpm as f64;
        let us = self.beats() * beat_us;
        Duration::from_micros(us as u64)
    }

    pub(crate) fn to_micros(&self, bpm: u16) -> u64 {
        let beat_us = 60_000_000.0 / bpm as f64;
        (self.beats() * beat_us) as u64
    }
}

/// 控制事件。
#[derive(Debug, Clone)]
pub enum PerfControl {
    Key { root: u8, scale_type: u8 },
    Tempo(u16),
    TimeSig { beats: u8, beat_value: u8 },
    PedalOn(u8),
    PedalOff(u8),
    Volume(u8),
    DynamicMark(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_beats() {
        let d = PerfDuration { base: 4, dotted: false };
        assert!((d.beats() - 1.0).abs() < 1e-10);

        let d = PerfDuration { base: 8, dotted: true };
        assert!((d.beats() - 0.75).abs() < 1e-10);

        let d = PerfDuration { base: 2, dotted: false };
        assert!((d.beats() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_duration_to_micros() {
        let d = PerfDuration { base: 4, dotted: false };
        // quarter note at 120 BPM = 0.5s = 500_000us
        assert_eq!(d.to_micros(120), 500_000);
    }
}
