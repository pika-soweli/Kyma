; ─────────────────────────────────────────────
; 帕赫贝尔《D大调卡农》完整版
; toki-musi v2 语法 — 含 Cycle 1 表现记号
; ─────────────────────────────────────────────

@title("Canon in D — Johann Pachelbel")
@key(D, major)
@tempo(108)
@time(4/4)
@dur(4)

; ── 主旋律：卡农模仿（延迟两拍进入）──
track "melody" violin {
    ; A段（4+4小节的卡农模式）
    section "A" repeat(2) {
        @dyn(mf) F#5:4 E5:4 D5:4 C#5:4 |
        B4:4 A4:4 B4:4 C#5:4 |
        D5 ~ D5 C#5:4 B4:4 A4:4 |
        B4:4 A4:4 B4:4 A4:4 |
    }

    ; B段（转调色彩，Bm→F#m→G→A）
    section "B" {
        @cresc D5:4 F#5:4 A5:4 G5:4 |
        F#5:4 E5:4 D5:4 C#5:4 |
        B4:4 D5:4 F#5:4 E5:4 |
        @dyn(f) D5:4 C#5:4 B4:4 A4:4 |
        B4:4 D5:4 F#5:4 E5:4 |
        D5:4 C#5:4 B4:4 A4:4 |
        B4:4 A4:4 B4:4 A4:4 |
        G4:4 A4:4 B4:4 C#5:4 |
    }

    ; A' 再现（同A，用 repeat(2) 复用）
    section "A'" repeat(2) {
        @dyn(mp) F#5:4 E5:4 D5:4 C#5:4 |
        B4:4 A4:4 B4:4 C#5:4 |
        D5 ~ D5 C#5:4 B4:4 A4:4 |
        B4:4 A4:4 B4:4 A4:4 |
    }

    ; Coda（终止式）— 含装饰音、三连音、延留记号
    section "Coda" {
        @rit A4:4 B4:4 C#5:4 D5:4 |
        grace(C#5) D5:4 C#5:4 B4:4 A4:4 |
        @accel 3:2 { A4:8 B4:8 C#5:8 } D5:4 E5:4 |
        @fermata C#5:2 ~ C#5:4 R:4 |
        @dyn(f) D5:4 R:4 R:4 R:4 |
    }
}

; ── 低音：固定 ostinato 和弦进行 ──
; 经典和弦循环：D A Bm F#m G D G A
track "bass" cello {
    section "A" repeat(3) {
        @dyn(p) D2:1 |
        A2:1 |
        B2:1 |
        F#2:1 |
        G2:1 |
        D2:1 |
        G2:1 |
        A2:1 |
    }

    section "B" {
        B2:1 |
        F#2:1 |
        G2:1 |
        D2:1 |
        E2:1 |
        B2:1 |
        E2:1 |
        A2:1 |
    }

    section "A'" repeat(2) {
        D2:1 |
        A2:1 |
        B2:1 |
        F#2:1 |
        G2:1 |
        D2:1 |
        G2:1 |
        A2:1 |
    }

    section "Coda" {
        D2:2 |
        F#2:1 |
        B2:1 |
        F#2:1 |
        G2:1 |
        D2:1 |
        G2:1 |
        @fermata A2:2 |
    }
}

; ── 和弦轨：尼龙弦吉他分解和弦 ──
track "harmony" guitar_nylon {
    section "A" repeat(3) {
        @pedal(sustain, on) [D maj]:2 [A maj]:2 |
        [B m]:2 [F# m]:2 |
        [G maj]:2 [D maj]:2 |
        [G maj]:2 [A 7]:2 @pedal(sustain, off) |
    }

    section "B" {
        [B m7]:2 [F# m7]:2 |
        [G maj7]:2 [D maj7]:2 |
        [E 7]:2 [B m7]:2 |
        [E 7]:2 [A 7]:2 |
    }

    section "A'" repeat(2) {
        [D maj]:2 [A maj]:2 |
        [B m]:2 [F# m]:2 |
        [G maj]:2 [D maj]:2 |
        [G maj]:2 [A 7]:2 |
    }

    section "Coda" {
        [D maj7]:4. [F# m7]:4. |
        [B m7]:4. [F# m7]:4. |
        [G maj7]:4. [D maj7]:4. |
        @rit [G maj7 / D]:2 [A 7 / E]:2 |
    }
}
