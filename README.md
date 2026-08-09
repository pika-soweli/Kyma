# magnisonus

A Rust music composition toolkit: a custom music language (**toki-musi**) with a compiler that produces a binary IR, backed by a pure music-theory domain model.

---

## Workspace structure

```
magnisonus/
├── sonus/   # Pure music-theory domain model (zero MIDI coupling)
├── opus/    # toki-musi compiler — .tm → .bm (Bin Musi IR)
└── src/     # CLI entry point (top-level orchestrator)
```

## Modules

| Crate | Purpose |
|-------|---------|
| `sonus` | Pitch, interval, scale, key, chord, duration, instrument, and score data model |
| `opus` | Lexer / Parser / IR encoder/decoder for the toki-musi language |
| `magnisonus` | Top-level CLI that coordinates both crates |

## toki-musi language

A human-readable music notation format. Example (`examples/canon.tm` — Pachelbel's Canon excerpt):

```
@title("Canon in D — Excerpt")
@key(D, major)
@tempo(120)
@time(4/4)

track "melody" piano {
    section "A" {
        F#5 E5 D5 C#5 |
        B4 A4 B5 C#5 |
    }
}

track "bass" cello {
    section "A" {
        D2:1 |
        A2:1 |
    }
}
```

## Building

```sh
cargo build
```

## Running

```sh
# Compile a .tm source file to .bm binary
cargo run --bin opus examples/canon.tm

# Decode and print a .bm file
cargo run --bin opus -- --decode examples/canon.bm
```

## Roadmap

| Cycle | Feature |
|-------|---------|
| Cycle 1 | MIDI exporter — `.bm` → `.mid` |
| Cycle 2 | Text renderer — terminal-friendly ASCII score view |
| Cycle 3 | Built-in audio player — WebAudio / cpal synthesis |
| Cycle 4 | Image exporter — PNG / SVG sheet music |

See [PLANNING.md](PLANNING.md) for full details.
