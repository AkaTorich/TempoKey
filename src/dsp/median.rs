// Скользящая медиана нечётной длины. Окно держится отсортированным: шаг — одна вставка и одно удаление.
// У краёв окно укорачивается до доступных значений.

pub fn median_filter(input: &[f32], len: usize, out: &mut [f32]) {
    let n = input.len();
    debug_assert_eq!(out.len(), n);
    if n == 0 {
        return;
    }
    let half = len / 2;
    let mut win: Vec<f32> = Vec::with_capacity(len + 1);
    // окно для i = 0: [0, half]
    for &v in &input[..(half + 1).min(n)] {
        insert(&mut win, v);
    }
    for i in 0..n {
        out[i] = win[win.len() / 2];
        // сдвиг к i + 1: входит i + 1 + half, выходит i − half
        let add = i + 1 + half;
        if add < n {
            insert(&mut win, input[add]);
        }
        if i >= half {
            remove(&mut win, input[i - half]);
        }
    }
}

fn insert(win: &mut Vec<f32>, v: f32) {
    let pos = win.partition_point(|&x| x < v);
    win.insert(pos, v);
}

fn remove(win: &mut Vec<f32>, v: f32) {
    let pos = win.partition_point(|&x| x < v);
    if pos < win.len() {
        win.remove(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::median_filter;

    #[test]
    fn matches_naive() {
        let data: Vec<f32> = (0..200).map(|i| ((i * 37 % 101) as f32).sin() * 10.0).collect();
        let len = 9;
        let mut out = vec![0.0; data.len()];
        median_filter(&data, len, &mut out);
        for i in 0..data.len() {
            let lo = i.saturating_sub(len / 2);
            let hi = (i + len / 2 + 1).min(data.len());
            let mut w: Vec<f32> = data[lo..hi].to_vec();
            w.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert_eq!(out[i], w[w.len() / 2], "i = {i}");
        }
    }
}
