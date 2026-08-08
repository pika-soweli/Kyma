//! 和弦品质 — 7 种三和弦品质及其音程模式。

/// 三和弦品质。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordQuality {
    Maj,
    Min,
    Dim,
    Aug,
    Sus2,
    Sus4,
    Power,
}

impl ChordQuality {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "maj" | "major" | "" => Some(Self::Maj),
            "min" | "m" | "minor" => Some(Self::Min),
            "dim" | "diminished" => Some(Self::Dim),
            "aug" | "augmented" | "+" => Some(Self::Aug),
            "sus2" => Some(Self::Sus2),
            "sus" | "sus4" => Some(Self::Sus4),
            "5" | "power" => Some(Self::Power),
            _ => None,
        }
    }

    /// 用于 from_str / 序列化的短标识。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Maj => "maj",
            Self::Min => "m",
            Self::Dim => "dim",
            Self::Aug => "aug",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            Self::Power => "5",
        }
    }

    /// 用于 Lead-sheet 显示的紧凑形式（Maj 为空串）。
    pub fn as_display_str(&self) -> &'static str {
        match self {
            Self::Maj => "",
            Self::Min => "m",
            Self::Dim => "dim",
            Self::Aug => "aug",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            Self::Power => "5",
        }
    }

    /// 三和弦音程模式（相对根音的半音偏移）。
    pub fn intervals(&self) -> &'static [i8] {
        match self {
            Self::Maj => &[0, 4, 7],
            Self::Min => &[0, 3, 7],
            Self::Dim => &[0, 3, 6],
            Self::Aug => &[0, 4, 8],
            Self::Sus2 => &[0, 2, 7],
            Self::Sus4 => &[0, 5, 7],
            Self::Power => &[0, 7],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intervals() {
        assert_eq!(ChordQuality::Maj.intervals(), &[0, 4, 7]);
        assert_eq!(ChordQuality::Min.intervals(), &[0, 3, 7]);
        assert_eq!(ChordQuality::Dim.intervals(), &[0, 3, 6]);
        assert_eq!(ChordQuality::Aug.intervals(), &[0, 4, 8]);
        assert_eq!(ChordQuality::Sus2.intervals(), &[0, 2, 7]);
        assert_eq!(ChordQuality::Sus4.intervals(), &[0, 5, 7]);
        assert_eq!(ChordQuality::Power.intervals(), &[0, 7]);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(ChordQuality::from_str("maj"), Some(ChordQuality::Maj));
        assert_eq!(ChordQuality::from_str("m"), Some(ChordQuality::Min));
        assert_eq!(ChordQuality::from_str("dim"), Some(ChordQuality::Dim));
        assert_eq!(ChordQuality::from_str("sus4"), Some(ChordQuality::Sus4));
        assert_eq!(ChordQuality::from_str("5"), Some(ChordQuality::Power));
        assert_eq!(ChordQuality::from_str("xyz"), None);
    }

    #[test]
    fn test_display_str() {
        assert_eq!(ChordQuality::Maj.as_display_str(), "");
        assert_eq!(ChordQuality::Min.as_display_str(), "m");
        assert_eq!(ChordQuality::Dim.as_display_str(), "dim");
    }
}
