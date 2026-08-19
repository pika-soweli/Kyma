//! kyma — toki-musi 编译器顶层 CLI
//!
//! 子命令：
//!   compile  input.tm [-o output.bm]     编译 .tm → .bm
//!   perform  input.bm [--port N]         读取 .bm 并通过 MIDI 播放
//!   show     input.tm [-f text|bm]       文本渲染乐谱
//!   help                                 显示帮助信息
//!
//! 用法：
//!   kyma <subcommand> [args...]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";

fn print_usage() {
    eprintln!("kyma v{VERSION} — toki-musi compiler & score tools");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  kyma compile <input.tm> [-o output.bm]       Compile to .bm binary");
    eprintln!("  kyma perform <input.bm> [--port N]           Play .bm via MIDI");
    eprintln!("  kyma ports                                   List MIDI output ports");
    eprintln!("  kyma show    <input.tm> [-f text|bm]         Render score to text");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  -h, --help     Print this help");
    eprintln!("  -V, --version  Print version");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  kyma compile examples/canon.tm");
    eprintln!("  kyma perform examples/canon.bm");
    eprintln!("  kyma show    examples/canon.tm -f text");
}

// ── compile: .tm → .bm ──────────────────────────────────

fn cmd_compile(input_path: &str, output_override: Option<&str>) -> ExitCode {
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    let bytes = match opus::compile(&source) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: compile failed: {}", e);
            return ExitCode::from(1);
        }
    };

    let output_path = match output_override {
        Some(p) => PathBuf::from(p),
        None => {
            let p = PathBuf::from(input_path);
            let stem = p.file_stem().unwrap_or_default().to_string_lossy();
            let mut out = p.clone();
            out.set_file_name(format!("{stem}.bm"));
            out
        }
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    match fs::write(&output_path, &bytes) {
        Ok(_) => {
            eprintln!(
                "ok: {} ({} bytes) → {} ({} bytes)",
                input_path,
                source.len(),
                output_path.display(),
                bytes.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: cannot write '{}': {}", output_path.display(), e);
            ExitCode::from(1)
        }
    }
}

// ── perform: .bm → MIDI playback ────────────────────────

fn cmd_perform(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: perform requires a .bm file path");
        eprintln!("usage: kyma perform <input.bm> [--port N]");
        return ExitCode::from(1);
    }

    let input_path = &args[0];
    let port_index = parse_port_flag(&args[1..]);

    let bytes = match fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    let score = match perform::reader::read(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read .bm: {}", e);
            return ExitCode::from(1);
        }
    };

    if let Some(ref title) = score.title {
        eprintln!("playing: {} (tempo: {} BPM)", title, score.global_tempo);
    } else {
        eprintln!("playing: (untitled) (tempo: {} BPM)", score.global_tempo);
    }

    match perform::player::play(&score, port_index) {
        Ok(_) => {
            eprintln!("done.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn parse_port_flag(args: &[String]) -> Option<usize> {
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
    }
    None
}

fn cmd_ports() -> ExitCode {
    let ports = perform::player::list_ports();
    if ports.is_empty() {
        eprintln!("no MIDI output ports found");
        return ExitCode::from(1);
    }
    for (i, name) in ports.iter().enumerate() {
        println!("  [{}] {}", i, name);
    }
    ExitCode::SUCCESS
}

// ── transpose ───────────────────────────────────────────

fn cmd_transpose(input_path: &str, semitones: i8) -> ExitCode {
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    let mut score = match opus::compile_to_score(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: compile failed: {}", e);
            return ExitCode::from(1);
        }
    };

    score.transpose(semitones);

    // Render transposed score to stdout
    for (tid, track) in score.tracks.iter().enumerate() {
        println!("\n--- Track #{} ({}) ---", tid, track.name);
        for (sid, section) in track.sections.iter().enumerate() {
            print!("[S{} ", sid);
            if let Some(repeat) = section.repeat_times {
                print!("repeat {} ", repeat);
            }
            print!("\"{}\" ]", section.name);
            println!();
            for (k, measure) in section.measures.iter().enumerate() {
                let event_summary: Vec<String> = measure
                    .events
                    .iter()
                    .map(format_event)
                    .collect();
                println!("      m{}: {}", k, event_summary.join("  "));
            }
        }
    }

    println!("-- Transposed by {} semitone(s) --", if semitones < 0 { "-" } else { "" });
    if semitones < 0 {
        println!("{} semitones", -semitones);
    } else {
        println!("{} semitones", semitones);
    }
    ExitCode::SUCCESS
}

