#!/usr/bin/env python3
"""Builds Turbine's Steam library art (SVG sources and PNGs) into packaging/steam-art/ (run from anywhere).

Usage: python3 tools/make-steam-art.py

Sets: turbine/ (Turbine Mod Manager) and coop/ (the optional ELDEN RING Co-op tile, Turbine-styled, plain text).
Each set: capsule (600x900, <appid>p.png), wide (920x430, <appid>.png), hero (1920x620, <appid>_hero.png),
logo (1280x400 transparent, <appid>_logo.png), icon (256x256, <appid>_icon.png). Turbine's own icon is the app icon
(packaging/...-256.png: the brass mark on a clear background); the mark in the other pictures is that icon's shape
(src-mark-mask.png) filled with brass.
"""
import base64
import subprocess
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "packaging" / "steam-art"
MASK = OUT / "src-mark-mask.png"
MASK_URI = "data:image/png;base64," + base64.b64encode(MASK.read_bytes()).decode()

# Turbine's theme (app/qml/Main.qml, brass theme).
GROUND, PANEL, RAISED, LINE = "#0B0A08", "#141109", "#211B12", "#33291B"
TEXT, SOFT, MUTED = "#EADFC6", "#CDBE9F", "#AC9873"
BRASS, BRASS_HI, BRASS_LO = "#C99E52", "#E6C67E", "#8F6A2C"
EMBER = "#CF6F54"


def defs(extra=""):
    return f"""<defs>
  <linearGradient id="brass" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{BRASS_HI}"/><stop offset="0.55" stop-color="{BRASS}"/><stop offset="1" stop-color="{BRASS_LO}"/>
  </linearGradient>
  <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
    <stop offset="0" stop-color="{BRASS}" stop-opacity="0.22"/><stop offset="1" stop-color="{BRASS}" stop-opacity="0"/>
  </radialGradient>
  <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{PANEL}"/><stop offset="1" stop-color="{GROUND}"/>
  </linearGradient>
  <linearGradient id="rule" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0" stop-color="{BRASS}" stop-opacity="0"/><stop offset="0.5" stop-color="{BRASS}"/><stop offset="1" stop-color="{BRASS}" stop-opacity="0"/>
  </linearGradient>
  {extra}
</defs>"""


def mark(mid, x, y, size, fill="url(#brass)", opacity=1.0, rotate=0):
    """The Turbine mark: the app icon's shape as a mask, filled with brass."""
    cx, cy = x + size / 2, y + size / 2
    return f"""<mask id="{mid}" maskUnits="userSpaceOnUse" x="{x}" y="{y}" width="{size}" height="{size}">
  <image href="{MASK_URI}" x="{x}" y="{y}" width="{size}" height="{size}" transform="rotate({rotate} {cx} {cy})"/>
</mask>
<rect x="{x}" y="{y}" width="{size}" height="{size}" fill="{fill}" mask="url(#{mid})" opacity="{opacity}"/>"""


