# Значок TempoKey и картинки установщика NSIS.
# Значок: тёмная плита со скошенными углами, жёлтая кромка, линия пульса с дисперсией
# (розовая и голубая копии сдвинуты в стороны, жёлтая поверх). Каждый размер рисуется заново
# в своём масштабе (с четырёхкратным запасом), чтобы на 16 и 32 точках линии не размывались.
# Запуск: python tools/icon/make_icon.py  (из любой папки)
import os
from PIL import Image, ImageDraw, ImageFont, ImageChops

HERE = os.path.dirname(os.path.abspath(__file__))
ICONS = os.path.normpath(os.path.join(HERE, "..", "..", "icons"))
os.makedirs(ICONS, exist_ok=True)

YELLOW = (252, 238, 10)
PINK = (255, 42, 109)
CYAN = (5, 217, 232)
NAVY = (11, 13, 32)
NAVY2 = (24, 20, 70)

SS = 4  # запас сглаживания


def plate_points(s, cut_big, cut_small, inset=0.0):
    i = inset
    return [
        (cut_big + i * 0.41, i), (s - cut_small - i * 0.41, i), (s - i, cut_small + i * 0.41),
        (s - i, s - cut_big - i * 0.41), (s - cut_big - i * 0.41, s - i), (cut_small + i * 0.41, s - i),
        (i, s - cut_small - i * 0.41), (i, cut_big + i * 0.41),
    ]


def pulse_points(s):
    # линия пульса: ровно — вверх — глубоко вниз — вверх — ровно
    m = s * 0.5
    xs = [0.14, 0.33, 0.42, 0.53, 0.62, 0.70, 0.86]
    ys = [0.56, 0.56, 0.24, 0.80, 0.42, 0.56, 0.56]
    return [(x * s, y * s) for x, y in zip(xs, ys)]


def draw_icon(size):
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    big, small = s * 0.22, s * 0.09
    border = max(s * 0.045, SS * 1.2)
    d.polygon(plate_points(s, big, small), fill=YELLOW)
    d.polygon(plate_points(s, big, small, border), fill=NAVY)
    # внутренняя полоса-градиент снизу
    grad = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grad)
    for y in range(int(s * 0.55), s):
        a = int(110 * (y - s * 0.55) / (s * 0.45))
        gd.line([(0, y), (s, y)], fill=NAVY2 + (a,))
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).polygon(plate_points(s, big, small, border), fill=255)
    img.paste(grad, (0, 0), ImageChops.multiply(mask, grad.split()[3]))
    # метка-выступ в правом верхнем углу
    d.polygon([(s * 0.60, border), (s * 0.80, border), (s * 0.76, border + s * 0.06), (s * 0.64, border + s * 0.06)], fill=YELLOW)

    width = max(int(s * 0.085), SS * 2)
    shift = max(s * 0.03, SS * 0.9)
    pts = pulse_points(s)
    # розовая и голубая каймы складываются, жёлтая сердцевина ложится поверх
    layer = Image.new("RGB", (s, s), (0, 0, 0))
    for color, dx in ((PINK, -shift), (CYAN, shift)):
        part = Image.new("RGB", (s, s), (0, 0, 0))
        ImageDraw.Draw(part).line([(x + dx, y) for x, y in pts], fill=color, width=width, joint="curve")
        layer = ImageChops.add(layer, part)
    lmask = layer.convert("L").point(lambda v: 255 if v > 8 else 0)
    img.paste(layer, (0, 0), ImageChops.multiply(lmask, mask))
    core = Image.new("L", (s, s), 0)
    ImageDraw.Draw(core).line(pts, fill=255, width=max(int(width * 0.66), SS), joint="curve")
    img.paste(YELLOW, (0, 0, s, s), ImageChops.multiply(core, mask))
    return img.resize((size, size), Image.LANCZOS)


def font(px):
    for name in ("bahnschrift.ttf", "segoeuib.ttf", "arialbd.ttf"):
        path = os.path.join(os.environ.get("WINDIR", "C:\\Windows"), "Fonts", name)
        if os.path.exists(path):
            return ImageFont.truetype(path, px)
    return ImageFont.load_default()


