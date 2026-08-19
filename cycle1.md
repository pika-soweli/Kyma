# Kyma 项目变更总结（自 Cycle 1 起）

## 一、Cycle 1 — 表情记号实现 (2026-08-14)

项目第一个完整功能周期，实现 toki-musi 语言的表情记号语法。

**变更内容：**

- `score.rs` 新增 `LocalControl` 枚举（`LocalKey`、`LocalTempo`、`LocalTime`、`PedalOn/Off`、`Volume`、`DynamicMark`）
- `score.rs` 新增 `PedalKind` 枚举（Sustain / Soft / Sostenuto）
- `score.rs` 新增 `Tuplet` 结构体（连音符，ratio + events）
- `MeasureEvent` 新增 `Control`、`Tuplet`、`Grace` 变体
- 解析器支持所有表情记号语法
- 修复 lexer 的 `:N` 词法问题（统一为 `Duration{base:N}`）
- 更新 `examples/canon.tm` 示例
- 新增 15 个 Cycle 1 测试 + 3 个 roundtrip 测试

**结果：170 测试通过**（sonus 108 + opus 62）

---

## 二、项目重命名 (2026-08-16)

- `Cargo.toml` 包名从 `magnisonus` → `kyma`
- 清理 `Cargo.lock`、构建产物中的旧名称残留
- 保留 `toki-musi` 作为音乐记谱语言名

---

## 三、底层乐理模型替换 — 集成 rust-music-theory (2026-08-16)

最大规模重构，将自定义乐理实现替换为开源库 `rust-music-theory 0.4`。

### sonus 新增/变更

| 变更类型 | 文件 | 内容 |
|---------|------|------|
| 新增依赖 | `sonus/Cargo.toml` | `rust-music-theory = "0.4"` |
| 双向转换 | `pitch.rs` | `NoteName ↔ rmt::NoteLetter`、`Pitch ↔ rmt::Pitch` 的 `From` 实现 |
| 双向转换 | `interval.rs` | `Interval ↔ rmt::Interval` 转换 |
| 双向转换 | `scale.rs` | `ScaleType ↔ rmt::ScaleType/Mode` 转换（30 种音阶） |
| 双向转换 | `chord/quality.rs` | `ChordQuality ↔ rmt::chord::Quality` 转换 |
| 新增枚举变体 | `chord/quality.rs` | `HalfDim`（半减七）、`Dom`（属和弦） |
| 新增枚举 | `scale.rs` | `ScaleDirection`（Ascending / Descending） |
| 新增方法 | `scale.rs` | `Scale::new_with_direction()`、`Scale::with_direction()` |
| 和弦解析 | `chord/symbol.rs` | `to_rmt_chord()` — 使用 `Chord::from_string` + 手动构造 fallback |

### 解析器重构 (opus)

| 变更 | 内容 |
|------|------|
| `ParseRule` trait | 统一的解析接口 |
| `ParseContext` | 共享上下文（token 流、位置、默认时值、调性、踏板状态） |
| 独立解析器 | `TrackParser`、`SectionParser`、`NoteParser`、`ChordParser` 等 |
| `EventDispatcher` | token 路由分发 |

**结果：230 → 249 → 259 测试通过**

---

## 四、MIDI 计算层 + 调号变音 (2026-08-16)

为 IR v2 直接存储 MIDI 值做准备，在 sonus 中新增 MIDI 桥接方法。

| 新增方法 | 文件 | 功能 |
|---------|------|------|
| `Pitch::to_midi() -> Option<u8>` | `pitch.rs` | 计算 MIDI 音符值（C4=60） |
| `Pitch::apply_key_signature()` | `pitch.rs` | 根据调号为 Natural 音高应用变音记号 |
| `Note::to_midi() -> Option<u8>` | `note.rs` | 委托给 `Pitch::to_midi()` |
| `Chord::to_midi() -> Option<Vec<u8>>` | `chord/symbol.rs` | 返回和弦所有音符的 MIDI 值 |
| `ParseContext` 新增字段 | `parser.rs` | `key_root`、`key_mode`、`pedals` |

