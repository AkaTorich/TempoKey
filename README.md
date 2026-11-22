# TempoKey — track tempo (BPM) and musical key

A Rust + Tauri 2 application: it analyzes FLAC, WAV, MP3, AAC (ADTS and M4A) and OGG Vorbis files and shows the tempo
to a hundredth of a BPM, the key (standard notation, Camelot, Open Key), the track's tuning (reference A frequency and
deviation in cents), the beat grid, tempo drift and the pitch profile. The interface is in English, cyberpunk-styled
(pink, cyan, yellow), with a custom window shape, its own font, colour dispersion effects and a splash screen on startup.

## Accuracy (checked on tracks with a known answer)

`tools/testgen/check.py` runs the program on the tracks from `examples/make_tests.rs` (kick, hi-hats, snare, bass,
chords; fractional tempos, shifted tuning, half-time rhythm, a track without drums):

| track | given | detected |
|---|---|---|
| t1 | 128, Am, 440 | 128.000, Am, ±0.0 ct |
| t2 | 145, F♯m, +20 ct | 144.999, F♯m, +20.1 ct |
| t3 | 87.5, D | 87.501, D, +0.1 ct |
| t4 | 174, Em | 174.000, Em, +0.2 ct |
| t5 | 100, C, no drums | 100.000, C, −2.4 ct |
| t6 | 124.37, B♭m, A = 432 Hz (−31.8 ct) | 124.370, B♭m, −31.7 ct |
| t7 | 140, Gm, half-time | 70.00 / 140.00 (both among candidates), Gm |

A 45 s track is analyzed in ~0.1 s, a 5-minute one in under a second.

## How the analysis works

### Audio input — `src/audio/`
- `decode.rs` → `decode_mono`: symphonia, all channels mixed to mono, corrupted packets skipped, progress by frame share.
- `decimate.rs` → `decimate`, `factor_for`: integer-factor decimation down to 16…32 kHz (Blackman window, cutoff at 92 %).
- `mod.rs` → `EXTENSIONS`, `is_audio`: which files are picked up from folders.

### Tempo — `src/tempo/`
- `onset.rs` → `superflux`: SuperFlux onset envelope (Böck, Widmer 2013) — 12 bands per octave, logarithm,
  increase over the frame μ back with a maximum over three neighbouring bands, 200 frames per second; local 1 s mean
  subtracted.
- `salience.rs` → `candidates`: tempo salience after Peeters (2007) — generalized autocorrelation of the envelope ×
  its spectrum; they agree only at the true tempo, so “×2 / ÷2” errors cancel out. Candidates 30…300 BPM.
- `refine.rs` → `fine_tempo`: each candidate is refined over the whole track — maximum of the summed power at the
  tempo frequency and its three harmonics (Goertzel), two passes with a parabola → hundredths of a BPM;
  `beat_offset` — time of the first beat from the phase; `local_tempo` — local tempo in 40 s windows every 20 s.
- `mod.rs` → `analyze`: candidates, merging of close ones, envelope overview, `densest` — the 8 s excerpt with the
  densest hits for the beat grid.
- Choosing the tempo from the candidates within the window's BPM range — `ui/js/music.js` → `chooseTempo`: salience ×
  a soft preference for the middle of the range (the centre is the geometric mean of the bounds). A candidate can be
  picked by hand.

### Key — `src/key/`
- `hpss.rs` → `harmonic`: harmonic/percussive separation (Fitzgerald 2010) — medians over time and frequency, soft
  mask, signal resynthesis; kick and cymbals don't pollute the note profile.
- `hpcp.rs` → `compute`: 0.7 s frames (~1.4 Hz resolution), Blackman–Harris window, spectral whitening, peaks with
  parabolic refinement → notes of the frame → track tuning from the refined fundamentals → 12-step profile.
- `notes.rs` → `extract`: frame notes one by one (estimation and cancellation, Klapuri 2006) — the fundamental with
  the largest harmonic sum is taken and its harmonics are removed; the note frequency is a weighted median over the
  harmonics. Overtones (the fifth from the 3rd harmonic) don't enter the profile as notes.
- `profiles.rs` → `scores`: correlation with the Krumhansl–Kessler, Temperley, Aarden and Bellman–Budge profiles
  (averaged); `agreement` — confidence: the share of profiles that agree with the result.

### Shared — `src/dsp/`
- `spectrum.rs` → `Plan`, `Workspace`: frame FFT (rustfft) across threads; `window.rs` — Hann and Blackman–Harris
  windows; `median.rs` → `median_filter`: sliding median; `mod.rs` → `smooth_size`: fast FFT sizes 2^a·3^b.

### Application — `src/`
- `main.rs`: Tauri window, commands; `--analyze <file> --out <json>` — analysis without a window.
- `analysis.rs` → `analyze_file`: stage order and progress shares (decoding 0–0.33, tempo 0.33–0.6, key 0.6–1), `cli`.
- `worker.rs`: queue — files one at a time, each file processed on half of the cores; `track-progress` and
  `track-done` events; cancellation by queue generation.