def svg(w, h, body, extra_defs="", bg=True):
    ground = f'<rect width="{w}" height="{h}" fill="url(#ground)"/>' if bg else ""
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
{defs(extra_defs)}
{ground}
{body}
</svg>
"""


def frame(w, h, inset=14):
    return f'<rect x="{inset}" y="{inset}" width="{w - 2 * inset}" height="{h - 2 * inset}" fill="none" stroke="{BRASS}" stroke-opacity="0.35" stroke-width="2"/>'


def word(text, x, y, size, family="Cinzel", weight=700, fill="url(#brass)", spacing=0, anchor="middle", opacity=1):
    return (f'<text x="{x}" y="{y}" font-family="{family}" font-weight="{weight}" font-size="{size}" '
            f'letter-spacing="{spacing}" text-anchor="{anchor}" fill="{fill}" opacity="{opacity}">{text}</text>')


# ------------------------------------------------------------------------------------------------ Turbine
def turbine_capsule():
    w, h = 600, 900
    b = f"""<circle cx="300" cy="360" r="300" fill="url(#glow)"/>
{mark("m", 130, 190, 340)}
{word("TURBINE", 300, 655, 76, spacing=6)}
<rect x="150" y="690" width="300" height="2" fill="url(#rule)"/>
{word("MOD MANAGER", 300, 735, 26, family="Barlow", weight=600, fill=SOFT, spacing=9)}
{frame(w, h)}"""
    return svg(w, h, b)


def turbine_wide():
    w, h = 920, 430
    b = f"""<circle cx="215" cy="215" r="240" fill="url(#glow)"/>
{mark("m", 70, 70, 290)}
{word("TURBINE", 405, 225, 92, spacing=6, anchor="start")}
<rect x="405" y="252" width="430" height="2" fill="url(#rule)"/>
{word("MOD MANAGER", 408, 298, 30, family="Barlow", weight=600, fill=SOFT, spacing=11, anchor="start")}
{frame(w, h, 12)}"""
    return svg(w, h, b)


def turbine_hero():
    w, h = 1920, 620
    b = f"""<circle cx="420" cy="420" r="620" fill="url(#glow)"/>
{mark("m", 1180, -330, 1280, opacity=0.13, rotate=18)}
<rect x="0" y="{h - 3}" width="{w}" height="3" fill="url(#rule)" opacity="0.6"/>"""
    return svg(w, h, b)


def turbine_logo():
    w, h = 1280, 400
    b = f"""{mark("m", 10, 10, 380)}
{word("TURBINE", 440, 235, 168, spacing=10, anchor="start")}
{word("MOD MANAGER", 448, 330, 54, family="Barlow", weight=600, fill=TEXT, spacing=22, anchor="start")}"""
    return svg(w, h, b, bg=False)


def turbine_icon():
    """0.62: the app icon itself (the brass mark on a clear background), copied, never drawn again here."""
    return None


# ------------------------------------------------------------------------------------------------ Co-op
# Plain text for the game's name (Barlow, Turbine's body face): no game art, no game logo, no game lettering.
def coop_capsule():
    w, h = 600, 900
    b = f"""<circle cx="300" cy="300" r="300" fill="url(#glow)"/>
{mark("m", 105, 95, 390, opacity=0.16)}
{word("ELDEN RING", 300, 470, 66, family="Barlow", weight=600, fill=TEXT, spacing=7)}
<rect x="130" y="510" width="340" height="2" fill="url(#rule)"/>
{word("CO-OP", 300, 610, 96, spacing=12)}
{mark("s", 222, 770, 44)}
{word("TURBINE", 274, 802, 26, spacing=3, anchor="start", fill=SOFT)}
{frame(w, h)}"""
    return svg(w, h, b)


def coop_wide():
    w, h = 920, 430
    b = f"""<circle cx="700" cy="215" r="260" fill="url(#glow)"/>
{mark("m", 560, 15, 400, opacity=0.14)}
{word("ELDEN RING", 70, 185, 64, family="Barlow", weight=600, fill=TEXT, spacing=6, anchor="start")}
<rect x="70" y="212" width="400" height="2" fill="url(#rule)"/>
{word("CO-OP", 66, 305, 88, spacing=12, anchor="start")}
{mark("s", 72, 345, 32)}
{word("TURBINE", 112, 369, 20, spacing=3, anchor="start", fill=SOFT)}
{frame(w, h, 12)}"""
    return svg(w, h, b)


def coop_hero():
    w, h = 1920, 620
    b = f"""<circle cx="420" cy="420" r="620" fill="url(#glow)"/>
{mark("m", 1180, -330, 1280, opacity=0.13, rotate=-18)}
<rect x="0" y="{h - 3}" width="{w}" height="3" fill="url(#rule)" opacity="0.6"/>"""
    return svg(w, h, b)


def coop_logo():
    w, h = 1280, 400
    b = f"""{word("ELDEN RING", 6, 150, 132, family="Barlow", weight=600, fill=TEXT, spacing=12, anchor="start")}
{word("CO-OP", 0, 330, 170, spacing=20, anchor="start")}"""
    return svg(w, h, b, bg=False)


def coop_icon():
    w = h = 256
    # 0.62: a clear background, like the app icon: the mark, dimmed, with the word over it.
    b = f"""{mark("m", 8, 8, 240, opacity=0.38)}
{word("CO-OP", 128, 146, 50, spacing=0)}"""
    return svg(w, h, b, bg=False)


SETS = {
    "turbine": {"capsule": turbine_capsule, "wide": turbine_wide, "hero": turbine_hero, "logo": turbine_logo, "icon": turbine_icon},
    "coop": {"capsule": coop_capsule, "wide": coop_wide, "hero": coop_hero, "logo": coop_logo, "icon": coop_icon},
}

if __name__ == "__main__":
    for name, parts in SETS.items():
        d = OUT / name
        d.mkdir(parents=True, exist_ok=True)
        for part, fn in parts.items():
            # The source kept in the repository points at the mask file; the render inlines it (rsvg-convert reads
            # no files outside the SVG's own folder).
            text = fn()
            if text is None:
                import shutil
                shutil.copyfile(OUT.parent / "io.github.ravenor907.TurbineModManager-256.png", d / f"{part}.png")
                (d / f"{part}.svg").unlink(missing_ok=True)
                print(d / f"{part}.png")
                continue
            (d / f"{part}.svg").write_text(text.replace(MASK_URI, "../src-mark-mask.png"))
            src = d / f".{part}.render.svg"
            src.write_text(text)
            subprocess.run(["rsvg-convert", "-o", str(d / f"{part}.png"), str(src)], check=True)
            if part == "logo":
                # Steam fits the logo by its image size: no empty margins, so it sits where Steam puts it.
                from PIL import Image
                im = Image.open(d / "logo.png")
                l, t, r, b = im.getbbox()
                zoom = min(1280 / (r - l + 16), 720 / (b - t + 16))
                if zoom > 1.05:
                    subprocess.run(["rsvg-convert", "-z", f"{zoom:.3f}", "-o", str(d / "logo.png"), str(src)], check=True)
                    im = Image.open(d / "logo.png")
                    l, t, r, b = im.getbbox()
                im.crop((max(0, l - 8), max(0, t - 8), min(im.width, r + 8), min(im.height, b + 8))).save(d / "logo.png")
            src.unlink()
            print(d / f"{part}.png")
