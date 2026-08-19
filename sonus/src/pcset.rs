//! 音级集合（Pitch Class Set）— Forte 集合论基础运算。
//!
//! 使用 12 位整数掩码表示集合，支持标准型、原型型与音程向量。

use super::pitch::PitchClass;

/// 音级集合：12 位掩码，每位对应一个音级类（0=C, 11=B）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PcSet {
    bits: u16,
}

impl PcSet {
    /// 从音级类切片构造。
    pub fn from_pcs(pcs: &[u8]) -> Self {
        let mut bits = 0u16;
        for &pc in pcs {
            bits |= 1 << (pc % 12);
        }
        Self { bits }
    }

    /// 从 `PitchClass` 切片构造。
    pub fn from_pitch_classes(pcs: &[PitchClass]) -> Self {
        Self::from_pcs(&pcs.iter().map(|p| p.get()).collect::<Vec<_>>())
    }

    /// 空集。
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// 是否包含某音级类。
    pub fn contains(&self, pc: u8) -> bool {
        (self.bits & (1 << (pc % 12))) != 0
    }

    /// 添加一个音级类（返回新集合）。
    pub fn insert(&self, pc: u8) -> Self {
        Self { bits: self.bits | (1 << (pc % 12)) }
    }

    /// 移除一个音级类（返回新集合）。
    pub fn remove(&self, pc: u8) -> Self {
        Self { bits: self.bits & !(1 << (pc % 12)) }
    }

    /// 集合中音级类数量。
    pub fn len(&self) -> usize {
        self.bits.count_ones() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// 返回所有音级类（升序）。
    pub fn pcs(&self) -> Vec<u8> {
        (0..12u8).filter(|&i| self.contains(i)).collect()
    }

    /// 转调（循环移位）。
    pub fn transpose(&self, semitones: i8) -> Self {
        let n = ((semitones % 12) + 12) % 12;
        let bits = self.bits;
        let lower = (bits << n) & 0xFFF;
        let upper = bits >> (12 - n);
        Self { bits: lower | upper }
    }

    /// 倒影（I0 变换）：pc → (12 - pc) mod 12。
    pub fn invert(&self) -> Self {
        let mut bits = 0u16;
        for i in 0..12u8 {
            if self.contains(i) {
                bits |= 1 << ((12 - i) % 12);
            }
        }
        Self { bits }
    }

    /// 并集。
    pub fn union(&self, other: &Self) -> Self {
        Self { bits: self.bits | other.bits }
    }

    /// 交集。
    pub fn intersection(&self, other: &Self) -> Self {
        Self { bits: self.bits & other.bits }
    }

    /// 差集（self - other）。
    pub fn difference(&self, other: &Self) -> Self {
        Self { bits: self.bits & !other.bits }
    }

    /// 是否为另一集合的子集。
    pub fn is_subset_of(&self, other: &Self) -> bool {
        (self.bits & other.bits) == self.bits
    }

    /// 是否为真子集。
    pub fn is_proper_subset_of(&self, other: &Self) -> bool {
        self.is_subset_of(other) && self.bits != other.bits
    }

    /// 是否相等（不考虑移位）。
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.bits == other.bits
    }

    // ── Forte 运算 ──

    /// 标准型（Normal Form）：最紧凑的循环排列。
    ///
    /// 返回排序后的音级类列表，从最佳起始音开始。
    pub fn normal_form(&self) -> Vec<u8> {
        let pcs = self.pcs();
        if pcs.len() <= 1 {
            return pcs;
        }

        let n = pcs.len();
        let mut best_start = 0;

        // 尝试从每个音开始，找最紧凑的排列
        for start in 1..n {
            if self.rotation_is_smaller(&pcs, start, best_start, n) {
                best_start = start;
            }
        }

        // 旋转到最佳起始
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push(pcs[(best_start + i) % n]);
        }

        // 归零到起始音
        let base = result[0];
        for pc in &mut result {
            *pc = (*pc + 12 - base) % 12;
        }
        result
    }

    /// 比较两个旋转排列的紧凑度（从 start 开始的更紧凑则返回 true）。
    fn rotation_is_smaller(&self, pcs: &[u8], start: usize, best: usize, n: usize) -> bool {
        for i in (1..n).rev() {
            let s_interval = (pcs[(start + i) % n] + 12 - pcs[start]) % 12;
            let b_interval = (pcs[(best + i) % n] + 12 - pcs[best]) % 12;
            if s_interval != b_interval {
                return s_interval < b_interval;
            }
        }
        // 完全相同，不替换
        false
    }

    /// 原型型（Prime Form）：标准型归零后，取正序与倒影中更小者。
    pub fn prime_form(&self) -> Vec<u8> {
        let normal = self.normal_form();
        if normal.len() <= 1 {
            return normal;
        }

        // 倒影后再求标准型
        let inverted = self.invert();
        let normal_inv = inverted.normal_form();

        // 两者都已归零，比较大小
        if Self::vector_le(&normal, &normal_inv) {
            normal
        } else {
            normal_inv
        }
    }

    /// 音程向量（Interval Vector）：6 元素数组 [ic1, ic2, ic3, ic4, ic5, ic6]。
    ///
    /// icN = 集合中相距 N 个半音的无序音对数量。
    pub fn interval_vector(&self) -> [u8; 6] {
        let pcs = self.pcs();
        let mut iv = [0u8; 6];
        for i in 0..pcs.len() {
            for j in (i + 1)..pcs.len() {
                let diff = ((pcs[j] as i8 - pcs[i] as i8).abs() % 12) as u8;
                let ic = diff.min(12 - diff);
                if ic >= 1 && ic <= 6 {
                    iv[ic as usize - 1] += 1;
                }
            }
        }
        iv
    }

    /// 比较两个等长向量（左到右字典序）。
    fn vector_le(a: &[u8], b: &[u8]) -> bool {
        for i in 0..a.len().min(b.len()) {
            if a[i] != b[i] {
                return a[i] < b[i];
            }
        }
        true
    }
}