- `commands.rs`: `enqueue_tracks`, `cancel_queue`, `scan_paths` (folders including subfolders), `write_text_file`
  (CSV with BOM).
- `shape.rs` → `set_window_shape`: window region cut along the outline (SetWindowRgn), corrected for the client area
  corner (in a maximized window they differ by 8 pixels). The shape relies on the region only: everything inside it
  is painted opaque, WebView2 transparency is off (`transparent: false`, dark `backgroundColor`) — on Windows it gets
  lost on hidden startup and with modal dialogs and is filled with white. Both windows are created hidden and are
  shown only with their outline: the splash as soon as it sends its outline, the main window when it's ready
  (`reveal_main` applies the stored outline before showing it).
- `boot.rs`: splash — `boot_step` (window build step), `boot_status`, `app_ready` (shows the main window), fallback
  reveal after 10 s.
- `report.rs`: what is sent to the window.

## Window — `ui/`
- `index.html` — main window, `splash.html` — splash screen.
- `css/base.css` — colours, the custom font (@font-face), scrollbars; `frame.css` — frame, neon edge along the outline
  on the inside, title bar, window buttons, status bar (it goes around the outline notch: its edges `--notch-l` /
  `--notch-r` are set by `js/shape.js`; left of the notch — the progress bar, right of it — percent and text), resize
  handles; `controls.css` — buttons (cut corners, arrows), fields, drop-down lists, the `.cprog` progress bar, the
  DECODE ▸ TEMPO ▸ KEY stages, candidate lists; `panels.css` — layout, list rows, track panel; `effects.css` —
  `.disp` dispersion (pink and cyan copies split away from the yellow core; `.burst` flash, rare `.idle` glitch),
  `.prism` prism sweep, file drop; `splash.css` — splash screen.
- `js/app.js` — builds the window and reports steps to the splash (`boot`); `tracks.js` — tracks and queue events;
  `list.js` — the list (sorting by column, arrows, Delete); `detail.js` — track panel, analysis progress by stage,
  candidate picking; `draw.js` — drawings with dispersion (beat grid, waveform overview, pitch wheel along the circle
  of fifths, drift); `music.js` — notation (Am, Camelot, Open Key), `chooseTempo`; `view.js` — settings (BPM range,
  notation, decimals), stored in localStorage; `select.js` — the custom drop-down list; `shape.js` — window outline;
  `guard.js` — no browser context menu and browser keys (F5, Ctrl+R/P/F/U, zoom, back/forward); `export.js` — CSV;
  `splash.js` — splash screen; `demo.js` — tracks for checking the window (`App.demo()`); `tauri.js` — bridge to Tauri.
- `fonts/TempoKey-Regular.ttf`, `TempoKey-Bold.ttf` — the custom font (built by `tools/font`).

### What the window can do
- Files: + FILES / + FOLDER buttons, dragging files and folders onto the window; duplicates are skipped.
- STOP stops the queue, then RESUME continues; CLEAR clears the list; EXPORT CSV.
- BPM RANGE — the tempo selection range (mouse wheel changes it); KEY — key notation; BPM — decimal places.
- Progress: a bar and percent in the track row; stages and a large bar in the track panel; overall progress, number
  and name of the current track at the bottom.
- Track panel: BPM and key with confidence, candidates (click to pick by hand, click again to return to automatic),
  beat grid on an excerpt with hits, waveform overview, pitch wheel, tempo drift, tuning, first beat.
- Borderless window with a custom shape (cut corners, a notch at the bottom), dragged by the title bar, double-click
  to maximize, edges and the corner grip to resize.

## Tools — `tools/`
- `font/make_font.py` — the TempoKey font: `glyphs.py` (glyph skeletons: Latin, Cyrillic, fixed-width digits,
  symbols, ♯ ♭), `pen.py` (strokes with sharp joins and a bevel on tight turns), written with fontTools; `sample.png`
  next to it.
- `icon/make_icon.py` — the icon and installer images (`icons/`).
- `testgen/check.py` + `examples/make_tests.rs` — accuracy check (see above).
- `check/` — mouse-free window check through the WebView2 debug port: `run-debug.ps1`, `cdp.ps1`, `shots.ps1`
  (screenshots at three sizes), `splash-shots.ps1` (splash screen), `desktop-shot.ps1` (screen capture with the
  window), `region-info.ps1` (window region), `dialog-check.ps1` (real screen: startup frames, the window, an open
  file dialog, the window after it closes — checks for white areas and an unshaped rectangle).

## Build
```
cargo build --release
cargo tauri build
```
The first builds the program `target\release\TempoKey.exe`; the second builds the NSIS installer
`target\release\bundle\nsis\TempoKey_1.0.0_x64-setup.exe` (English, per-user install).

Accuracy check:
```
cargo run --release --example make_tests -- <folder>
python tools/testgen/check.py <folder>
```
