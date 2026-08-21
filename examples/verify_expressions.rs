//! 验证表情记号播放效果 —— 读取已有 .bm 并 dump 调度事件
//!
//! 用法：cargo run --example verify_expressions
//! 依赖：examples/canon.bm（已存在）

use std::fs;

fn main() {
    // 读取已有的 .bm 文件
    let bytes = fs::read("examples/canon.bm").expect("无法读取 examples/canon.bm");
    let perf = perform::reader::read(&bytes).expect("读取 .bm 失败");

    println!(
        "标题: {} | BPM: {} | 轨数: {}",
        perf.title.as_deref().unwrap_or("(无标题)"),
        perf.global_tempo,
        perf.tracks.len()
    );
    println!();

    // 打印每个轨道的表达式控制事件
    for (track_idx, track) in perf.tracks.iter().enumerate() {
        println!("══════ 轨道 {} ── {} ══════", track_idx, track.name);
        let mut tempo = perf.global_tempo;
        let mut ctx = perform::player::PlayContext::new();

        for section in &track.sections {
            let repeat = if section.repeat > 0 { section.repeat as usize } else { 1 };
            for rep in 0..repeat {
                println!("\n  段落: \"{}\"{}", section.name, if repeat > 1 { format!(" (第 {} 遍)", rep + 1) } else { String::new() });
                for (midx, measure) in section.measures.iter().enumerate() {
                    for event in &measure.events {
                        match event {
                            perform::PerfEvent::Control(ctrl) => {
                                match ctrl {
                                    perform::PerfControl::Tempo(bpm) => {
                                        println!(
                                            "    m{midx}: @tempo({bpm}) → ctx.vol={} scale={:.2} fermata={:.1}",
                                            ctx.current_vol, ctx.tempo_scale, ctx.fermata_extend
                                        );
                                    }
                                    perform::PerfControl::DynamicMark(s) => {
                                        println!(
                                            "    m{midx}: @{s} → ctx.vol={} scale={:.2} fermata={:.1} cresc_target={:?}",
                                            ctx.current_vol, ctx.tempo_scale, ctx.fermata_extend, ctx.crescendo_target
                                        );
                                    }
                                    perform::PerfControl::Volume(v) => {
                                        println!(
                                            "    m{midx}: @vol({v}) → ctx.vol={v} cresc_target={:?}",
                                            ctx.crescendo_target
                                        );
                                    }
                                    perform::PerfControl::PedalOn(p) => {
                                        let cc = match p { 0 => 64, 1 => 67, 2 => 66, _ => 64 };
                                        println!("    m{midx}: @pedal(on CC#{cc})");
                                    }
                                    perform::PerfControl::PedalOff(p) => {
                                        let cc = match p { 0 => 64, 1 => 67, 2 => 66, _ => 64 };
                                        println!("    m{midx}: @pedal(off CC#{cc})");
                                    }
                                    perform::PerfControl::Key { .. } | perform::PerfControl::TimeSig { .. } => {}
                                }
                            }
                            perform::PerfEvent::Note { midi, duration, .. } => {
                                let dur_us = effective_dur(duration, tempo, ctx.tempo_scale, ctx.fermata_extend);
                                println!(
                                    "    m{midx}: Note#{midi} dur={dur_us}us (scale×{:.2} fermata×{:.1})",
                                    ctx.tempo_scale, ctx.fermata_extend
                                );
                            }
                            perform::PerfEvent::Chord { midis, duration, .. } => {
                                let dur_us = effective_dur(duration, tempo, ctx.tempo_scale, ctx.fermata_extend);
                                println!(
                                    "    m{midx}: Chord[{:?}] dur={dur_us}us (scale×{:.2} fermata×{:.1})",
                                    midis, ctx.tempo_scale, ctx.fermata_extend
                                );
                            }
                            perform::PerfEvent::Rest { duration } => {
                                let dur_us = effective_dur(duration, tempo, ctx.tempo_scale, ctx.fermata_extend);
                                println!("    m{midx}: Rest dur={dur_us}us");
                            }
                            perform::PerfEvent::Grace { midi, duration, .. } => {
                                let dur_us = effective_dur(duration, tempo, ctx.tempo_scale, ctx.fermata_extend) / 2;
                                println!("    m{midx}: Grace#{midi} dur={dur_us}us");
                            }
                            perform::PerfEvent::Tuplet { ratio, events: inner } => {
                                println!(
                                    "    m{midx}: Tuplet({}×{}) {} notes (scale×{:.2})",
                                    ratio.0, ratio.1, inner.len(), ratio.0 as f64 / ratio.1 as f64
                                );
                            }
                        }
                    }
                }
            }
        }
        println!();
    }

    // 完整 MIDI 事件调度（含插值点），仅展示前 400 条有 delay 的事件
    println!("═══════════════════════════════════════════════════");
    println!("完整 MIDI 调度事件（含 crescendo 插值点）");
    println!("═══════════════════════════════════════════════════");
    let events = build_full_schedule(&perf);
    let mut count = 0;
    for (delay, desc) in &events {
        if *delay > 0 {
            println!("  +{}us  {}", delay, desc);
            count += 1;
            if count >= 400 {
                println!("  ... (共 {} 条事件，已截断)", events.len());
                break;
            }
        }
    }
    println!("\n总计 {} 条事件（含 delay>0）", count);
}

