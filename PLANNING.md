# toki-musi 开发规划（修订版）

## 设计原则（讨论结论）

- **track = 多声部**，voice 是多余的，删除 voice 概念
- **section 嵌套**：可选，非必须，暂不实现
- **|: ... :||** 反复记号：与 `repeat(X)` 重复，删除
- **let / $name** 变量：保留，但与 track/section 命名不重叠
- **include** 导入：保留，与变量互补（文件级 vs 作品内）
- **导入去重**：同名变量/section 报错，不静默覆盖

---

## 现状盘点

### Cycle 0 — 已实现 ✅

| 语法 | 状态 |
|------|------|
| `C4` `F#5` `Bb4` 音符 | ✅ |
| `R` `R:2` 休止 | ✅ |
| `:N` `:N.` 时值 | ✅ |
| `@key` `@tempo` `@time` `@dur` 全局头 | ✅ |
| `track "name" instrument { section { ... } }` | ✅ |
| `[Cmaj7]` `[D m7]` `[G / D]` 和弦 | ✅ |
| `|` 小节分隔 | ✅ |
| `;` 注释 | ✅ |
| `~` 连音线 | ✅ |
| `section "A" repeat(3) { ... }` | ✅ 本次新增 |

### 模型层已就绪但 parser 未接入

| 模型字段 | 位置 | 说明 |
|----------|------|------|
| `LocalKey` / `LocalTempo` / `LocalTime` | `sonus/score.rs:60-63` | 段落内局部 key/tempo/time 切换 |
| `PedalOn` / `PedalOff` | `sonus/score.rs:64-65` | 踏板事件 |
| `DynamicMark` | `sonus/score.rs:69` | 力度记号 p/mp/mf/f |
| `Volume` | `sonus/score.rs:67` | 音量 0-127 |
| `ChordAlterItem` (no5/add9/sus4 等) | `sonus/chord/symbol.rs:48-63` | 和弦变更 |
| IR 控制事件编码占位 | `opus/ir.rs:369-372` | `w.u8(3); w.u8(0);` 预留 |

---

## Cycle 1 — 表现记号

> 让序列「会呼吸」

### 1.1 局部控制事件（模型已就绪，只需 parser + IR）

```
@key(E, minor)     → LocalKey
@tempo(90)         → LocalTempo
@time(3/4)         → LocalTime
@pedal(sustain, on)   → PedalOn
@pedal(sustain, off)  → PedalOff
@dyn(p) / @dyn(mf) / @dyn(ff) → DynamicMark
```

**位置**：measure 内任意位置，作为 `MeasureEvent::Control` 插入
**IR**：复用现有 `Control(0)` 占位，将 `LocalControl` enum discriminant 序列化

### 1.2 连音符

```
3:2 { F#5:8 E5:8 D5:8 }    → 三连音（3个八分音符替代2个）
```

**模型**：新增 `sonus/src/tuplet.rs`，`Tuplet { ratio: (3,2), events: Vec<MeasureEvent> }`
**IR**：新增 event type（当前占位的 Control(0) 可复用）

### 1.3 装饰音

```
grace(C#5:8)               → 装饰音，时值极短，不占拍
```

**模型**：`MeasureEvent::Grace(Note)` 或新字段 `Note::grace: bool`
**实现**：简化为时值 × 0.25 直接折入 MeasureEvent

### 1.4 表情记号

```
@cresc / @decresc          → DynamicMark("cresc")
@rit / @accel              → LocalTempo（速度变化，需记录变化量）
@fermata                   → Note 附加 fermata 标记
```

---

## Cycle 2 — 结构复用

> 让谱面「可组织」

### 2.1 反复记号 ||: ... :||

```
||: A段 :||
    F#5:4 E5:4 D5:4 C#5:4 |
    B4:4 A4:4 B4:4 C#5:4 |
:||
```

**语义**：标记一段内容为反复区域，演奏时返回到 `||:` 位置重新演奏。次数由上下文或演奏者决定，不同于 `repeat(X)` 的固定次数。
**实现**：
- lexer 新增 `RepeatBegin` (`||:`) / `RepeatEnd` (`:||`) token
- parser 在 measure 列表中标记范围，解码时展开为重复小节
- 可与 `section repeat(X)` 组合：`section "A" repeat(2) { ||: ... :|| }`

