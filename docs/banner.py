"""Bannière du README : le nom, une phrase, et la carte en graphe (capture de la fixture).

Usage : python docs/banner.py docs   (après `cargo test captures_readme -- --ignored`)
"""
import sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

docs = sys.argv[1]
W, H = 1600, 520
FONT = r"C:\Windows\Fonts\SegUIVar.ttf"

def font(size, style):
    f = ImageFont.truetype(FONT, size)
    f.set_variation_by_name(style)
    return f

BG = (18, 17, 16)
INK = (243, 239, 232)
MUTED = (176, 168, 156)
FAINT = (120, 113, 104)
AMBER = (240, 164, 67)
LINE = (58, 52, 45)

img = Image.new("RGB", (W, H), BG)
d = ImageDraw.Draw(img)

# Une lueur ambre très douce derrière la capture, et la trame de points de la carte.
glow = Image.new("L", (W, H), 0)
ImageDraw.Draw(glow).ellipse((880, -120, 1700, 640), fill=60)
glow = glow.filter(ImageFilter.GaussianBlur(140))
img.paste(Image.new("RGB", (W, H), (60, 40, 18)), (0, 0), glow)
d = ImageDraw.Draw(img)
for y in range(18, H, 26):
    for x in range(18, W, 26):
        d.ellipse((x - 1, y - 1, x + 1, y + 1), fill=(32, 30, 28))

# La carte : la zone du graphe, aux coins arrondis, avec une ombre, fondue à gauche.
shot = Image.open(f"{docs}/carte.png").convert("RGB").crop((300, 232, 1440, 828))
target_h = 430
shot = shot.resize((int(shot.width * target_h / shot.height), target_h), Image.LANCZOS)
x0, y0 = W - shot.width + 60, (H - target_h) // 2
shadow = Image.new("L", (W, H), 0)
ImageDraw.Draw(shadow).rounded_rectangle((x0 + 10, y0 + 24, x0 + shot.width + 10, y0 + target_h + 24), 22, fill=150)
img.paste((0, 0, 0), (0, 0), shadow.filter(ImageFilter.GaussianBlur(28)))
mask = Image.new("L", shot.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((0, 0, shot.width - 1, shot.height - 1), 18, fill=255)
fade = Image.new("L", shot.size, 255)
fd = ImageDraw.Draw(fade)
for i in range(220):
    fd.line((i, 0, i, shot.height), fill=int(255 * (i / 220) ** 1.6))
mask = Image.composite(mask, Image.new("L", shot.size, 0), fade)
img.paste(shot, (x0, y0), mask)
d = ImageDraw.Draw(img)
d.rounded_rectangle((x0, y0, x0 + shot.width - 1, y0 + target_h - 1), 18, outline=LINE, width=1)
# Le bord gauche de la capture se fond dans le fond.
grad = Image.new("L", (240, target_h + 4), 0)
gd = ImageDraw.Draw(grad)
for i in range(240):
    gd.line((i, 0, i, target_h + 4), fill=int(255 * (1 - i / 240) ** 1.4))
img.paste(Image.new("RGB", grad.size, BG), (x0 - 2, y0 - 2), grad)

# Le texte.
d = ImageDraw.Draw(img)
left = 96
d.rounded_rectangle((left, 128, left + 190, 158), 15, fill=(46, 33, 16), outline=(120, 82, 34))
d.text((left + 95, 143), "Windows and Linux", font=font(17, "Semibold Text"), fill=AMBER, anchor="mm")
d.text((left - 3, 178), "Rust Project", font=font(92, "Bold Display"), fill=INK)
d.rectangle((left, 306, left + 64, 311), fill=AMBER)
d.text((left, 348), "All your side projects in one place.", font=font(34, "Semibold Display"), fill=INK)
d.text((left, 398), "It finds them on your disk, sorts them", font=font(25, "Regular"), fill=MUTED)
d.text((left, 430), "and opens them in one click.", font=font(25, "Regular"), fill=MUTED)

img.save(f"{docs}/banner.png", optimize=True)
print("ok", img.size)
