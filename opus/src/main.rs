//! opus CLI — toki-musi 编译器命令行入口。
//!
//! ```sh
//! opus <input.tm> [-o output.bm]   编译 .tm → .bm
//! opus --decode <input.bm>         解码 .bm 并打印 Score 概要
//! opus --version                   版本信息
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";

fn print_usage() {
    eprintln!("opus {} — toki-musi compiler (.tm → .bm)", VERSION);
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  opus <input.tm> [-o output.bm]    Compile .tm source to .bm binary");
    eprintln!("  opus --decode <input.bm>          Decode .bm and print score summary");
    eprintln!("  opus --version                    Print version");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  opus song.tm                      → produces song.bm");
    eprintln!("  opus song.tm -o out/song.bm       → custom output path");
    eprintln!("  opus --decode song.bm             → prints score structure");
}

fn cmd_compile(input_path: &str, output_override: Option<&str>) -> ExitCode {
    // 读取源文件
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    // 编译
    let bytes = match opus::compile(&source) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    // 确定输出路径
    let output_path = match output_override {
        Some(p) => PathBuf::from(p),
        None => {
            let p = PathBuf::from(input_path);
            let stem = p.file_stem().unwrap_or_default().to_string_lossy();
            let mut out = p.clone();
            out.set_file_name(format!("{}.bm", stem));
            out
        }
    };

    // 写入
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

fn cmd_decode(input_path: &str) -> ExitCode {
    let bytes = match fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return ExitCode::from(1);
        }
    };

    // 打印 magic
    if bytes.len() >= 4 {
        eprintln!("magic: {:02X} {:02X} {:02X} {:02X} ({})",
            bytes[0], bytes[1], bytes[2], bytes[3],
            std::str::from_utf8(&bytes[0..4]).unwrap_or("?"));
    }
    eprintln!("size: {} bytes", bytes.len());

    let score = match opus::ir::decode(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: IR decode failed: {}", e);
            return ExitCode::from(1);
        }
    };

    // 打印概要
    println!("── Score ──────────────────────────");
    if let Some(ref title) = score.title {
        println!("  title:  {}", title);
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
        println!("  ── Track {} ──", i);
        println!("    name:  {}", track.name);
        if let Some(ref inst) = track.instrument {
            println!("    inst:  {} (#{})", inst.display_name(), inst.index());
        }
        println!("    sections: {}", track.sections.len());
        for (j, section) in track.sections.iter().enumerate() {
            let repeat = section.repeat_times
                .map(|r| format!(" ×{}", r))
                .unwrap_or_default();
            println!("    [{}] \"{}\"{} — {} measures", j, section.name, repeat, section.measures.len());
            for (k, measure) in section.measures.iter().enumerate() {
                let event_summary: Vec<String> = measure.events.iter().map(|e| {
                    match e {
                        sonus::MeasureEvent::Note(n) => n.display(),
                        sonus::MeasureEvent::Chord(c) => c.display(),
                        sonus::MeasureEvent::Tuplet(t) => t.display(),
                        sonus::MeasureEvent::GraceNote(g) => g.display(),
                        sonus::MeasureEvent::Control(_) => "(ctrl)".to_string(),
                    }
                }).collect();
                println!("      m{}: {}", k, event_summary.join("  "));
            }
        }
    }

    println!("── End ────────────────────────────");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "--version" | "-V" => {
            println!("opus {}", VERSION);
            ExitCode::SUCCESS
        }
        "--help" | "-h" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "--decode" => {
            if args.len() < 3 {
                eprintln!("error: --decode requires a .bm file path");
                return ExitCode::from(1);
            }
            cmd_decode(&args[2])
        }
        _ => {
            // 编译模式: opus <input.tm> [-o output.bm]
            let input_path = &args[1];
            let output_override = if args.len() >= 4 && args[2] == "-o" {
                Some(args[3].as_str())
            } else {
                None
            };
            cmd_compile(input_path, output_override)
        }
    }
}
