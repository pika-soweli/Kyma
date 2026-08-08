; ─────────────────────────────────────────────
; toki-musi 示例：帕赫贝尔《卡农》片段
; 新语法 v2 演示
; ─────────────────────────────────────────────

@title("Canon in D — Excerpt")
@key(D, major)
@tempo(120)
@time(4/4)
@dur(4)

; ── 主旋律轨道 ──
track "melody" piano {
    section "A" {
        F#5 E5 D5 C#5 |
        B4 A4 B4 C#5 |
    }

    section "B" {
        D5 ~ D5 C#5 B4 A4 |
    }
}

; ── 低音轨道 ──
track "bass" cello {
    section "A" {
        D2:1 |
        A2:1 |
        B2:1 |
        F#2:1 |
        G2:1 |
        D2:1 |
        G2:1 |
        A2:1 |
    }
}

; ── 和弦轨道 ──
track "harmony" guitar_nylon {
    section "A" {
        [D maj]:2 [A maj]:2 |
        [B m]:2 [F# m]:2 |
        [G maj]:2 [D maj]:2 |
        [G maj]:2 [A 7]:2 |
    }

    section "B" {
        ; 附点节奏 + slash 和弦
        [D maj7]:4. [A maj7]:4. |
        [B m7]:4. [F# m7]:4. |
        [G maj7 / D]:2 [A 7 / E]:2 |
    }
}