impl std::fmt::Display for PcSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pcs = self.pcs();
        write!(f, "[")?;
        for (i, &pc) in pcs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", pc)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ops() {
        let s = PcSet::from_pcs(&[0, 4, 7]); // C major triad
        assert_eq!(s.len(), 3);
        assert!(s.contains(0));
        assert!(s.contains(4));
        assert!(s.contains(7));
        assert!(!s.contains(3));
    }

    #[test]
    fn test_transpose() {
        let s = PcSet::from_pcs(&[0, 4, 7]); // C major
        let t = s.transpose(2); // D major
        assert!(t.contains(2));
        assert!(t.contains(6));
        assert!(t.contains(9));
    }

    #[test]
    fn test_invert() {
        let s = PcSet::from_pcs(&[0, 4, 7]); // C major
        let inv = s.invert();
        // I0 of [0,4,7] = [0, 8, 5] = [0, 5, 8] = C minor
        assert!(inv.contains(0));
        assert!(inv.contains(5));
        assert!(inv.contains(8));
    }

    #[test]
    fn test_set_ops() {
        let a = PcSet::from_pcs(&[0, 4, 7]);
        let b = PcSet::from_pcs(&[0, 3, 7]);
        let u = a.union(&b);
        assert!(u.contains(3) && u.contains(4));
        let i = a.intersection(&b);
        assert_eq!(i.pcs(), vec![0, 7]);
        let d = a.difference(&b);
        assert_eq!(d.pcs(), vec![4]);
    }

    #[test]
    fn test_subset() {
        let big = PcSet::from_pcs(&[0, 2, 4, 7]);
        let small = PcSet::from_pcs(&[0, 4, 7]);
        assert!(small.is_subset_of(&big));
        assert!(small.is_proper_subset_of(&big));
        assert!(!big.is_subset_of(&small));
    }

    #[test]
    fn test_interval_vector_major_triad() {
        // C major triad: {0, 4, 7}
        // Pairs: (0,4)=ic4, (0,7)=ic5, (4,7)=ic3
        let s = PcSet::from_pcs(&[0, 4, 7]);
        let iv = s.interval_vector();
        assert_eq!(iv, [0, 0, 1, 1, 1, 0]);
    }

    #[test]
    fn test_interval_vector_whole_tone() {
        // Whole tone scale: {0, 2, 4, 6, 8, 10}
        // 6 notes, each pair separated by ic2 or ic4 or ic6
        let s = PcSet::from_pcs(&[0, 2, 4, 6, 8, 10]);
        let iv = s.interval_vector();
        // All pairs: C(6,2)=15 pairs
        // ic2: 6 pairs (adjacent), ic4: 6 pairs, ic6: 3 pairs (tritone pairs)
        assert_eq!(iv, [0, 6, 0, 6, 0, 3]);
    }

    #[test]
    fn test_normal_form() {
        // C major triad {0, 4, 7} - already in normal form
        let s = PcSet::from_pcs(&[0, 4, 7]);
        let nf = s.normal_form();
        assert_eq!(nf, vec![0, 4, 7]);
    }

    #[test]
    fn test_prime_form_major_triad() {
        // 大三和弦 {0,4,7} 的原型型 = [0,3,7]（其倒影小三和弦更左紧致，Forte 3-11）
        let s = PcSet::from_pcs(&[0, 4, 7]);
        let pf = s.prime_form();
        assert_eq!(pf, vec![0, 3, 7]);
    }

    #[test]
    fn test_prime_form_minor_triad() {
        // Minor triad {0, 3, 7} - prime form should be [0, 3, 7]
        let s = PcSet::from_pcs(&[0, 3, 7]);
        let pf = s.prime_form();
        assert_eq!(pf, vec![0, 3, 7]);
    }

    #[test]
    fn test_empty_pcset() {
        let empty = PcSet::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(!empty.contains(0));
    }

    #[test]
    fn test_single_note_pcset() {
        let single = PcSet::from_pcs(&[0]);
        assert!(!single.is_empty());
        assert_eq!(single.len(), 1);
        assert!(single.contains(0));
        assert!(!single.contains(1));
    }

    #[test]
    fn test_full_octave_pcset() {
        let full = PcSet::from_pcs(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(full.len(), 12);
        for i in 0..12 {
            assert!(full.contains(i));
        }
    }

    #[test]
    fn test_duplicate_notes_dedup() {
        let dup = PcSet::from_pcs(&[0, 0, 0, 4, 4, 7, 7]);
        assert_eq!(dup.len(), 3);
    }

    #[test]
    fn test_out_of_range_normalized() {
        let pcs = PcSet::from_pcs(&[0, 12, 14]);
        assert_eq!(pcs.len(), 2);
        assert!(pcs.contains(0));
        assert!(pcs.contains(2));
    }
}