// ── show: text render ───────────────────────────────────

fn cmd_show(input_path: &str, format: &str) -> ExitCode {
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    let score = match opus::compile_to_score(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: compile failed: {}", e);
            return ExitCode::from(1);
        }
    };

    if format == "bm" {
        return render_score(&score);
    }

    // text 模式：逐轨逐小节打印
    println!("title:  {}", score.title.as_deref().unwrap_or("(none)"));
    println!(
        "key:    {}",
        score
            .global_key
            .as_ref()
            .map(|k| k.display())
            .unwrap_or_else(|| "(none)".into())
    );
    println!(
        "tempo:  {}",
        score
            .global_tempo
            .as_ref()
            .map(|t| format!("{} BPM", t.bpm()))
            .unwrap_or_else(|| "(none)".into())
    );
    println!(
        "time:   {}",
        score
            .global_time
            .as_ref()
            .map(|t| format!("{}/{}", t.beats_per_bar, t.beat_value))
            .unwrap_or_else(|| "(none)".into())
    );
    println!();

    for track in &score.tracks {
        println!(
            "── {} [{}] ──────────────────────────",
            track.name,
            track
                .instrument
                .as_ref()
                .map(|i| i.display_name())
                .unwrap_or("unnamed")
        );

        for section in &track.sections {
            let repeat = section
                .repeat_times
                .map(|r| format!(" (×{r})"))
                .unwrap_or_default();
            println!("  section \"{}\"{}", section.name, repeat);

            for (k, measure) in section.measures.iter().enumerate() {
                let parts: Vec<String> = measure
                    .events
                    .iter()
                    .map(format_event)
                    .collect();
                println!("    m{k}: {}", parts.join("  "));
            }
        }
        println!();
    }

    ExitCode::SUCCESS
}

fn p_as_str(p: &sonus::PedalKind) -> &str {
    match p {
        sonus::PedalKind::Sustain => "sustain",
        sonus::PedalKind::Soft => "soft",
        sonus::PedalKind::Sostenuto => "sostenuto",
    }
}

/// 统一格式化 MeasureEvent，覆盖所有 Cycle 1 变体。
fn format_event(e: &sonus::MeasureEvent) -> String {
    match e {
        sonus::MeasureEvent::Note(n) => n.display(),
        sonus::MeasureEvent::Chord(c) => c.display(),
        sonus::MeasureEvent::Control(ctrl) => format_control(ctrl),
        sonus::MeasureEvent::Tuplet(t) => {
            let inner: Vec<String> = t.events.iter().map(format_event).collect();
            format!("{}:{}{{{}}}", t.ratio.0, t.ratio.1, inner.join(" "))
        }
        sonus::MeasureEvent::Grace(n) => format!("grace({})", n.display()),
    }
}

/// 格式化局部控制事件。
fn format_control(ctrl: &sonus::LocalControl) -> String {
    match ctrl {
        sonus::LocalControl::LocalKey(k) => format!("@key({})", k.display()),
        sonus::LocalControl::LocalTempo(t) => format!("@tempo({})", t.bpm()),
        sonus::LocalControl::LocalTime(ts) => {
            format!("@time({}/{})", ts.beats_per_bar, ts.beat_value)
        }
        sonus::LocalControl::PedalOn(p) => format!("@pedal({} on)", p_as_str(p)),
        sonus::LocalControl::PedalOff(p) => format!("@pedal({} off)", p_as_str(p)),
        sonus::LocalControl::Volume(v) => format!("@vol({v})"),
        sonus::LocalControl::DynamicMark(d) => format!("@{d}"),
    }
}

