//! MIDI 播放器 — 将 `PerformScore` 发送到 MIDI 输出端口。
//!
//! 使用 `midir` 库进行 MIDI 输出。
//!
//! 表情记号处理：
//! - `cresc`/`decresc`：在当前音量与目标音量之间线性插值，生成 CC#7 渐变消息
//! - `rit`/`accel`：缩放后续所有事件的时值（通过 tempo multiplier 实现）
//! - `fermata`：延长当前音符的 NoteOff 延迟（默认 ×2.5）

use crate::{PerfControl, PerfEvent, PerformScore};
use std::thread;
use std::time::Duration as StdDuration;

/// 播放错误。
#[derive(Debug)]
pub enum PlayError {
    NoMidiOutput,
    ConnectFailed(String),
    SendFailed(String),
}

impl std::fmt::Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMidiOutput => write!(f, "no MIDI output available"),
            Self::ConnectFailed(e) => write!(f, "MIDI connect failed: {}", e),
            Self::SendFailed(e) => write!(f, "MIDI send failed: {}", e),
        }
    }
}

impl std::error::Error for PlayError {}

struct MidiEvent {
    delay_us: u64,
    msg: MidiMessage,
}

enum MidiMessage {
    NoteOn { ch: u8, note: u8, vel: u8 },
    NoteOff { ch: u8, note: u8 },
    ProgramChange { ch: u8, program: u8 },
    ControlChange { ch: u8, cc: u8, val: u8 },
    NOP,
}

/// 播放上下文 —— 追踪需要跨事件传播的状态。
#[derive(Debug, Clone)]
pub struct PlayContext {
    /// 当前 CC#7 主音量（0–127）。
    pub current_vol: u8,
    /// 正在进行的渐强/渐弱目标音量（None 表示无活动渐变）。
    pub crescendo_target: Option<u8>,
    /// 上一控制事件与本次之间的步数，用于计算渐变插值位置。
    pub steps_since_last_ctrl: u32,
    /// tempo 缩放因子（rit/accel 累积叠加）。
    pub tempo_scale: f64,
    /// fermata 延长倍数（0 表示无延音）。
    pub fermata_extend: f64,
}

impl PlayContext {
    pub fn new() -> Self {
        Self {
            current_vol: 100,
            crescendo_target: None,
            steps_since_last_ctrl: 0,
            tempo_scale: 1.0,
            fermata_extend: 1.0,
        }
    }

    /// 重置渐变状态（在 track 或 section 边界处调用）。
    pub fn reset_crescendo(&mut self) {
        self.crescendo_target = None;
        self.steps_since_last_ctrl = 0;
    }
}

