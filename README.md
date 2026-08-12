# kyma

A Rust music composition toolkit: a custom music language (**toki-musi**) with a compiler that produces a binary IR, backed by a pure music-theory domain model.

---

## Workspace structure

```
kyma/
├── sonus/   # Pure music-theory domain model (zero MIDI coupling)
├── opus/    # toki-musi compiler — .tm → .bm (Bin Musi IR)
└── src/     # CLI entry point (top-level orchestrator)
```

## Modules

| Crate | Purpose |
|-------|---------|
| `sonus` | Pitch, interval, scale, key, chord, duration, instrument, and score data model |
| `opus` | Lexer / Parser / IR encoder/decoder for the toki-musi language |
| `kyma` | Top-level CLI that coordinates both crates |

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

| Cycle | Phase | Feature |
|-------|-------|---------|
| **0** ✅ | MVP | 音符 C4 · 休止 R · 时值 :N / :N. · @key @tempo @time @dur · track/section · [Cmaj7] · | · ; · @inst |
| **1** | expression | ~ 连音线 · 3:2 {…} 连音符 · grace(…) 装饰音 · @cresc @decresc · @rit @accel · @fermata |
| **2** | structure | let / $name 变量 · |: … :| 反复记号 · :|1 :|2 跳房子 · @repeat(N) 段落反复 · include "file.tm" · section 嵌套 |
| **3** | harmony | voice "R/L" {…} 多声部 · [Cmaj7/B3] slash 和弦 · [C add9] [C sus4] · [C no5] 去音 · @dyn(p…ff) · @pedal(sustain) |
| **4** | analysis | @transpose(+5) · @analyze(chord) · @scale(C, dorian) · pcset {0,4,7} · @voicing(close) · MIDI / LilyPond 导出 |