def scanlines(img, step=3, alpha=38):
    over = Image.new("RGBA", img.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(over)
    for y in range(0, img.size[1], step):
        od.line([(0, y), (img.size[0], y)], fill=(0, 0, 0, alpha))
    return Image.alpha_composite(img, over)


def dispersed_text(img, xy, text, f, base=YELLOW, shift=2):
    d = ImageDraw.Draw(img)
    x, y = xy
    d.text((x - shift, y), text, font=f, fill=PINK + (200,))
    d.text((x + shift, y), text, font=f, fill=CYAN + (200,))
    d.text((x, y), text, font=f, fill=base + (255,))


def nsis_sidebar():
    w, h = 164, 314
    img = Image.new("RGBA", (w, h), NAVY + (255,))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / h
        d.line([(0, y), (w, y)], fill=(int(11 + 18 * t), int(13 + 6 * t), int(32 + 50 * t), 255))
    # косые полосы внизу, под надписями
    for i, c in enumerate((PINK, CYAN, YELLOW)):
        x0 = 60 + i * 16
        d.polygon([(x0, h), (x0 + 9, h), (x0 + 9 + 44, h - 44), (x0 + 44, h - 44)], fill=c + (255,))
    icon = draw_icon(96)
    img.alpha_composite(icon, ((w - 96) // 2, 26))
    dispersed_text(img, (18, 140), "TEMPO", font(40))
    dispersed_text(img, (18, 182), "KEY", font(40), base=(240, 240, 255))
    d.rectangle([(18, 232), (w - 18, 234)], fill=YELLOW + (255,))
    d.text((18, 242), "BPM  //  KEY  DETECTOR", font=font(12), fill=CYAN + (255,))
    img = scanlines(img)
    img.convert("RGB").save(os.path.join(ICONS, "nsis-sidebar.bmp"))


def nsis_header():
    w, h = 150, 57
    img = Image.new("RGBA", (w, h), NAVY + (255,))
    d = ImageDraw.Draw(img)
    # шапку NSIS рисует справа от своего заголовка — только знак и полосы
    d.polygon([(w - 70, 0), (w, 0), (w, h), (w - 48, h)], fill=YELLOW + (255,))
    d.polygon([(w - 80, 0), (w - 74, 0), (w - 52, h), (w - 58, h)], fill=PINK + (255,))
    d.polygon([(w - 90, 0), (w - 84, 0), (w - 62, h), (w - 68, h)], fill=CYAN + (255,))
    icon = draw_icon(44)
    img.alpha_composite(icon, (w - 50, 6))
    img = scanlines(img, alpha=30)
    img.convert("RGB").save(os.path.join(ICONS, "nsis-header.bmp"))


def main():
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    images = {s: draw_icon(s) for s in sizes + [512]}
    # размеры в классическом BMP: их читает любая часть Windows (ярлыки, панель задач, Проводник);
    # PNG внутри .ico для малых размеров кое-где не берётся — тогда на ярлыке встаёт стандартный значок
    images[256].save(os.path.join(ICONS, "icon.ico"), sizes=[(s, s) for s in sizes],
                     append_images=[images[s] for s in sizes if s != 256], bitmap_format="bmp")
    images[32].save(os.path.join(ICONS, "32x32.png"))
    images[128].save(os.path.join(ICONS, "128x128.png"))
    images[256].save(os.path.join(ICONS, "128x128@2x.png"))
    images[512].save(os.path.join(ICONS, "icon.png"))
    nsis_sidebar()
    nsis_header()
    # лист для просмотра всех размеров
    sheet = Image.new("RGBA", (sum(sizes) + 20 * len(sizes) + 20, 300), (40, 40, 48, 255))
    x = 20
    for s in sizes:
        sheet.alpha_composite(images[s], (x, 20))
        x += s + 20
    sheet.save(os.path.join(HERE, "preview.png"))
    print("значки:", ICONS)


if __name__ == "__main__":
    main()
