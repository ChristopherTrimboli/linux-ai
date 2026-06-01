#!/usr/bin/env python3
"""Generate app icons for the Linux AI Companion.

Draws a rounded dark tile with a teal->green gradient prompt glyph (">_") and a
small spark, then exports the PNG sizes Tauri references plus an .ico.
"""
import math
import os

from PIL import Image, ImageDraw

OUT = os.path.join(os.path.dirname(__file__), "icons")
os.makedirs(OUT, exist_ok=True)

BASE = 1024


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def rounded_mask(size, radius):
    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return m


def render(size):
    img = Image.new("RGBA", (BASE, BASE), (0, 0, 0, 0))
    bg = Image.new("RGBA", (BASE, BASE), (0, 0, 0, 255))
    px = bg.load()
    top = (13, 17, 23)        # near-black slate
    bot = (16, 40, 38)        # deep teal
    for y in range(BASE):
        c = lerp(top, bot, y / BASE)
        for x in range(BASE):
            px[x, y] = (c[0], c[1], c[2], 255)
    mask = rounded_mask(BASE, int(BASE * 0.22))
    img.paste(bg, (0, 0), mask)

    d = ImageDraw.Draw(img)

    # Prompt chevron ">"
    accent = (45, 212, 191)   # teal-400
    accent2 = (74, 222, 128)  # green-400
    lw = int(BASE * 0.055)
    cx, cy = int(BASE * 0.34), int(BASE * 0.50)
    arm = int(BASE * 0.14)
    d.line([(cx - arm, cy - arm), (cx + arm * 0.2, cy)], fill=accent, width=lw, joint="curve")
    d.line([(cx + arm * 0.2, cy), (cx - arm, cy + arm)], fill=accent, width=lw, joint="curve")

    # Underscore "_"
    ux0, ux1 = int(BASE * 0.46), int(BASE * 0.66)
    uy = int(BASE * 0.62)
    d.line([(ux0, uy), (ux1, uy)], fill=accent2, width=lw)

    # Spark / star top-right
    sx, sy = int(BASE * 0.72), int(BASE * 0.34)
    r1, r2 = int(BASE * 0.085), int(BASE * 0.032)
    pts = []
    for i in range(8):
        ang = math.pi / 4 * i - math.pi / 2
        r = r1 if i % 2 == 0 else r2
        pts.append((sx + r * math.cos(ang), sy + r * math.sin(ang)))
    d.polygon(pts, fill=(226, 232, 240))

    return img.resize((size, size), Image.LANCZOS)


def main():
    sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    for name, size in sizes.items():
        render(size).save(os.path.join(OUT, name))
    # Multi-resolution .ico for completeness (Windows bundles).
    render(256).save(
        os.path.join(OUT, "icon.ico"),
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print("wrote icons to", OUT)


if __name__ == "__main__":
    main()