impl Default for PlayContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 列出可用的 MIDI 输出端口。
pub fn list_ports() -> Vec<String> {
    let midi_out = match midir::MidiOutput::new("kyma-perform") {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    midi_out
        .ports()
        .iter()
        .map(|p| midi_out.port_name(p).unwrap_or_default())
        .collect()
}

/// 播放 `PerformScore`。
///
/// `port_index` 指定 MIDI 输出端口（`None` 使用第一个可用端口）。
pub fn play(score: &PerformScore, port_index: Option<usize>) -> Result<(), PlayError> {
    let midi_out = midir::MidiOutput::new("kyma-perform")
        .map_err(|e| PlayError::ConnectFailed(e.to_string()))?;

    let ports = midi_out.ports();
    if ports.is_empty() {
        return Err(PlayError::NoMidiOutput);
    }

    let idx = port_index.unwrap_or(0);
    if idx >= ports.len() {
        return Err(PlayError::ConnectFailed(format!(
            "port index {} out of range ({} ports)",
            idx,
            ports.len()
        )));
    }

    let mut conn = midi_out
        .connect(&ports[idx], "kyma-play")
        .map_err(|e| PlayError::ConnectFailed(e.to_string()))?;

    let schedule = build_schedule(score);

    let mut send_errs: Vec<String> = Vec::new();
    for event in &schedule {
        if event.delay_us > 0 {
            thread::sleep(StdDuration::from_micros(event.delay_us));
        }
        match &event.msg {
            MidiMessage::NoteOn { ch, note, vel } => {
                if let Err(e) = conn.send(&[0x90 | ch, *note, *vel]) {
                    send_errs.push(format!("NoteOn ch={ch} note={note}: {e}"));
                }
            }
            MidiMessage::NoteOff { ch, note } => {
                if let Err(e) = conn.send(&[0x80 | ch, *note, 0]) {
                    send_errs.push(format!("NoteOff ch={ch} note={note}: {e}"));
                }
            }
            MidiMessage::ProgramChange { ch, program } => {
                if let Err(e) = conn.send(&[0xC0 | ch, *program]) {
                    send_errs.push(format!("ProgramChange ch={ch}: {e}"));
                }
            }
            MidiMessage::ControlChange { ch, cc, val } => {
                if let Err(e) = conn.send(&[0xB0 | ch, *cc, *val]) {
                    send_errs.push(format!("ControlChange ch={ch} cc={cc}: {e}"));
                }
            }
            MidiMessage::NOP => {}
        }
    }

    if send_errs.is_empty() {
        Ok(())
    } else {
        Err(PlayError::SendFailed(format!(
            "{} MIDI send error(s) occurred",
            send_errs.len()
        )))
    }
}

fn build_schedule(score: &PerformScore) -> Vec<MidiEvent> {
    let mut events = Vec::new();
    let mut tempo = score.global_tempo;

    for (track_idx, track) in score.tracks.iter().enumerate() {
        let ch = track_idx as u8;
        let mut ctx = PlayContext::new();

        if let Some(inst) = track.instrument {
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ProgramChange {
                    ch,
                    program: inst,
                },
            });
        }

        for section in &track.sections {
            for _ in 0..(if section.repeat > 0 { section.repeat as usize } else { 1 }) {
                for measure in &section.measures {
                    flatten_measure(&mut events, &measure.events, ch, &mut tempo, &mut ctx);
                }
            }
        }

        all_notes_off(&mut events, ch);
    }

    events
}

fn flatten_measure(
    events: &mut Vec<MidiEvent>,
    perf_events: &[PerfEvent],
    ch: u8,
    tempo: &mut u16,
    ctx: &mut PlayContext,
) {
    for event in perf_events {
        match event {
            PerfEvent::Note {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend);
                events.push(MidiEvent {
                    delay_us: 0,
                    msg: MidiMessage::NoteOn {
                        ch,
                        note: *midi,
                        vel: *velocity,
                    },
                });
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NoteOff { ch, note: *midi },
                });
            }
            PerfEvent::Rest { duration } => {
                let dur_us = effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend);
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NOP,
                });
            }
            PerfEvent::Chord {
                midis,
                duration,
                velocity,
            } => {
                let dur_us = effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend);
                for midi in midis {
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NoteOn {
                            ch,
                            note: *midi,
                            vel: *velocity,
                        },
                    });
                }
                for midi in midis {
                    events.push(MidiEvent {
                        delay_us: dur_us,
                        msg: MidiMessage::NoteOff { ch, note: *midi },
                    });
                }
            }
            PerfEvent::Grace {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) / 2;
                events.push(MidiEvent {
                    delay_us: 0,
                    msg: MidiMessage::NoteOn {
                        ch,
                        note: *midi,
                        vel: *velocity,
                    },
                });
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NoteOff { ch, note: *midi },
                });
            }
            PerfEvent::Tuplet { ratio, events: inner } => {
                let scale = ratio.0 as f64 / ratio.1 as f64;
                flatten_tuplet(events, inner, ch, tempo, scale, ctx);
            }
            PerfEvent::Control(ctrl) => {
                apply_control(events, ctrl, ch, tempo, ctx);
            }
        }
    }
}

