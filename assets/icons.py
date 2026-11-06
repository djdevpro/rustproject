"""Icônes de l'application à partir de `icon.svg` : `icon.png` (fenêtre, Linux, bannière) et
`rust-project.ico` (l'exécutable Windows, de 16 à 256 px).

Le dessin est recadré au plus près (3 % de marge) : sinon l'icône paraît plus petite que celles
des autres applications dans la barre des tâches.

Usage : python assets/icons.py   (rend le SVG avec Edge en mode headless, puis Pillow)
"""
import os
import subprocess
import tempfile

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
MARGIN = 0.03
ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]


def render(svg, size=2048):
    """Le SVG rendu en PNG transparent de `size` px, par Edge."""
    tmp = tempfile.mkdtemp()
    html = os.path.join(tmp, "render.html")
    with open(html, "w", encoding="utf-8") as f:
        uri = "file:///" + svg.replace("\\", "/")
        f.write(f'<!doctype html><style>html,body{{margin:0;background:transparent}}</style>'
                f'<img src="{uri}" style="display:block;width:{size}px;height:{size}px">')
    out = os.path.join(tmp, "icon.png")
    subprocess.run([EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--default-background-color=00000000",
                    "--force-device-scale-factor=1", f"--window-size={size},{size}", f"--screenshot={out}", "file:///" + html.replace("\\", "/")],
                   check=True, capture_output=True)
    return Image.open(out).convert("RGBA")


def squared(img):
    """Le dessin seul, centré dans un carré avec une petite marge."""
    box = img.getchannel("A").point(lambda v: 255 if v > 8 else 0).getbbox()
    art = img.crop(box)
    side = round(max(art.size) / (1 - 2 * MARGIN))
    out = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    out.paste(art, ((side - art.width) // 2, (side - art.height) // 2))
    return out


def resized(img, size):
    # En alpha prémultiplié : pas de liseré sombre autour du dessin.
    return img.convert("RGBa").resize((size, size), Image.LANCZOS).convert("RGBA")


master = squared(render(os.path.join(HERE, "icon.svg")))
resized(master, 256).save(os.path.join(HERE, "icon.png"), optimize=True)
images = [resized(master, s) for s in ICO_SIZES]
images[-1].save(os.path.join(HERE, "rust-project.ico"), sizes=[(s, s) for s in ICO_SIZES], append_images=images[:-1])
print("ok", master.size)
