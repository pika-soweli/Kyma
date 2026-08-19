//! opus CLI — toki-musi 编译器命令行入口。
//!
//! ```sh
//! opus <input.tm> [-o output.bm]   编译 .tm → .bm
//! opus --perform <input.bm>        读取 .bm 并通过 MIDI 播放
//! opus --list-ports                列出可用 MIDI 端口
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
    eprintln!("  opus --perform <input.bm>          Read .bm and play via MIDI");
    eprintln!("  opus --list-ports                  List available MIDI output ports");
    eprintln!("  opus --version                     Print version");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  opus song.tm                       → produces song.bm");
    eprintln!("  opus song.tm -o out/song.bm        → custom output path");
    eprintln!("  opus --perform song.bm             → play .bm via MIDI");
    eprintln!("  opus --perform song.bm --port 1    → play on specific MIDI port");
}

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
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

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

fn cmd_perform(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: --perform requires a .bm file path");
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

fn cmd_list_ports() -> ExitCode {
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
        "--perform" => cmd_perform(&args[2..]),
        "--list-ports" => cmd_list_ports(),
        _ => {
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