fn effective_dur(d: &perform::PerfDuration, tempo: u16, tempo_scale: f64, fermata_extend: f64) -> u64 {
    let base_us = d.to_duration(tempo).as_micros() as f64;
    (base_us * tempo_scale * fermata_extend).round() as u64
}

fn build_full_schedule(score: &perform::PerformScore) -> Vec<(u64, String)> {
    let mut events = Vec::new();
    let mut tempo = score.global_tempo;

    for (track_idx, track) in score.tracks.iter().enumerate() {
        let ch = track_idx as u8;
        let mut ctx = perform::player::PlayContext::new();

        if let Some(inst) = track.instrument {
            events.push((0, format!("ProgramChange program={inst} ch={ch}")));
        }

        for section in &track.sections {
            let repeat = if section.repeat > 0 { section.repeat as usize } else { 1 };
            for _ in 0..repeat {
                for measure in &section.measures {
                    flatten_show(&mut events, &measure.events, ch, &mut tempo, &mut ctx);
                }
            }
        }

        events.push((0, format!("AllNotesOff ch={ch}")));
    }
    events
}

fn flatten_show(
    events: &mut Vec<(u64, String)>,
    perf_events: &[perform::PerfEvent],
    ch: u8,
    tempo: &mut u16,
    ctx: &mut perform::player::PlayContext,
) {
    for event in perf_events {
        match event {
            perform::PerfEvent::Note { midi, duration, velocity } => {
                let dur_us = effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend);
                events.push((0, format!("NoteOn ch={ch} #{} vel={velocity}", midi)));
                events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
            }
            perform::PerfEvent::Rest { duration } => {
                events.push((effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend), "NOP(rest)".into()));
            }
            perform::PerfEvent::Chord { midis, duration, velocity } => {
                let dur_us = effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend);
                for midi in midis {
                    events.push((0, format!("NoteOn ch={ch} #{} vel={velocity}", midi)));
                }
                for midi in midis {
                    events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
                }
            }
            perform::PerfEvent::Grace { midi, duration, velocity } => {
                let dur_us = effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) / 2;
                events.push((0, format!("NoteOn ch={ch} #{} vel={velocity} (grace)", midi)));
                events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
            }
            perform::PerfEvent::Tuplet { ratio, events: inner } => {
                let scale = ratio.0 as f64 / ratio.1 as f64;
                flatten_tuplet_show(events, inner, ch, tempo, scale, ctx);
            }
            perform::PerfEvent::Control(ctrl) => {
                apply_show(events, ctrl, ch, tempo, ctx);
            }
        }
    }
}

