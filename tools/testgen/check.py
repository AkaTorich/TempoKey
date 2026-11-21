# Сверка разбора с заданными темпом, тональностью и строем проверочных треков.
# Запуск: python tools/testgen/check.py <папка с треками и expected.json> [диапазон BPM, по умолчанию 70 180]
# Темп выбирается из кандидатов так же, как в окне (ui/js/music.js, chooseTempo).
import json
import math
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EXE = os.path.normpath(os.path.join(HERE, "..", "..", "target", "release", "TempoKey.exe"))
NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]


def choose(cands, lo, hi):
    c = math.sqrt(lo * hi)
    sigma = max(math.log2(hi / lo) / 2, 0.1)
    best = None
    for k in cands:
        if lo <= k["bpm"] <= hi:
            z = math.log2(k["bpm"] / c) / sigma
            s = k["salience"] * math.exp(-0.5 * z * z)
            if best is None or s > best[0]:
                best = (s, k["bpm"])
    return best[1] if best else None


def key_name(t, m):
    return NAMES[t] + ("m" if m else "")


def main():
    folder = sys.argv[1]
    lo, hi = (float(sys.argv[2]), float(sys.argv[3])) if len(sys.argv) > 3 else (70.0, 180.0)
    expected = json.load(open(os.path.join(folder, "expected.json"), encoding="utf-8"))
    bad = 0
    print(f"{'файл':28} {'BPM задан':>9} {'найден':>9} {'ошибка':>8} | {'тон':>4} {'найден':>6} {'увер.':>5} | {'строй':>6} {'найден':>7} | кандидаты")
    for e in expected:
        path = os.path.join(folder, e["file"])
        out = path + ".json"
        subprocess.run([EXE, "--analyze", path, "--out", out], check=False)
        r = json.load(open(out, encoding="utf-8"))
        if "error" in r:
            print(e["file"], "ОШИБКА:", r["error"])
            bad += 1
            continue
        bpm = choose(r["tempo"]["candidates"], lo, hi)
        err = (bpm - e["bpm"]) if bpm else float("nan")
        k = r["key"]
        key_ok = k["tonic"] == e["tonic"] and k["minor"] == e["minor"]
        tune_err = k["tuning_cents"] - e["cents"]
        ok = bpm is not None and abs(err) <= 0.02 and key_ok and abs(tune_err) <= 3.0
        bad += 0 if ok else 1
        cands = ", ".join(f"{c['bpm']:.2f}({c['salience']:.2f})" for c in r["tempo"]["candidates"][:5])
        print(f"{e['file']:28} {e['bpm']:9.2f} {bpm or 0:9.3f} {err:+8.3f} | {key_name(e['tonic'], e['minor']):>4} "
              f"{key_name(k['tonic'], k['minor']):>6} {k['confidence']:5.2f} | {e['cents']:+6.1f} {k['tuning_cents']:+7.1f} | "
              f"{cands}  {'OK' if ok else '<<< НЕ СОВПАЛО'}  [{r['elapsed_ms']} мс]")
    print("не совпало:", bad)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