**保留**：与 `repeat(X)` 不重叠——一个标记范围，一个标记次数。

### 2.2 变量 let / $name

```
let "A" = section "A" {
    F#5:4 E5:4 D5:4 C#5:4 |
    B4:4 A4:4 B4:4 C#5:4 |
};

track "melody" violin {
    $A repeat(2) { }    // 引用变量 A，内容展开 + repeat 生效
}
```

**实现**：
- lexer 新增 `Let` / `DollarIdent` token
- parser 维护 `HashMap<String, Vec<Measure>>` 变量注册表
- 遇到 `$name` 时展开为 measure 列表，合并到当前 section
- 同名变量二次定义 → 报错

### 2.3 include "file.tm"

```
include "common.tm"
```

**实现**：
- lexer 新增 `Include` token
- parser 读取文件 → 递归编译 → 合并到当前 Score
- 同名 section / 变量 → 报错
- 防止循环包含（维护已导入文件 Set）

### 2.4 删除的功能

| 功能 | 原因 |
|------|------|
| `voice "name" { ... }` | track 已是多声部单位 |
| section 嵌套 | 结构冗余，flat section list 足够 |

---

## Cycle 3 — 和弦进阶

> 让织体「有厚度」

### 3.1 和弦变更（模型已就绪）

```
[C add9]   → add 9th extension
[C sus4]   → sus4 quality（已支持）
[C no5]    → remove 5th degree
```

**现状**：`ChordAlterItem` 已有 `AlterType::No`，需要 parser 接入

### 3.2 Slash 进阶（已实现）

```
[Cmaj7 / D]   → 已支持
```

### 3.3 @dyn 动态力度

```
@dyn(p) / @dyn(mp) / @dyn(mf) / @dyn(f) / @dyn(ff)
```

**模型**：`LocalControl::DynamicMark(String)` 已就绪
**实现**：Cycle 1 一并完成

### 3.4 @pedal 踏板

```
@pedal(sustain, on) / @pedal(sustain, off)
```

**模型**：`LocalControl::PedalOn/Off` 已就绪
**实现**：Cycle 1 一并完成

---

## Cycle 4 — 乐理分析

> 让语言「懂乐理」

### 4.1 和弦识别

```
@analyze(C, E, G)   → 输出 "[C maj]"
```

**依赖**：`sonus::chord::detect` 已实现，直接调用

### 4.2 转调

```
@transpose(C, +5)   → 整段移调
```

**依赖**：`Pitch::transpose`、`Chord::transpose` 已实现

### 4.3 音级集合

```
pcset {0, 4, 7}     → 输出 prime form / interval vector
```

**依赖**：`sonus::pcset` 已实现

### 4.4 和弦排列

```
@voicing(close)     → 将和弦音重新排列到最近位置
```

**依赖**：`Pitch`、`Chord` 操作已就绪

### 4.5 MIDI / LilyPond 导出

**依赖**：Cycle 1 的 MIDI 导出器（见 PLANNING.md）

---

## 执行顺序

```
Cycle 1 (表现记号)
  └─ 1.1 局部控制（最基础，模型已就绪）
  └─ 1.2 连音符
  └─ 1.3 装饰音
  └─ 1.4 表情记号

Cycle 2 (结构复用)
  └─ 2.1 let / $name
  └─ 2.2 include "file.tm"

Cycle 3 (和弦进阶)
  └─ 3.1 no5 / add9 parser 接入
  └─ 3.2-3.3 已在 Cycle 1 完成

Cycle 4 (乐理分析)
  └─ 依赖 sonus 现有模型，实现较直接
```

## 文件改动范围

| 文件 | 改动内容 |
|------|----------|
| `sonus/src/score.rs` | 可能需要扩 `LocalControl` 或新增 `Tuplet` |
| `sonus/src/tuplet.rs` | **新增**：连音符模型 |
| `opus/src/token.rs` | 新增 `Let` / `Dollar` / `Include` / `Grace` 等 token |
| `opus/src/lexer.rs` | 新增对应词法 |
| `opus/src/parser.rs` | 解析 let/include/grace/tuplet，删除 voice 相关代码 |
| `opus/src/ir.rs` | 新增 Tuplet/Grace/Control discriminant 编码 |
| `src/main.rs` | show/decode 渲染变量引用和 include 结果 |
| `examples/canon.tm` | 用 let/section repeat 精简重构 |
