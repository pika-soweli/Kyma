# magnisonus — 开发规划 Cycle 1-4

## 项目全景

| 模块 | 职责 |
|------|------|
| `sonus` | 纯乐理领域模型（零 MIDI 耦合） |
| `opus` | toki-musi 编译器（`.tm` → `.bm`） |
| `magnisonus` | 顶层编排（CLI 入口） |

当前已完成：Lexer / Parser / Score 模型 / Bin-Musi IR（encode/decode）/ 测试覆盖。
缺失层：渲染（MIDI / audio playback / 可视化导出）。

---

## Cycle 1 — MIDI 导出器

**目标**：将 `Score` 转换为标准 `.mid` 文件，使作品可在任何 DAW/播放器中打开。

### 实现要点

1. 新增 `sonus/src/midi.rs` — 纯二进制 MIDI 写入器（无外部依赖）
   - Track 0：Meta Events（`title`、`key`、`tempo`、`time_sig`、`track_name`）
   - Track 1..N：Note On / Note Off（PPQ tick 映射，利用 `Duration::quarter_notes()` × BPM 计算 tick）
   - 支持多轨（每 `Track` → 一个 MIDI Track Chunk）

2. 导出 API（放在 `opus` 层，保持 `sonus` 纯理论）
   ```rust
   // opus/src/midi.rs
   pub fn score_to_midi_bytes(score: &Score) -> Vec<u8>;
   pub fn score_to_midi(score: &Score, path: &str) -> Result<(), MidiError>;
   ```

3. `src/main.rs` 增加子命令
   ```
   magnisonus compile input.tm              # → input.bm
   magnisonus midi input.tm -o out.mid      # → out.mid（直接编译 + 导出）
   magnisonus midi input.bm -o out.mid      # → out.mid（解码 + 导出）
   ```

4. 测试
   - 在 `opus/src/midi.rs` 添加 roundtrip：生成 `.mid` → 解析关键字节（track 数、note on/off 对）验证
   - 集成测试：`examples/canon.tm` → `canon.mid`，人工用播放器验证声音正确

### 验收标准
- `magnisonus midi examples/canon.tm -o /tmp/canon.mid` 生成有效 `.mid`
- 三轨（melody / bass / harmony）各自独立 channel / track
- 速度、拍号、调号在 Track 0 meta 事件中

---

## Cycle 2 — 文本渲染器（ASCII / 乐谱草稿）

**目标**：将 `Score` 渲染为终端友好的文本格式，便于快速审查和版本控制。

### 实现要点

1. 新增 `opus/src/render/text.rs`
   - 输出格式：逐轨逐小节文本，音高用 `C4 D5 F#5`，休止用 `R`，时值后缀 `-4 / -8`
   - 支持合并和弦视图（同一拍位多个事件用 `/` 连接）
   - 可选 Markdown 表格输出（供 git diff 友好）

   示例输出：
   ```
   [melody] piano  4/4 @120
   m0: F#5-4 E5-4 D5-4 C#5-4 |
   m1: B4-4 A4-4 B4-4 C#5-4 |
   ...
   [bass] cello
   m0: D2-1 |
   ...
   ```

2. CLI 增加子命令
   ```
   magnisonus show input.tm           # 输出文本渲染到 stdout
   magnisonus show input.bm --format=text
   magnisonus show input.bm --format=markdown
   ```

3. 测试
   - 给定 `Score` 结构，断言渲染字符串包含预期音符序列
   - 边缘情况：全休止小节、附点时值、slash 和弦显示 `/`

### 验收标准
- `magnisonus show examples/canon.tm` 在终端打印格式正确的文本乐谱
- Markdown 模式输出可直接粘贴到 README / 文档

---

## Cycle 3 — 音频合成播放器（内置）

**目标**：无需外部工具即可试听作品，通过 Rust 内置音频后端直接播放。

### 实现要点

1. 新增依赖：`cpal`（跨平台音频后端，轻量）或 `rodio`（基于 cpal，更高层）
   - 推荐 `rodio`，减少样板代码

2. 新增 `opus/src/player.rs`
   - 将 MIDI 事件（note on/off + velocity）映射到简单合成波形（PCM 采样）
   - 合成策略：三角波 / 正弦波基础音色（不追求音质，追求功能）
   - `play_score(score: &Score)` 函数，阻塞播放到完成

3. CLI 增加子命令
   ```
   magnisonus play input.tm           # 编译 + 合成播放
   magnisonus play input.bm           # 解码 + 合成播放
   ```

4. 可选：支持 MIDI 文件直接播放（利用 Cycle 1 的输出）

### 测试策略
- 无 GUI 环境测试：运行 `play_score` 并验证音频流采样数符合预期时长
- `examples/canon.tm` 播放时长约 20-30 秒，BPM 120，4/4 拍，可计算预期样本数

### 验收标准
- `magnisonus play examples/canon.tm` 在支持的平台上播放声音
- 多轨同时播放（混合波形）
- Ctrl+C 可中断播放

---

## Cycle 4 — 乐谱图像导出（PNG / SVG）

**目标**：生成可发布的乐谱图像，用于文档、分享、印刷。

### 实现要点

1. 新增依赖：`resvg`（SVG 渲染引擎，无 GUI） + `image` crate（PNG 输出）
   - 或纯 `resvg` 直接输出 PNG

2. 新增 `opus/src/render/image.rs`
   - 将 `Score` 转换为 SVG 乐谱（使用标准五线谱符号路径）
   - 音符头、符干、符尾、小节线、谱号、调号、拍号
   - 多轨垂直堆叠，每轨一个 staff

3. CLI 增加子命令
   ```
   magnisonus render input.tm -o out.png     # 默认 PNG
   magnisonus render input.tm --format=svg -o out.svg
   ```

4. 测试
   - SVG 结构验证（检查关键元素是否存在：note-head, staff-line, bar-line）
   - PNG 导出验证：文件大小 > 0，图像尺寸合理

### 验收标准
- `magnisonus render examples/canon.tm -o /tmp/canon.png` 生成可读的乐谱图像
- SVG 输出可直接在浏览器中打开

---

## 依赖关系

```
Cycle 1 (MIDI)  ←  Cycle 3 (Player)  需要 MIDI 事件作为输入
Cycle 2 (Text)  ←  无依赖，可独立
Cycle 3 (Player)←  Cycle 1 (MIDI) 复用 MIDI 事件
Cycle 4 (Image) ←  无强依赖，可独立于 1/2/3
```

建议执行顺序：**Cycle 2 → Cycle 1 → Cycle 3 → Cycle 4**
- Cycle 2 可先实现，快速获得"能看到作品"的反馈
- Cycle 1 是 Cycle 3 的前置，也是 MIDI 生态兼容的基础
- Cycle 3 依赖 Cycle 1 的 MIDI 事件
- Cycle 4 可并行开发，但放到最后做图像渲染更稳妥

---

## 模块位置规划

```
opus/src/
  midi.rs        ← Cycle 1
  render/
    text.rs      ← Cycle 2
    image.rs     ← Cycle 4
  player.rs      ← Cycle 3

src/main.rs      ← Cycle 1/2/3/4 均在此添加子命令
```
