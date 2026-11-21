// Результат разбора одного файла — уходит в окно как JSON.
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct TrackReport {
    pub path: String,
    pub sample_rate: u32,
    pub channels: usize,
    /// Длительность, секунды.
    pub duration: f64,
    pub tempo: TempoReport,
    pub key: KeyReport,
    /// Обзор волны: пары (минимум, максимум) по равным отрезкам, нормированы к пику.
    pub waveform: Vec<[f32; 2]>,
    /// Время разбора, мс.
    pub elapsed_ms: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct TempoCandidate {
    /// Уточнённый темп, удары в минуту.
    pub bpm: f64,
    /// Выраженность темпа (у лучшего кандидата 1).
    pub salience: f64,
    /// Время первой доли сетки, секунды (0 ≤ offset < 60/bpm).
    pub offset: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct TempoReport {
    /// Кандидаты по убыванию выраженности; выбор по диапазону BPM делает окно.
    pub candidates: Vec<TempoCandidate>,
    /// Огибающая атак для рисунка (нормирована 0…1), равномерно по длительности.
    pub envelope: Vec<f32>,
    /// Самый насыщенный ударами отрезок огибающей в полном разрешении — для сетки долей.
    pub excerpt: Vec<f32>,
    /// Начало отрезка, секунды.
    pub excerpt_start: f64,
    /// Кадров огибающей в секунду.
    pub frame_rate: f64,
    /// Местный темп по окнам — отношение к темпу лучшего кандидата; None — в окне нет ударов.
    pub drift: Vec<Option<f64>>,
    /// Шаг окон местного темпа, секунды.
    pub drift_step: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct KeyReport {
    /// Тоника 0…11 (до = 0).
    pub tonic: u8,
    pub minor: bool,
    /// Доля профилей тональностей, согласных с итогом, 0…1.
    pub confidence: f64,
    /// Совпадение с 24 тональностями: 0…11 мажор, 12…23 минор.
    pub scores: Vec<f64>,
    /// Профиль высот 12 ступеней (до … си), максимум 1.
    pub chroma: Vec<f64>,
    /// Отклонение строя от ля 440 Гц, центы.
    pub tuning_cents: f64,
    /// Частота ля первой октавы в строе трека, Гц.
    pub reference_a: f64,
}
