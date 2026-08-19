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

    /// 返回四分音符一拍的毫秒数。
    pub fn ms_per_beat(&self) -> f64 {
        60_000.0 / self.bpm as f64
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

    #[test]
    fn test_tempo_clone_copy() {
        let t = Tempo::new(96);
        let t2 = t;
        assert_eq!(t.bpm(), t2.bpm());
    }

    #[test]
    fn test_tempo_methods() {
        let mut t = Tempo::new(120);
        assert_eq!(t.bpm(), 120);
        t.set_bpm(80);
        assert_eq!(t.bpm(), 80);
        t.set_bpm(0);
        assert_eq!(t.bpm(), 1);
        t.set_bpm(5000);
        assert_eq!(t.bpm(), 999);
    }

    #[test]
    fn test_tempo_global_local_same() {
        let a = Tempo::global(100);
        let b = Tempo::local(100);
        assert_eq!(a.bpm(), b.bpm());
    }

    #[test]
    fn test_ms_per_beat() {
        let t = Tempo::new(60);
        assert!((t.ms_per_beat() - 1000.0).abs() < 1e-6);
        let t = Tempo::new(120);
        assert!((t.ms_per_beat() - 500.0).abs() < 1e-6);
        let t = Tempo::new(200);
        assert!((t.ms_per_beat() - 300.0).abs() < 1e-6);
    }
}