**结果：268 测试通过**

---

## 五、IR v2 — 直接存储 MIDI 值 (2026-08-16)

IR 格式从符号化表示升级为直接存储 MIDI 值。

**格式变更：**

```
Note:  midi(u8) duration velocity              ← 原来存 Pitch 符号
Chord: midi_count [midi(u8)]* duration velocity ← 原来存 ChordSymbol
Grace: midi(u8) duration velocity
```

**影响：**

- 编码更简单，不再需要序列化完整的和弦符号信息
- 解码会丢失和弦品质、扩展音、变音、转位等符号信息
- 为 perform 模块直接播放奠定基础

---

## 六、移除 decode + 新增 perform 模块 (2026-08-16)

### 移除

- `ir.rs` 中所有 decode 函数（`decode`、`decode_track`、`decode_section`、`decode_measure`、`decode_event`、`decode_control`、`decode_chord`）
- `Reader` 结构体、`IrError` 枚举、`midi_to_pitch` 及所有反向转换函数
- `main.rs`（根 crate + opus）中的 `--decode` / `decode` 命令
- 所有 roundtrip 测试

### 新增 perform 模块（最初在 opus 内）

- `perform/mod.rs` — `PerformScore`、`PerfEvent`、`PerfDuration`、`PerfControl` 等轻量 IR 类型
- `perform/reader.rs` — 独立解析 `.bm` 二进制为 `PerformScore`
- `perform/player.rs` — 使用 `midir` 库发送 MIDI 消息（NoteOn/Off、ProgramChange、ControlChange）
- CLI 新增：`kyma perform <file.bm> [--port N]`、`kyma ports`
- 新增依赖：`midir = "0.10"`

**结果：258 测试通过**

---

## 七、perform 拆分为独立 lib crate (2026-08-17)

将 perform 从 opus 的子模块提升为 workspace 中的独立 crate。

### 最终 workspace 结构

```
magnisonus/
├── Cargo.toml          # workspace: sonus, opus, perform; 包名 kyma
├── src/main.rs         # kyma CLI → 依赖 opus + perform
├── sonus/              # 乐理库 (177 tests)
│   └── Cargo.toml      # 依赖 rust-music-theory
├── opus/               # 编译器 .tm → .bm (80 tests)
│   └── Cargo.toml      # 依赖 sonus + perform + rust-music-theory
└── perform/            # 演奏库 .bm → MIDI (6 tests)
    └── Cargo.toml      # 仅依赖 midir
```

### 关键设计决策

- perform 只依赖 `midir`，不依赖 sonus/opus — 零耦合
- roundtrip 测试（encode→read）放在 opus 中（5 个集成测试）
- perform 内部测试只测原始字节解析和时值计算

**结果：263 测试通过**

---

## 测试数量演变

| 阶段 | sonus | opus | perform | 总计 |
|------|-------|------|---------|------|
| Cycle 1 完成 | 108 | 62 | — | **170** |
| rmt 集成 | 168 | 91 | — | **259** |
| MIDI 计算层 | 175 | 93 | — | **268** |
| 移除 decode + perform 模块 | 175 | 77 | 6 | **258** |
| perform 独立 crate | 177 | 80 | 6 | **263** |

---

## 经验教训

1. `rust-music-theory` 0.5 不存在，最高版本为 0.4
2. `Chord::from_string` 在 0.4 中错误解析 `G7` 为 Major+Seventh，属七/属九和弦需手动构造 Dominant quality
3. 重命名根目录需先关闭 IDE/进程以避免文件锁
4. IR v2 存储 MIDI 值会丢失和弦符号信息（品质、扩展音、变音、转位），这是有意的设计取舍