fn flatten_tuplet(
    events: &mut Vec<MidiEvent>,
    inner: &[PerfEvent],
    ch: u8,
    tempo: &mut u16,
    scale: f64,
    ctx: &mut PlayContext,
) {
    for event in inner {
        match event {
            PerfEvent::Note {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = (effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64;
                events.push(MidiEvent {
                    delay_us: 0,
                    msg: MidiMessage::NoteOn {
                        ch,
                        note: *midi,
                        vel: *velocity,
                    },
                });
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NoteOff { ch, note: *midi },
                });
            }
            PerfEvent::Rest { duration } => {
                let dur_us = (effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64;
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NOP,
                });
            }
            PerfEvent::Control(ctrl) => {
                apply_control(events, ctrl, ch, tempo, ctx);
            }
            PerfEvent::Grace {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = (effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64 / 2;
                events.push(MidiEvent {
                    delay_us: 0,
                    msg: MidiMessage::NoteOn {
                        ch,
                        note: *midi,
                        vel: *velocity,
                    },
                });
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NoteOff { ch, note: *midi },
                });
            }
            PerfEvent::Chord {
                midis,
                duration,
                velocity,
            } => {
                let dur_us = (effective_duration(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64;
                for midi in midis {
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NoteOn {
                            ch,
                            note: *midi,
                            vel: *velocity,
                        },
                    });
                }
                for midi in midis {
                    events.push(MidiEvent {
                        delay_us: dur_us,
                        msg: MidiMessage::NoteOff { ch, note: *midi },
                    });
                }
            }
            PerfEvent::Tuplet {
                ratio: inner_ratio,
                events: inner_events,
            } => {
                let inner_scale = scale * (inner_ratio.0 as f64 / inner_ratio.1 as f64);
                flatten_tuplet(events, inner_events, ch, tempo, inner_scale, ctx);
            }
        }
    }
}

/// 计算有效时值（us），考虑 tempo 缩放和 fermata 延长。
fn effective_duration(
    duration: &crate::PerfDuration,
    tempo: u16,
    tempo_scale: f64,
    fermata_extend: f64,
) -> u64 {
    let base_us = duration.to_micros(tempo) as f64;
    // tempo_scale > 1 表示 rit（变慢），< 1 表示 accel（变快）
    (base_us * tempo_scale * fermata_extend).round() as u64
}

fn apply_control(
    events: &mut Vec<MidiEvent>,
    ctrl: &PerfControl,
    ch: u8,
    tempo: &mut u16,
    ctx: &mut PlayContext,
) {
    match ctrl {
        PerfControl::Tempo(bpm) => {
            *tempo = *bpm;
            ctx.steps_since_last_ctrl = 0;
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::NOP,
            });
        }
        PerfControl::PedalOn(p) => {
            let cc = pedal_cc(*p);
            emit_crescendo_interpolation(events, ch, ctx);
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange {
                    ch,
                    cc,
                    val: 127,
                },
            });
            ctx.steps_since_last_ctrl += 1;
        }
        PerfControl::PedalOff(p) => {
            let cc = pedal_cc(*p);
            emit_crescendo_interpolation(events, ch, ctx);
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange { ch, cc, val: 0 },
            });
            ctx.steps_since_last_ctrl += 1;
        }
        PerfControl::Volume(v) => {
            emit_crescendo_interpolation(events, ch, ctx);
            ctx.current_vol = *v;
            ctx.crescendo_target = None;
            ctx.steps_since_last_ctrl = 0;
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange {
                    ch,
                    cc: 7,
                    val: *v,
                },
            });
        }
        PerfControl::DynamicMark(s) => {
            emit_crescendo_interpolation(events, ch, ctx);
            match s.as_str() {
                "cresc" | "crescendo" => {
                    // 渐强到 ff（112）
                    let target = 112u8;
                    ctx.crescendo_target = Some(target);
                    ctx.steps_since_last_ctrl = 0;
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NOP,
                    });
                }
                "decresc" | "decrescendo" | "diminuendo" => {
                    // 渐弱到 p（48）
                    let target = 48u8;
                    ctx.crescendo_target = Some(target);
                    ctx.steps_since_last_ctrl = 0;
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NOP,
                    });
                }
                "rit" | "ritardando" | "ritard." => {
                    // 变慢 1.3×（逐步累积）
                    ctx.tempo_scale = (ctx.tempo_scale * 1.3).min(3.0);
                    ctx.steps_since_last_ctrl = 0;
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NOP,
                    });
                }
                "accel" | "accelerando" => {
                    // 变快 0.75×（逐步累积）
                    ctx.tempo_scale = (ctx.tempo_scale * 0.75).max(0.4);
                    ctx.steps_since_last_ctrl = 0;
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NOP,
                    });
                }
                "fermata" => {
                    // 延长当前音符 2.5×；下一个音符的 NoteOff 延迟乘以 fermata_extend
                    ctx.fermata_extend = 2.5;
                    ctx.steps_since_last_ctrl = 0;
                    events.push(MidiEvent {
                        delay_us: 0,
                        msg: MidiMessage::NOP,
                    });
                }
                // 静态力度：直接设置 CC#7
                "ppp" => { ctx.current_vol = 16;  ctx.crescendo_target = None; }
                "pp"  => { ctx.current_vol = 32;  ctx.crescendo_target = None; }
                "p"   => { ctx.current_vol = 48;  ctx.crescendo_target = None; }
                "mp"  => { ctx.current_vol = 64;  ctx.crescendo_target = None; }
                "mf"  => { ctx.current_vol = 80;  ctx.crescendo_target = None; }
                "f"   => { ctx.current_vol = 96;  ctx.crescendo_target = None; }
                "ff"  => { ctx.current_vol = 112; ctx.crescendo_target = None; }
                "fff" => { ctx.current_vol = 120; ctx.crescendo_target = None; }
                _ => {}
            }
            if ctx.crescendo_target.is_none() {
                // 静态力度或无可识别记号：发送 CC#7
                events.push(MidiEvent {
                    delay_us: 0,
                    msg: MidiMessage::ControlChange {
                        ch,
                        cc: 7,
                        val: ctx.current_vol,
                    },
                });
                ctx.steps_since_last_ctrl = 0;
            } else {
                ctx.steps_since_last_ctrl += 1;
            }
        }
        // Key 和 TimeSig 是元数据，不产生 MIDI 消息。
        PerfControl::Key { .. } | PerfControl::TimeSig { .. } => {
            // 控制元数据不中断渐变的步骤计数
        }
    }
}

