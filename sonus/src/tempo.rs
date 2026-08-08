//! 节拍 — 纯 BPM 表示，无物理时间换算。

/// 节拍速度（BPM = 每分钟四分音符数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tempo {
    bpm: u16,
}

impl Tempo {
    pub fn new(bpm: u16) -> Self {
        Self { bpm: bpm.clamp(1, 999) }
    }

    /// 全局节拍。
    pub fn global(bpm: u16) -> Self {
        Self::new(bpm)
    }

    /// 局部节拍。
    pub fn local(bpm: u16) -> Self {
        Self::new(bpm)
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn set_bpm(&mut self, new_bpm: u16) {
        self.bpm = new_bpm.clamp(1, 999);
    }

    pub fn display(&self) -> String {
        format!("tempo({})", self.bpm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_basic() {
        let t = Tempo::new(120);
        assert_eq!(t.bpm(), 120);
    }

    #[test]
    fn test_tempo_clamp() {
        let t = Tempo::new(0);
        assert_eq!(t.bpm(), 1);
        let t = Tempo::new(10000);
        assert_eq!(t.bpm(), 999);
    }

    #[test]
    fn test_tempo_display() {
        assert_eq!(Tempo::new(140).display(), "tempo(140)");
    }
}
