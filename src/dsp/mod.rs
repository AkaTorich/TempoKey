// Общие помощники обработки сигнала: окна, кадровое БПФ, скользящая медиана.
pub mod median;
pub mod spectrum;
pub mod window;

/// Наименьшее число вида 2^a·3^b (a ≥ min_pow2), не меньшее `target`: быстрые размеры БПФ.
pub fn smooth_size(target: usize, min_pow2: u32) -> usize {
    let target = target.max(1 << min_pow2);
    let mut best = usize::MAX;
    let mut p3 = 1usize;
    while p3 < best && p3 <= target * 3 {
        let mut v = p3 << min_pow2;
        while v < target {
            v <<= 1;
        }
        best = best.min(v);
        p3 *= 3;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::smooth_size;

    #[test]
    fn smooth_sizes() {
        assert_eq!(smooth_size(1014, 0), 1024);
        assert_eq!(smooth_size(1104, 0), 1152);
        assert_eq!(smooth_size(3969, 2), 4096);
        assert_eq!(smooth_size(4320, 2), 4608);
        assert_eq!(smooth_size(15435, 2), 15552);
    }
}