/// 如果存在进行中的 crescendo/decrescendo，在此插入插值点。
///
/// 策略：从 `current_vol` 到 `crescendo_target` 做线性插值，
/// 在每次控制事件处输出一个中间值，使渐变平滑。
fn emit_crescendo_interpolation(events: &mut Vec<MidiEvent>, ch: u8, ctx: &mut PlayContext) {
    if let Some(target) = ctx.crescendo_target {
        let steps = ctx.steps_since_last_ctrl.max(1) as f64;
        let progress = (steps - 1.0).max(0.0) / steps;
        let interpolated = lerp(ctx.current_vol as f64, target as f64, progress) as u8;
        events.push(MidiEvent {
            delay_us: 0,
            msg: MidiMessage::ControlChange {
                ch,
                cc: 7,
                val: interpolated,
            },
        });
    }
}

/// 线性插值：a → b，t ∈ [0, 1]。
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn pedal_cc(p: u8) -> u8 {
    match p {
        0 => 64,
        1 => 67,
        2 => 66,
        _ => 64,
    }
}

fn all_notes_off(events: &mut Vec<MidiEvent>, ch: u8) {
    events.push(MidiEvent {
        delay_us: 0,
        msg: MidiMessage::ControlChange {
            ch,
            cc: 123,
            val: 0,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_dynamics_set_vol() {
        let mut ctx = PlayContext::new();
        ctx.current_vol = 80;
        for (mark, expected) in [
            ("ppp", 16), ("pp", 32), ("p", 48), ("mp", 64),
            ("mf", 80), ("f", 96), ("ff", 112), ("fff", 120),
        ] {
            ctx.current_vol = 80;
            ctx.crescendo_target = None;
            apply_control_dummy(&mut ctx, mark);
            assert_eq!(ctx.current_vol, expected, "static dynamic '{}' failed", mark);
            assert!(ctx.crescendo_target.is_none(), "static dynamic '{}' should not start crescendo", mark);
        }
    }

    #[test]
    fn test_expression_marks_start_crescendo_or_tempo() {
        let mut ctx = PlayContext::new();
        ctx.current_vol = 80;

        // cresc 启动渐变
        apply_control_dummy(&mut ctx, "cresc");
        assert!(ctx.crescendo_target.is_some());
        assert_eq!(ctx.crescendo_target.unwrap(), 112);

        // decresc 启动渐变
        let mut ctx2 = PlayContext::new();
        apply_control_dummy(&mut ctx2, "decresc");
        assert!(ctx2.crescendo_target.is_some());
        assert_eq!(ctx2.crescendo_target.unwrap(), 48);

        // rit / accel 不影响音量
        let mut ctx3 = PlayContext::new();
        ctx3.current_vol = 80;
        apply_control_dummy(&mut ctx3, "rit");
        assert_eq!(ctx3.current_vol, 80);
        assert!(ctx3.crescendo_target.is_none());

        let mut ctx4 = PlayContext::new();
        apply_control_dummy(&mut ctx4, "accel");
        assert_eq!(ctx4.current_vol, 100);
        assert!(ctx4.crescendo_target.is_none());

        // fermata 不影响音量
        let mut ctx5 = PlayContext::new();
        apply_control_dummy(&mut ctx5, "fermata");
        assert_eq!(ctx5.current_vol, 100);
        assert!(ctx5.crescendo_target.is_none());
    }

    #[test]
    fn test_crescendo_context_flow() {
        let mut ctx = PlayContext::new();
        // 启动 cresc，从默认 vol(100) 到 ff(112)
        apply_control_dummy(&mut ctx, "cresc");
        assert_eq!(ctx.crescendo_target, Some(112));
        assert_eq!(ctx.current_vol, 100);
        assert_eq!(ctx.steps_since_last_ctrl, 1);

        // 第一次插值（紧接 cresc 之后，step=1 → progress=0/1=0 → vol=100）
        emit_crescendo_interpolation(&mut vec![], 0, &mut ctx);
        assert_eq!(ctx.steps_since_last_ctrl, 1);

        // 模拟第二个控制事件到来（例如 pedal off），它先触发插值，再递增
        emit_crescendo_interpolation(&mut vec![], 0, &mut ctx);
        assert_eq!(ctx.steps_since_last_ctrl, 1); // 插值本身不递增
        // 模拟该控制事件内部递增
        ctx.steps_since_last_ctrl += 1;
        assert_eq!(ctx.steps_since_last_ctrl, 2);

        // 第三个控制事件：progress=1/2=0.5 → vol=106
        emit_crescendo_interpolation(&mut vec![], 0, &mut ctx);
        assert_eq!(ctx.steps_since_last_ctrl, 2);
    }

    #[test]
    fn test_rit_accumulates() {
        let mut ctx = PlayContext::new();
        ctx.tempo_scale = 1.0;
        apply_control_dummy(&mut ctx, "rit");
        assert!((ctx.tempo_scale - 1.3).abs() < 1e-6);
        apply_control_dummy(&mut ctx, "rit");
        assert!((ctx.tempo_scale - 1.69).abs() < 1e-2);
    }

    #[test]
    fn test_accel_accumulates() {
        let mut ctx = PlayContext::new();
        ctx.tempo_scale = 1.0;
        apply_control_dummy(&mut ctx, "accel");
        assert!((ctx.tempo_scale - 0.75).abs() < 1e-6);
        apply_control_dummy(&mut ctx, "accel");
        assert!((ctx.tempo_scale - 0.5625).abs() < 1e-4);
    }

    #[test]
    fn test_fermata_extend() {
        let mut ctx = PlayContext::new();
        apply_control_dummy(&mut ctx, "fermata");
        assert!((ctx.fermata_extend - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_effective_duration_with_rit() {
        let d = crate::PerfDuration { base: 4, dotted: false };
        let base = d.to_micros(120);
        assert_eq!(base, 500_000);

        let eff = effective_duration(&d, 120, 1.5, 1.0);
        assert_eq!(eff, (base as f64 * 1.5) as u64);
    }

    #[test]
    fn test_effective_duration_with_fermata() {
        let d = crate::PerfDuration { base: 4, dotted: false };
        let base = d.to_micros(120); // 500_000

        let eff = effective_duration(&d, 120, 1.0, 2.5);
        assert_eq!(eff, (base as f64 * 2.5) as u64);
    }

    fn apply_control_dummy(ctx: &mut PlayContext, mark: &str) {
        let ctrl = PerfControl::DynamicMark(mark.to_string());
        let mut events = Vec::new();
        let mut tempo = 120u16;
        apply_control(&mut events, &ctrl, 0, &mut tempo, ctx);
    }
}
