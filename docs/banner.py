"""Bannière du README : l'icône et le nom, une phrase, trois verbes, et le bureau de l'application.

Usage : python docs/banner.py docs   (après `cargo test captures_readme -- --ignored`)
"""
import glob
import os
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

docs = sys.argv[1]
W, H = 2000, 667
SEGOE = r"C:\Windows\Fonts\SegUIVar.ttf"
LUCIDE = glob.glob(os.path.expanduser("~/.cargo/registry/src/*/lucide-icons-*/lucide.ttf"))[0]

BG = (17, 16, 15)
INK = (243, 239, 232)
AMBER = (240, 164, 67)
FRAME = (120, 88, 50)


def font(size, style):
    f = ImageFont.truetype(SEGOE, size)
    f.set_variation_by_name(style)
    return f


img = Image.new("RGB", (W, H), BG)

# Trame de points discrète, comme la carte de l'application.
d = ImageDraw.Draw(img)
for y in range(16, H, 28):
    for x in range(16, W, 28):
        d.ellipse((x - 1.4, y - 1.4, x + 1.4, y + 1.4), fill=(52, 48, 43))

# Une lueur ambre derrière la fenêtre, plus forte vers le haut.
glow = Image.new("L", (W, H), 0)
ImageDraw.Draw(glow).ellipse((900, -260, 2100, 420), fill=120)
glow = glow.filter(ImageFilter.GaussianBlur(160))
img.paste(Image.new("RGB", (W, H), (92, 58, 20)), (0, 0), glow)

# La fenêtre : le bureau, coins arrondis, coupée par le bas de la bannière.
shot = Image.open(f"{docs}/bureau.png").convert("RGB")
scale = 1020 / shot.width
shot = shot.resize((1020, int(shot.height * scale)), Image.LANCZOS)
x0, y0 = W - shot.width - 42, 154
radius = 22
shadow = Image.new("L", (W, H), 0)
ImageDraw.Draw(shadow).rounded_rectangle((x0 - 6, y0 + 18, x0 + shot.width + 6, H + 60), radius, fill=150)
img.paste((0, 0, 0), (0, 0), shadow.filter(ImageFilter.GaussianBlur(30)))
# Le halo : une lueur ambre large et douce tout autour de la fenêtre…
halo = Image.new("L", (W, H), 0)
ImageDraw.Draw(halo).rounded_rectangle((x0 - 14, y0 - 14, x0 + shot.width + 14, H + 60), radius + 14, fill=150)
img.paste((240, 150, 50), (0, 0), halo.filter(ImageFilter.GaussianBlur(42)))
# … et une plus serrée, qui dessine le contour.
rim = Image.new("L", (W, H), 0)
ImageDraw.Draw(rim).rounded_rectangle((x0 - 3, y0 - 3, x0 + shot.width + 3, H + 60), radius + 3, outline=255, width=6)
img.paste((255, 180, 90), (0, 0), rim.filter(ImageFilter.GaussianBlur(9)).point(lambda v: v * 0.55))
mask = Image.new("L", shot.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((0, 0, shot.width - 1, shot.height + radius), radius, fill=255)
img.paste(shot, (x0, y0), mask)
d = ImageDraw.Draw(img)
d.rounded_rectangle((x0, y0, x0 + shot.width - 1, H + radius), radius, outline=FRAME, width=2)

# Le texte, à gauche.
d = ImageDraw.Draw(img)
left = 98
room = x0 - left - 60  # la place avant la fenêtre


def fitted(text, size, style, room=room):
    """La plus grande taille (jusqu'à `size`) qui tient avant la fenêtre."""
    while size > 20 and d.textlength(text, font=font(size, style)) > room:
        size -= 2
    return font(size, style)


# L'icône devant le nom, centrée sur la hauteur des capitales.
ICON, GAP = 138, 30
title = fitted("Rust Project", 120, "Bold Display", room - ICON - GAP - 70)
box = d.textbbox((left - 4, 0), "Rust Project", font=title, anchor="ls")
top = 300  # ligne de base du titre
caps = d.textbbox((0, top), "R", font=title, anchor="ls")
icon = Image.open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "icon.png")).convert("RGBa")
icon = icon.resize((ICON, ICON), Image.LANCZOS).convert("RGBA")
img.paste(icon, (left - round(ICON * 0.03), round((caps[1] + top) / 2 - ICON / 2)), icon)
d = ImageDraw.Draw(img)
d.text((left + ICON + GAP - 4, top), "Rust Project", font=title, fill=INK, anchor="ls")
bar_y = top + box[3] + 26
d.rectangle((left, bar_y, left + 102, bar_y + 10), fill=AMBER)
tag = fitted("All your side projects in one place.", 46, "Semibold Display")
d.text((left, bar_y + 46), "All your side projects in one place.", font=tag, fill=INK, anchor="lt")

# Trois verbes avec leurs icônes (celles de l'application, police Lucide).
icons = ImageFont.truetype(LUCIDE, 42)
label = font(33, "Semibold Display")
x, cy = left, bar_y + 178
for i, (glyph, word) in enumerate([("\ue151", "Discover"), ("\ue529", "Organize"), ("\ue5a4", "Open")]):
    if i:
        d.line((x, cy - 22, x, cy + 22), fill=(70, 66, 60), width=2)
        x += 34
    d.text((x, cy), glyph, font=icons, fill=AMBER, anchor="lm")
    x += 56
    d.text((x, cy), word, font=label, fill=INK, anchor="lm")
    x += d.textlength(word, font=label) + 34

img.save(f"{docs}/banner.png", optimize=True)
print("ok", img.size)
