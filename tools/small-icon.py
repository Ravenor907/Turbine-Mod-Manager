#!/usr/bin/env python3
"""Draws the rail's small app icon (packaging/...-small.png) from the full icon: the same gold, with six bold blades,
a thick ring and a plain hub, so it still reads at 32 to 64 pixels. The corners are transparent.

Usage: python3 tools/small-icon.py   (needs Pillow)"""
import math
import os
from PIL import Image, ImageDraw

root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
full = Image.open(os.path.join(root, "packaging/io.github.ravenor907.TurbineModManager-512.png")).convert("RGBA")
gold = full.getpixel((256, 20))  # the outer ring's color
dark = (11, 10, 8, 255)

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
c = S / 2
R = S * 0.48
d.ellipse([c - R, c - R, c + R, c + R], fill=gold)
ring = R * 0.86
d.ellipse([c - ring, c - ring, c + ring, c + ring], fill=dark)
blades = 6
r0, r1 = R * 0.24, R * 0.80
twist = 1.9  # radians swept from the hub to a blade's tip
for i in range(blades):
    a0 = 2 * math.pi * i / blades
    lead, trail = [], []
    steps = 60
    for k in range(steps + 1):
        t = k / steps
        r = r0 + (r1 - r0) * t
        th = a0 + twist * t ** 1.15
        # Widest a little past the middle, pointed at the tip.
        w = 0.78 * math.sin(math.pi * min(1, t * 1.05)) ** 0.8 * (1 - 0.35 * t) + 0.12 * (1 - t)
        lead.append((c + r * math.cos(th), c + r * math.sin(th)))
        trail.append((c + r * math.cos(th - w), c + r * math.sin(th - w)))
    d.polygon(lead + trail[::-1], fill=gold)
h = R * 0.20
d.ellipse([c - h, c - h, c + h, c + h], fill=gold)
h2 = R * 0.09
d.ellipse([c - h2, c - h2, c + h2, c + h2], fill=dark)
img.resize((128, 128), Image.LANCZOS).save(os.path.join(root, "packaging/io.github.ravenor907.TurbineModManager-small.png"))
