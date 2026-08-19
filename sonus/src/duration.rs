//! 时值载体 — 纯乐理，无物理时间换算。
//!
//! 词法：`-N` = 非附点 N 分音符，`.N` = 附点 N 分音符。

/// 时值：基础分母 + 是否附点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration {
    /// 时值分母：1=全音符，2=二分，4=四分，8=八分，16=十六分……
    pub base: u32,
    /// 是否附点（时值延长一半）。
    pub dotted: bool,
}

impl Duration {
    pub fn new(base: u32, dotted: bool) -> Self {
        Self { base, dotted }
    }

    /// 相对时值（以全音符 = 1.0 为单位）。
    pub fn value(&self) -> f32 {
        let unit = 1.0 / self.base.max(1) as f32;
        if self.dotted { unit * 1.5 } else { unit }
    }

    /// 以四分音符为单位的时值（-4 = 1.0）。
    pub fn quarter_notes(&self) -> f32 {
        self.value() * 4.0
    }

    /// 文本表示：非附点 `-N`，附点 `.N`。
    pub fn display(&self) -> String {
        let prefix = if self.dotted { "." } else { "-" };
        format!("{}{}", prefix, self.base)
    }

    // ── 常用快捷构造 ──

    pub fn whole() -> Self { Self::new(1, false) }
    pub fn half() -> Self { Self::new(2, false) }
    pub fn quarter() -> Self { Self::new(4, false) }
    pub fn eighth() -> Self { Self::new(8, false) }
    pub fn sixteenth() -> Self { Self::new(16, false) }
    pub fn dotted_half() -> Self { Self::new(2, true) }
    pub fn dotted_quarter() -> Self { Self::new(4, true) }
    pub fn dotted_eighth() -> Self { Self::new(8, true) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarter_note() {
        let d = Duration::quarter();
        assert!((d.value() - 0.25).abs() < 1e-6);
        assert!((d.quarter_notes() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dotted_quarter() {
        let d = Duration::dotted_quarter();
        assert!((d.value() - 0.375).abs() < 1e-6);
        assert!((d.quarter_notes() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_eighth_note() {
        let d = Duration::eighth();
        assert!((d.value() - 0.125).abs() < 1e-6);
    }

    #[test]
    fn test_display() {
        assert_eq!(Duration::quarter().display(), "-4");
        assert_eq!(Duration::dotted_quarter().display(), ".4");
        assert_eq!(Duration::eighth().display(), "-8");
        assert_eq!(Duration::dotted_half().display(), ".2");
    }

    #[test]
    fn test_all_durations() {
        let whole = Duration::whole();
        assert!((whole.value() - 1.0).abs() < 1e-6);
        assert!((whole.quarter_notes() - 4.0).abs() < 1e-6);

        let half = Duration::half();
        assert!((half.value() - 0.5).abs() < 1e-6);
        assert!((half.quarter_notes() - 2.0).abs() < 1e-6);

        let dotted_half = Duration::dotted_half();
        assert!((dotted_half.value() - 0.75).abs() < 1e-6);
        assert!((dotted_half.quarter_notes() - 3.0).abs() < 1e-6);

        let sixteenth = Duration::sixteenth();
        assert!((sixteenth.value() - 0.0625).abs() < 1e-6);
        assert!((sixteenth.quarter_notes() - 0.25).abs() < 1e-6);

        let dotted_eighth = Duration::dotted_eighth();
        assert!((dotted_eighth.value() - 0.1875).abs() < 1e-6);
        assert!((dotted_eighth.quarter_notes() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_dur_value_ordering() {
        assert!(Duration::whole().value() > Duration::half().value());
        assert!(Duration::half().value() > Duration::quarter().value());
        assert!(Duration::quarter().value() > Duration::eighth().value());
        assert!(Duration::eighth().value() > Duration::sixteenth().value());
        assert!(Duration::dotted_half().value() > Duration::half().value());
    }

    #[test]
    fn test_dur_construct() {
        let d = Duration::new(32, false);
        assert_eq!(d.base, 32);
        assert!(!d.dotted);
        assert!((d.value() - 1.0 / 32.0).abs() < 1e-6);

        let d2 = Duration::new(8, true);
        assert!(d2.dotted);
        assert!((d2.value() - 0.1875).abs() < 1e-6);
    }

    #[test]
    fn test_dur_equal() {
        assert_eq!(Duration::quarter(), Duration::quarter());
        assert_eq!(Duration::eighth(), Duration::eighth());
        assert_ne!(Duration::quarter(), Duration::eighth());
        assert_ne!(Duration::dotted_quarter(), Duration::quarter());
    }

    #[test]
    fn test_dur_clone_copy_hash() {
        use std::collections::HashSet;
        let d1 = Duration::quarter();
        let d2 = d1;
        assert_eq!(d1, d2);
        let mut set = HashSet::new();
        set.insert(d1);
        set.insert(d2);
        assert_eq!(set.len(), 1);
    }
}
