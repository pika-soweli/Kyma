//! MIDI 播放器 — 将 `PerformScore` 发送到 MIDI 输出端口。
//!
//! 使用 `midir` 库进行 MIDI 输出。

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

    for event in &schedule {
        if event.delay_us > 0 {
            thread::sleep(StdDuration::from_micros(event.delay_us));
        }
        match &event.msg {
            MidiMessage::NoteOn { ch, note, vel } => {
                let _ = conn.send(&[0x90 | ch, *note, *vel]);
            }
            MidiMessage::NoteOff { ch, note } => {
                let _ = conn.send(&[0x80 | ch, *note, 0]);
            }
            MidiMessage::ProgramChange { ch, program } => {
                let _ = conn.send(&[0xC0 | ch, *program]);
            }
            MidiMessage::ControlChange { ch, cc, val } => {
                let _ = conn.send(&[0xB0 | ch, *cc, *val]);
            }
            MidiMessage::NOP => {}
        }
    }

    Ok(())
}

fn build_schedule(score: &PerformScore) -> Vec<MidiEvent> {
    let mut events = Vec::new();
    let mut tempo = score.global_tempo;

    for (track_idx, track) in score.tracks.iter().enumerate() {
        let ch = track_idx as u8;

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
            let repeat = if section.repeat > 0 {
                section.repeat as usize
            } else {
                1
            };

            for _ in 0..repeat {
                for measure in &section.measures {
                    flatten_measure(&mut events, &measure.events, ch, &mut tempo);
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
) {
    for event in perf_events {
        match event {
            PerfEvent::Note {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = duration.to_micros(*tempo);
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
                events.push(MidiEvent {
                    delay_us: duration.to_micros(*tempo),
                    msg: MidiMessage::NOP,
                });
            }
            PerfEvent::Chord {
                midis,
                duration,
                velocity,
            } => {
                let dur_us = duration.to_micros(*tempo);
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
                        delay_us: if *midi == midis[0] { dur_us } else { 0 },
                        msg: MidiMessage::NoteOff { ch, note: *midi },
                    });
                }
            }
            PerfEvent::Grace {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = duration.to_micros(*tempo) / 2;
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
                flatten_tuplet(events, inner, ch, tempo, scale);
            }
            PerfEvent::Control(ctrl) => {
                apply_control(events, ctrl, ch, tempo);
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
) {
    for event in inner {
        match event {
            PerfEvent::Note {
                midi,
                duration,
                velocity,
            } => {
                let dur_us = (duration.to_micros(*tempo) as f64 / scale) as u64;
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
                let dur_us = (duration.to_micros(*tempo) as f64 / scale) as u64;
                events.push(MidiEvent {
                    delay_us: dur_us,
                    msg: MidiMessage::NOP,
                });
            }
            PerfEvent::Control(ctrl) => {
                apply_control(events, ctrl, ch, tempo);
            }
            _ => {}
        }
    }
}

fn apply_control(
    events: &mut Vec<MidiEvent>,
    ctrl: &PerfControl,
    ch: u8,
    tempo: &mut u16,
) {
    match ctrl {
        PerfControl::Tempo(bpm) => {
            *tempo = *bpm;
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::NOP,
            });
        }
        PerfControl::PedalOn(p) => {
            let cc = pedal_cc(*p);
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange {
                    ch,
                    cc,
                    val: 127,
                },
            });
        }
        PerfControl::PedalOff(p) => {
            let cc = pedal_cc(*p);
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange { ch, cc, val: 0 },
            });
        }
        PerfControl::Volume(v) => {
            events.push(MidiEvent {
                delay_us: 0,
                msg: MidiMessage::ControlChange {
                    ch,
                    cc: 7,
                    val: *v,
                },
            });
        }
        _ => {}
    }
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
