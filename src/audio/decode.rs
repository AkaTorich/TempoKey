// Чтение FLAC / WAV / MP3 / AAC (ADTS и M4A) / OGG Vorbis через symphonia в моно f32.
use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::analysis::{Failure, Reporter};

pub struct Decoded {
    /// Среднее по каналам.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

/// Декодирует файл целиком. Ход — доля `0…1`, умноженная на `span` и прибавленная к `base`.
pub fn decode_mono(path: &Path, rep: &dyn Reporter, base: f64, span: f64) -> Result<Decoded, Failure> {
    let file = File::open(path).map_err(|e| Failure::msg(format!("cannot open file: {e}")))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions { enable_gapless: true, ..Default::default() },
            &MetadataOptions::default(),
        )
        .map_err(|e| Failure::msg(format!("unrecognized format: {e}")))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| Failure::msg("no audio track in file"))?;
    let track_id = track.id;
    let total_frames = track.codec_params.n_frames.unwrap_or(0);
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(0);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| Failure::msg(format!("unsupported codec: {e}")))?;

    let mut mono: Vec<f32> = Vec::with_capacity(total_frames as usize);
    let mut buf: Option<SampleBuffer<f32>> = None;
    let mut buf_channels = 0usize;
    let mut last_report = 0.0f64;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => {
                if mono.is_empty() {
                    return Err(Failure::msg(format!("read error: {e}")));
                }
                break;
            }
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio) => {
                let spec = *audio.spec();
                sample_rate = spec.rate;
                channels = spec.channels.count().max(1);
                let need = audio.capacity() * channels;
                if buf.as_ref().map_or(true, |b| b.capacity() < need) || buf_channels != channels {
                    buf = Some(SampleBuffer::new(audio.capacity() as u64, spec));
                    buf_channels = channels;
                }
                let b = buf.as_mut().unwrap();
                b.copy_interleaved_ref(audio);
                let scale = 1.0 / channels as f32;
                mono.extend(b.samples().chunks_exact(channels).map(|f| f.iter().sum::<f32>() * scale));
            }
            // испорченный пакет пропускается, как делают проигрыватели
            Err(SymError::DecodeError(_)) => continue,
            Err(SymError::IoError(_)) => break,
            Err(e) => return Err(Failure::msg(format!("decode error: {e}"))),
        }
        if total_frames > 0 {
            let frac = (mono.len() as f64 / total_frames as f64).min(1.0);
            if frac - last_report >= 0.02 {
                last_report = frac;
                rep.progress(base + span * frac);
                if rep.cancelled() {
                    return Err(Failure::Cancelled);
                }
            }
        }
    }

    if mono.is_empty() || sample_rate == 0 {
        return Err(Failure::msg("file contains no audio"));
    }
    Ok(Decoded { samples: mono, sample_rate, channels })
}
