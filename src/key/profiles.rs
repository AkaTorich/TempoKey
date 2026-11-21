// Профили тональностей и сравнение: коэффициент корреляции Пирсона профиля высот трека с каждым
// профилем, повёрнутым на 12 тоник, — среднее по четырём независимым профилям. Профили сняты с разных
// собраний музыки, поэтому их среднее устойчивее любого одного.

type Pair = ([f64; 12], [f64; 12]);

/// Krumhansl, Kessler (1982) — оценки слушателей.
const KRUMHANSL: Pair = (
    [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88],
    [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17],
);
/// Temperley (2007) — учебник гармонии Kostka, Payne.
const TEMPERLEY: Pair = (
    [0.748, 0.060, 0.488, 0.082, 0.670, 0.460, 0.096, 0.715, 0.104, 0.366, 0.057, 0.400],
    [0.712, 0.084, 0.474, 0.618, 0.049, 0.460, 0.105, 0.747, 0.404, 0.067, 0.133, 0.330],
);
/// Aarden (2003) — собрание народных мелодий Essen.
const AARDEN: Pair = (
    [17.7661, 0.145624, 14.9265, 0.160186, 19.8049, 11.3587, 0.291248, 22.062, 0.145624, 8.15494, 0.232998, 4.95122],
    [18.2648, 0.737619, 14.0499, 16.8599, 0.702494, 14.4362, 0.702494, 18.6161, 4.56621, 1.93186, 7.37619, 1.75623],
);
/// Bellman, Budge (2005) — хоралы и многоголосие.
const BELLMAN_BUDGE: Pair = (
    [16.80, 0.86, 12.95, 1.41, 13.49, 11.93, 1.25, 20.28, 1.80, 8.04, 0.62, 10.57],
    [18.16, 0.69, 12.99, 13.34, 1.07, 11.15, 1.38, 21.07, 7.49, 1.53, 0.92, 10.21],
);

const PROFILES: [Pair; 4] = [KRUMHANSL, TEMPERLEY, AARDEN, BELLMAN_BUDGE];

/// 24 оценки: 0…11 — мажор от до, 12…23 — минор от до.
pub fn scores(chroma: &[f64; 12]) -> [f64; 24] {
    let mut out = [0f64; 24];
    for (major, minor) in PROFILES.iter() {
        for tonic in 0..12 {
            out[tonic] += pearson(chroma, major, tonic);
            out[12 + tonic] += pearson(chroma, minor, tonic);
        }
    }
    out.iter_mut().for_each(|v| *v /= PROFILES.len() as f64);
    out
}

/// Доля профилей, которые сами по себе выбирают тональность `key` (0…23).
pub fn agreement(chroma: &[f64; 12], key: usize) -> f64 {
    let mut agree = 0;
    for (major, minor) in PROFILES.iter() {
        let best = (0..24)
            .max_by(|&a, &b| {
                let s = |k: usize| if k < 12 { pearson(chroma, major, k) } else { pearson(chroma, minor, k - 12) };
                s(a).total_cmp(&s(b))
            })
            .unwrap_or(0);
        if best == key {
            agree += 1;
        }
    }
    agree as f64 / PROFILES.len() as f64
}

/// Корреляция профиля трека с профилем тональности, тоника которой — ступень `tonic`.
fn pearson(chroma: &[f64; 12], profile: &[f64; 12], tonic: usize) -> f64 {
    let p = |pc: usize| profile[(pc + 12 - tonic) % 12];
    let mc = chroma.iter().sum::<f64>() / 12.0;
    let mp = profile.iter().sum::<f64>() / 12.0;
    let (mut num, mut dc, mut dp) = (0.0, 0.0, 0.0);
    for pc in 0..12 {
        let a = chroma[pc] - mc;
        let b = p(pc) - mp;
        num += a * b;
        dc += a * a;
        dp += b * b;
    }
    if dc <= 0.0 || dp <= 0.0 {
        return 0.0;
    }
    num / (dc * dp).sqrt()
}

#[cfg(test)]
mod tests {
    use super::scores;

    /// Звукоряд ля минора с упором на ля, до, ми — должен дать ля минор (тоника 9, индекс 21).
    #[test]
    fn a_minor_from_scale() {
        let mut c = [0.0; 12];
        for (pc, w) in [(9, 1.0), (11, 0.4), (0, 0.8), (2, 0.5), (4, 0.9), (5, 0.4), (7, 0.5)] {
            c[pc] = w;
        }
        let s = scores(&c);
        let best = (0..24).max_by(|&a, &b| s[a].total_cmp(&s[b])).unwrap();
        assert_eq!(best, 21);
    }
}