fn flatten_tuplet_show(
    events: &mut Vec<(u64, String)>,
    inner: &[perform::PerfEvent],
    ch: u8,
    tempo: &mut u16,
    scale: f64,
    ctx: &mut perform::player::PlayContext,
) {
    for event in inner {
        match event {
            perform::PerfEvent::Note { midi, duration, velocity } => {
                let dur_us = (effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64;
                events.push((0, format!("NoteOn ch={ch} #{} vel={velocity} (tuplet)", midi)));
                events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
            }
            perform::PerfEvent::Rest { duration } => {
                events.push(((effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64, "NOP".into()));
            }
            perform::PerfEvent::Control(ctrl) => {
                apply_show(events, ctrl, ch, tempo, ctx);
            }
            perform::PerfEvent::Grace { midi, duration, velocity } => {
                let dur_us = (effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64 / 2;
                events.push((0, format!("NoteOn ch={ch} #{} vel={velocity} (grace+tuplet)", midi)));
                events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
            }
            perform::PerfEvent::Chord { midis, duration, velocity } => {
                let dur_us = (effective_dur(duration, *tempo, ctx.tempo_scale, ctx.fermata_extend) as f64 / scale) as u64;
                for midi in midis {
                    events.push((0, format!("NoteOn ch={ch} #{} vel={velocity} (tuplet chord)", midi)));
                }
                for midi in midis {
                    events.push((dur_us, format!("NoteOff ch={ch} #{}", midi)));
                }
            }
            perform::PerfEvent::Tuplet { ratio: ir, events: ie } => {
                let is = scale * (ir.0 as f64 / ir.1 as f64);
                flatten_tuplet_show(events, ie, ch, tempo, is, ctx);
            }
        }
    }
}

fn apply_show(
    events: &mut Vec<(u64, String)>,
    ctrl: &perform::PerfControl,
    ch: u8,
    tempo: &mut u16,
    ctx: &mut perform::player::PlayContext,
) {
    match ctrl {
        perform::PerfControl::Tempo(bpm) => {
            *tempo = *bpm;
            ctx.steps_since_last_ctrl = 0;
            events.push((0, format!("Tempo→{bpm} (ctx.vol={} scale={:.2})", ctx.current_vol, ctx.tempo_scale)));
        }
        perform::PerfControl::PedalOn(p) => {
            let cc = match p { 0 => 64, 1 => 67, 2 => 66, _ => 64 };
            emit_interp(events, ch, ctx);
            events.push((0, format!("CC#{cc}=127 (pedal on)")));
            ctx.steps_since_last_ctrl += 1;
        }
        perform::PerfControl::PedalOff(p) => {
            let cc = match p { 0 => 64, 1 => 67, 2 => 66, _ => 64 };
            emit_interp(events, ch, ctx);
            events.push((0, format!("CC#{cc}=0 (pedal off)")));
            ctx.steps_since_last_ctrl += 1;
        }
        perform::PerfControl::Volume(v) => {
            emit_interp(events, ch, ctx);
            ctx.current_vol = *v;
            ctx.crescendo_target = None;
            ctx.steps_since_last_ctrl = 0;
            events.push((0, format!("CC#7={v} (vol explicit)")));
        }
        perform::PerfControl::DynamicMark(s) => {
            emit_interp(events, ch, ctx);
            match s.as_str() {
                "cresc" | "crescendo" => {
                    ctx.crescendo_target = Some(112);
                    ctx.steps_since_last_ctrl = 0;
                    events.push((0, format!("@cresc target=ff(112) start_vol={}", ctx.current_vol)));
                }
                "decresc" | "decrescendo" | "diminuendo" => {
                    ctx.crescendo_target = Some(48);
                    ctx.steps_since_last_ctrl = 0;
                    events.push((0, format!("@decresc target=p(48) start_vol={}", ctx.current_vol)));
                }
                "rit" | "ritardando" | "ritard." => {
                    ctx.tempo_scale = (ctx.tempo_scale * 1.3).min(3.0);
                    ctx.steps_since_last_ctrl = 0;
                    events.push((0, format!("@rit tempo_scale→{:.2}", ctx.tempo_scale)));
                }
                "accel" | "accelerando" => {
                    ctx.tempo_scale = (ctx.tempo_scale * 0.75).max(0.4);
                    ctx.steps_since_last_ctrl = 0;
                    events.push((0, format!("@accel tempo_scale→{:.2}", ctx.tempo_scale)));
                }
                "fermata" => {
                    ctx.fermata_extend = 2.5;
                    ctx.steps_since_last_ctrl = 0;
                    events.push((0, format!("@fermata fermata_extend→2.5x")));
                }
                other => {
                    if let Some(vol) = static_vol(other) {
                        emit_interp(events, ch, ctx);
                        ctx.current_vol = vol;
                        ctx.crescendo_target = None;
                        ctx.steps_since_last_ctrl = 0;
                        events.push((0, format!("@dyn({other}) CC#7={vol}")));
                    }
                }
            }
        }
        perform::PerfControl::Key { .. } | perform::PerfControl::TimeSig { .. } => {}
    }
}

fn static_vol(s: &str) -> Option<u8> {
    match s {
        "ppp" => Some(16), "pp" => Some(32), "p" => Some(48), "mp" => Some(64),
        "mf" => Some(80), "f" => Some(96), "ff" => Some(112), "fff" => Some(120),
        _ => None,
    }
}

fn emit_interp(events: &mut Vec<(u64, String)>, ch: u8, ctx: &mut perform::player::PlayContext) {
    if let Some(target) = ctx.crescendo_target {
        let steps = ctx.steps_since_last_ctrl.max(1) as f64;
        let progress = (steps - 1.0).max(0.0) / steps;
        let vol = ctx.current_vol as f64 + (target as f64 - ctx.current_vol as f64) * progress;
        let vol = vol.round() as u8;
        events.push((0, format!("CC#7={vol} (interpolated prog={progress:.2} target={target})")));
    }
}
