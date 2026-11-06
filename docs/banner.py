"""Bannière du README : le nom, une phrase, trois verbes, et le bureau de l'application.

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
FRAME = (70, 58, 44)


def font(size, style):
    f = ImageFont.truetype(SEGOE, size)
    f.set_variation_by_name(style)
    return f


img = Image.new("RGB", (W, H), BG)

# Trame de points discrète, comme la carte de l'application.
d = ImageDraw.Draw(img)
for y in range(16, H, 28):
    for x in range(16, W, 28):
        d.ellipse((x - 1, y - 1, x + 1, y + 1), fill=(30, 28, 26))

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
ImageDraw.Draw(shadow).rounded_rectangle((x0 - 6, y0 + 10, x0 + shot.width + 6, H + 60), radius, fill=170)
img.paste((0, 0, 0), (0, 0), shadow.filter(ImageFilter.GaussianBlur(26)))
mask = Image.new("L", shot.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((0, 0, shot.width - 1, shot.height + radius), radius, fill=255)
img.paste(shot, (x0, y0), mask)
d = ImageDraw.Draw(img)
d.rounded_rectangle((x0, y0, x0 + shot.width - 1, H + radius), radius, outline=FRAME, width=2)
# Un liseré de lumière sur le bord haut.
edge = Image.new("L", (W, H), 0)
ImageDraw.Draw(edge).rounded_rectangle((x0 + 40, y0 - 3, x0 + shot.width - 40, y0 + 3), 3, fill=90)
img.paste((255, 196, 120), (0, 0), edge.filter(ImageFilter.GaussianBlur(6)))

# Le texte, à gauche.
d = ImageDraw.Draw(img)
left = 98
room = x0 - left - 60  # la place avant la fenêtre


def fitted(text, size, style):
    """La plus grande taille (jusqu'à `size`) qui tient avant la fenêtre."""
    while size > 20 and d.textlength(text, font=font(size, style)) > room:
        size -= 2
    return font(size, style)


title = fitted("Rust Project", 120, "Bold Display")
box = d.textbbox((left - 4, 0), "Rust Project", font=title, anchor="ls")
top = 300  # ligne de base du titre
d.text((left - 4, top), "Rust Project", font=title, fill=INK, anchor="ls")
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
