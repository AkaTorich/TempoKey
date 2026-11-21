// Тональность: гармоническая часть (hpss) → профиль высот с подстройкой строя (hpcp) → сравнение с профилями (profiles).
pub mod hpcp;
pub mod hpss;
pub mod notes;
pub mod profiles;

use crate::analysis::{Failure, Reporter};
use crate::report::KeyReport;

pub fn analyze(x: &[f32], sr: f64, rep: &dyn Reporter, base: f64, span: f64) -> Result<KeyReport, Failure> {
    let harmonic = hpss::harmonic(x, sr, rep, base, span * 0.55)?;
    let chroma = hpcp::compute(&harmonic, sr, rep, base + span * 0.55, span * 0.45)?;
    let scores = profiles::scores(&chroma.pcp);

    let best = (0..24).max_by(|&a, &b| scores[a].total_cmp(&scores[b])).unwrap_or(0);

    Ok(KeyReport {
        tonic: (best % 12) as u8,
        minor: best >= 12,
        confidence: profiles::agreement(&chroma.pcp, best),
        scores: scores.to_vec(),
        chroma: chroma.pcp.to_vec(),
        tuning_cents: chroma.tuning_cents,
        reference_a: 440.0 * 2f64.powf(chroma.tuning_cents / 1200.0),
    })
}
