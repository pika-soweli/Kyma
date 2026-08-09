//! magnisonus — toki-musi 编译器顶层 CLI
//!
//! 子命令：
//!   compile  input.tm [-o output.bm]     编译 .tm → .bm
//!   decode   input.bm                    解码 .bm 并打印 Score 概要
//!   show     input.tm [-f text|bm]       文本渲染乐谱
//!   help                                 显示帮助信息
//!
//! 用法：
//!   magnisonus <subcommand> [args...]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";

fn print_usage() {
    eprintln!("magnisonus v{VERSION} — toki-musi compiler & score tools");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  magnisonus compile <input.tm> [-o output.bm]   Compile to .bm binary");
    eprintln!("  magnisonus decode  <input.bm>                  Decode .bm and print score");
    eprintln!("  magnisonus show    <input.tm> [-f text|bm]     Render score to text");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  -h, --help     Print this help");
    eprintln!("  -V, --version  Print version");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  magnisonus compile examples/canon.tm");
    eprintln!("  magnisonus decode  examples/canon.bm");
    eprintln!("  magnisonus show    examples/canon.tm -f text");
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

// ── decode: .bm → Score summary ─────────────────────────

fn cmd_decode(input_path: &str) -> ExitCode {
    let bytes = match fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    let score = match opus::ir::decode(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: IR decode failed: {}", e);
            return ExitCode::from(1);
        }
    };

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
                    .map(|e| match e {
                        sonus::MeasureEvent::Note(n) => n.display(),
                        sonus::MeasureEvent::Chord(c) => c.display(),
                        sonus::MeasureEvent::Tuplet(t) => t.display(),
                        sonus::MeasureEvent::GraceNote(g) => g.display(),
                        sonus::MeasureEvent::Control(_) => "(ctrl)".to_string(),
                    })
                    .collect();
                println!("      m{k}: {}", event_summary.join("  "));
            }
        }
    }

    println!("── End ────────────────────────────");
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
                    .map(|e| match e {
                        sonus::MeasureEvent::Note(n) => n.display(),
                        sonus::MeasureEvent::Chord(c) => c.display(),
                        sonus::MeasureEvent::Tuplet(t) => t.display(),
                        sonus::MeasureEvent::GraceNote(g) => g.display(),
                        sonus::MeasureEvent::Control(ctrl) => match ctrl {
                            sonus::LocalControl::LocalKey(k) => format!("@key({})", k.display()),
                            sonus::LocalControl::LocalTempo(t) => format!("@tempo({})", t.bpm()),
                            sonus::LocalControl::LocalTime(ts) => {
                                format!("@time({}/{})", ts.beats_per_bar, ts.beat_value)
                            }
                            sonus::LocalControl::PedalOn(p) => format!("@pedal({} on)", p_as_str(p)),
                            sonus::LocalControl::PedalOff(p) => format!("@pedal({} off)", p_as_str(p)),
                            sonus::LocalControl::Volume(v) => format!("volume={v}"),
                            sonus::LocalControl::DynamicMark(d) => format!("dyn({d})"),
                        },
                    })
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
                    .map(|e| match e {
                        sonus::MeasureEvent::Note(n) => n.display(),
                        sonus::MeasureEvent::Chord(c) => c.display(),
                        sonus::MeasureEvent::Tuplet(t) => t.display(),
                        sonus::MeasureEvent::GraceNote(g) => g.display(),
                        sonus::MeasureEvent::Control(_) => "(ctrl)".to_string(),
                    })
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
            println!("magnisonus {VERSION}");
            ExitCode::SUCCESS
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("error: compile requires an input file");
                eprintln!("usage: magnisonus compile <input.tm> [-o output.bm]");
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
        "decode" => {
            if args.len() < 3 {
                eprintln!("error: decode requires a .bm file path");
                eprintln!("usage: magnisonus decode <input.bm>");
                return ExitCode::from(1);
            }
            cmd_decode(&args[2])
        }
        "show" => {
            if args.len() < 3 {
                eprintln!("error: show requires an input file");
                eprintln!("usage: magnisonus show <input.tm> [-f text|bm]");
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
        _ => {
            eprintln!("error: unknown subcommand '{}'", args[1]);
            eprintln!();
            print_usage();
            ExitCode::from(1)
        }
    }
}