fn render_score(score: &sonus::Score) -> ExitCode {
    println!("── Score ──────────────────────────");
    if let Some(ref title) = score.title {
        println!("  title:  {title}");
    }
    if let Some(ref key) = score.global_key {
        println!("  key:    {}", key.display());
    }
    if let Some(ref tempo) = score.global_tempo {
        println!("  tempo:  {}", tempo.display());
    }
    if let Some(ref time) = score.global_time {
        println!("  time:   {}/{}", time.beats_per_bar, time.beat_value);
    }
    println!("  tracks: {}", score.tracks.len());

    for (i, track) in score.tracks.iter().enumerate() {
        println!("  ── Track {i} ──");
        println!("    name:  {}", track.name);
        if let Some(ref inst) = track.instrument {
            println!("    inst:  {} (#{})", inst.display_name(), inst.index());
        }
        println!("    sections: {}", track.sections.len());
        for (j, section) in track.sections.iter().enumerate() {
            let repeat = section
                .repeat_times
                .map(|r| format!(" ×{r}"))
                .unwrap_or_default();
            println!(
                "    [{}] \"{}\"{} — {} measures",
                j, section.name, repeat, section.measures.len()
            );
            for (k, measure) in section.measures.iter().enumerate() {
                let event_summary: Vec<String> = measure
                    .events
                    .iter()
                    .map(format_event)
                    .collect();
                println!("      m{k}: {}", event_summary.join("  "));
            }
        }
    }

    println!("── End ────────────────────────────");
    ExitCode::SUCCESS
}

// ── main dispatch ───────────────────────────────────────

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "-V" | "--version" => {
            println!("kyma {VERSION}");
            ExitCode::SUCCESS
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("error: compile requires an input file");
                eprintln!("usage: kyma compile <input.tm> [-o output.bm]");
                return ExitCode::from(1);
            }
            let input_path = &args[2];
            let output_override = if args.len() >= 5 && args[3] == "-o" {
                Some(args[4].as_str())
            } else {
                None
            };
            cmd_compile(input_path, output_override)
        }
        "perform" => {
            if args.len() < 3 {
                eprintln!("error: perform requires a .bm file path");
                eprintln!("usage: kyma perform <input.bm> [--port N]");
                return ExitCode::from(1);
            }
            cmd_perform(&args[2..])
        }
        "ports" => cmd_ports(),
        "show" => {
            if args.len() < 3 {
                eprintln!("error: show requires an input file");
                eprintln!("usage: kyma show <input.tm> [-f text|bm]");
                return ExitCode::from(1);
            }
            let input_path = &args[2];
            let mut format = "text";
            let mut i = 3;
            while i < args.len() {
                if args[i] == "-f" {
                    if i + 1 < args.len() {
                        format = &args[i + 1];
                        i += 2;
                    } else {
                        eprintln!("error: -f requires a format argument (text|bm)");
                        return ExitCode::from(1);
                    }
                } else {
                    i += 1;
                }
            }
            if format != "text" && format != "bm" {
                eprintln!("error: unknown format '{format}' (use 'text' or 'bm')");
                return ExitCode::from(1);
            }
            cmd_show(input_path, format)
        }
        "transpose" => {
            if args.len() < 4 {
                eprintln!("error: transpose requires an input file and semitones");
                eprintln!("usage: kyma transpose <input.tm> <semitones>");
                return ExitCode::from(1);
            }
            let input_path = &args[2];
            let semitones: i8 = match args[3].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("error: semitones must be an integer, got '{}'", args[3]);
                    return ExitCode::from(1);
                }
            };
            cmd_transpose(input_path, semitones)
        }
        _ => {
            eprintln!("error: unknown subcommand '{}'", args[1]);
            eprintln!();
            print_usage();
            ExitCode::from(1)
        }
    }
}
