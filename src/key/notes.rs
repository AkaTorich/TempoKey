// Ноты кадра по очереди (оценка и вычитание, Klapuri 2006): из спектральных пиков находится основной тон
// с наибольшей суммой гармоник (вес гармоники h — DECAY^(h−1)), его частота уточняется по совпавшим
// гармоникам, эти гармоники снимаются, поиск повторяется. Так обертон (квинта выше от третьей гармоники,
// терция от пятой) не попадает в профиль как отдельная нота.

/// Сетка основных тонов: MIDI от до1 (32,7 Гц) до до7, по трети полутона.
const PITCH_LO: f64 = 24.0;
const PITCH_HI: f64 = 96.0;
const PITCH_STEP: f64 = 1.0 / 3.0;
const HARMONICS: usize = 8;
const DECAY: f64 = 0.8;
/// Не больше нот в кадре.
const MAX_NOTES: usize = 6;
/// Нота слабее этой доли первой в кадре не берётся.
const STOP_RATIO: f64 = 0.2;
/// Гармоника совпала, если её частота / номер отличается от основного тона меньше чем на долю (≈ полтона).
const MATCH_TOLERANCE: f64 = 0.03;

pub struct Note {
    /// Уточнённая частота основного тона, Гц.
    pub freq: f64,
    /// Сумма гармоник.
    pub salience: f64,
}

/// `peaks` — (частота, амплитуда).
pub fn extract(peaks: &[(f64, f64)]) -> Vec<Note> {
    let bins = ((PITCH_HI - PITCH_LO) / PITCH_STEP) as usize + 1;
    let mut left: Vec<f64> = peaks.iter().map(|p| p.1).collect();
    let mut sal = vec![0f64; bins];
    let mut out: Vec<Note> = Vec::new();
    let mut first = 0.0;

    for _ in 0..MAX_NOTES {
        sal.iter_mut().for_each(|v| *v = 0.0);
        for (i, &(f, _)) in peaks.iter().enumerate() {
            if left[i] <= 0.0 {
                continue;
            }
            let mut w = left[i];
            for h in 1..=HARMONICS {
                let p = (12.0 * (f / h as f64 / 440.0).log2() + 69.0 - PITCH_LO) / PITCH_STEP;
                if p < 0.0 {
                    break;
                }
                if p < (bins - 1) as f64 {
                    let k = p.floor() as usize;
                    let fr = p - k as f64;
                    sal[k] += w * (1.0 - fr);
                    sal[k + 1] += w * fr;
                }
                w *= DECAY;
            }
        }
        let Some((k, &s)) = sal.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) else { break };
        if s <= 0.0 {
            break;
        }
        if out.is_empty() {
            first = s;
        } else if s < STOP_RATIO * first {
            break;
        }
        let guess = 440.0 * 2f64.powf((PITCH_LO + k as f64 * PITCH_STEP - 69.0) / 12.0);
        // (оценка основного тона по гармонике, вес)
        let mut votes: Vec<(f64, f64)> = Vec::new();
        let mut matched = Vec::new();
        for (i, &(f, _)) in peaks.iter().enumerate() {
            if left[i] <= 0.0 {
                continue;
            }
            let h = (f / guess).round();
            if h < 1.0 || h > HARMONICS as f64 {
                continue;
            }
            if ((f / h - guess) / guess).abs() < MATCH_TOLERANCE {
                votes.push((f / h, left[i] * DECAY.powi(h as i32 - 1)));
                matched.push(i);
            }
        }
        if votes.is_empty() {
            break;
        }
        for i in matched {
            left[i] = 0.0;
        }
        out.push(Note { freq: weighted_median(&mut votes), salience: s });
    }
    out
}

/// Взвешенная медиана: чужая гармоника рядом (натуральная терция на 14 центов ниже сетки) не сдвигает ноту.
fn weighted_median(v: &mut [(f64, f64)]) -> f64 {
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = v.iter().map(|x| x.1).sum();
    let mut acc = 0.0;
    for &(f, w) in v.iter() {
        acc += w;
        if acc >= total * 0.5 {
            return f;
        }
    }
    v[v.len() - 1].0
}

#[cfg(test)]
mod tests {
    use super::extract;

    fn harmonics(f0: f64, n: usize, out: &mut Vec<(f64, f64)>) {
        for h in 1..=n {
            out.push((f0 * h as f64, 1.0));
        }
    }

    /// Аккорд до мажор с плоскими гармониками: найдены до, ми, соль, а не соль от третьей гармоники до.
    #[test]
    fn c_major_chord() {
        let mut p = Vec::new();
        for f in [261.63, 329.63, 392.0] {
            harmonics(f, 8, &mut p);
        }
        let notes = extract(&p);
        let mut found: Vec<i32> = notes
            .iter()
            .map(|n| ((12.0 * (n.freq / 440.0).log2() + 69.0).round() as i32).rem_euclid(12))
            .collect();
        found.sort();
        found.dedup();
        assert_eq!(found, vec![0, 4, 7]);
        assert!((notes[0].freq - 261.63).abs() < 1.0 || notes.iter().any(|n| (n.freq - 261.63).abs() < 1.0));
    }
}
