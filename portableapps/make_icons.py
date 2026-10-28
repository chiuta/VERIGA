#!/usr/bin/env python3
"""Generează pictogramele PortableApps.com (appicon.ico + appicon_16/32/75/128/256.png) din desenul VERIGA.
Rulare: python3 portableapps/make_icons.py   (necesită Pillow)"""
import io, os, struct
from PIL import Image, ImageDraw

OUT = os.path.join(os.path.dirname(__file__), "VERIGAPortable", "App", "AppInfo")
S = 1024  # suprafață de desen, redusă apoi cu LANCZOS


def draw() -> Image.Image:
    """Fundal întunecat rotunjit + două verigi împletite (albă și cobalt)."""
    r, w = int(S * .19), int(S * .085)
    box_l, box_r = (int(S * .13), int(S * .31), int(S * .60), int(S * .69)), (int(S * .40), int(S * .31), int(S * .87), int(S * .69))

    def link(box, color):
        layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        ImageDraw.Draw(layer).rounded_rectangle(box, radius=r, outline=color, width=w)
        return layer

    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(im).rounded_rectangle((0, 0, S - 1, S - 1), radius=int(S * 0.19), fill=(13, 22, 32, 255))
    white, cobalt = link(box_l, (255, 255, 255, 255)), link(box_r, (88, 120, 255, 255))
    im.alpha_composite(cobalt)
    im.alpha_composite(white)
    # împletire: în jumătatea de sus a zonei de intersecție veriga cobalt trece peste cea albă
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rectangle((int(S * .40), int(S * .20), int(S * .60), int(S * .50)), fill=255)
    over = cobalt.copy()
    over.putalpha(Image.composite(cobalt.getchannel("A"), Image.new("L", (S, S), 0), mask))
    im.alpha_composite(over)
    return im


def bmp_entry(img: Image.Image, bpp: int) -> bytes:
    """Intrare ICO de tip BMP (DIB) cu 32 sau 8 biți."""
    w, h = img.size
    if bpp == 32:
        px = img.convert("RGBA")
        rows = []
        for y in range(h - 1, -1, -1):
            rows.append(b"".join(struct.pack("BBBB", b, g, r, a) for r, g, b, a in (px.getpixel((x, y)) for x in range(w))))
        pixels, pal = b"".join(rows), b""
        colors = 0
    else:
        rgba = img.convert("RGBA")
        flat = Image.new("RGB", rgba.size, (13, 22, 32))
        flat.paste(rgba, mask=rgba.getchannel("A"))
        q = flat.quantize(255, dither=Image.Dither.NONE)  # indexul 255 rămâne liber = transparent
        palette = ((q.getpalette() or []) + [0] * 768)[: 255 * 3]
        pal = b"".join(struct.pack("BBBB", palette[i * 3 + 2], palette[i * 3 + 1], palette[i * 3], 0) for i in range(255)) + b"\0\0\0\0"
        alpha = rgba.getchannel("A")
        rows = []
        for y in range(h - 1, -1, -1):
            rows.append(bytes(255 if alpha.getpixel((x, y)) < 128 else q.getpixel((x, y)) for x in range(w)))
        pixels, colors = b"".join(rows), 256
    # masca AND (1 bit/pixel, rânduri aliniate la 4 octeți), 0 = opac
    stride = ((w + 31) // 32) * 4
    a = img.getchannel("A")
    mask = b""
    for y in range(h - 1, -1, -1):
        bits = bytearray(stride)
        for x in range(w):
            if a.getpixel((x, y)) < (128 if bpp == 8 else 1):
                bits[x // 8] |= 0x80 >> (x % 8)
        mask += bytes(bits)
    hdr = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, bpp, 0, len(pixels) + len(mask), 0, 0, colors, 0)
    return hdr + pal + pixels + mask


def write_ico(path: str, big: Image.Image) -> None:
    entries = []  # (w, h, bpp, data)
    for sz in (16, 32, 48):
        im = big.resize((sz, sz), Image.LANCZOS)
        entries.append((sz, 8, bmp_entry(im, 8)))
        entries.append((sz, 32, bmp_entry(im, 32)))
    buf = io.BytesIO()
    big.resize((256, 256), Image.LANCZOS).save(buf, "PNG")
    entries.append((256, 32, buf.getvalue()))
    out = struct.pack("<HHH", 0, 1, len(entries))
    off = 6 + 16 * len(entries)
    for sz, bpp, data in entries:
        out += struct.pack("<BBBBHHII", 0 if sz == 256 else sz, 0 if sz == 256 else sz, 0 if bpp == 8 else 0, 0, 1, bpp, len(data), off)
        off += len(data)
    out += b"".join(e[2] for e in entries)
    open(path, "wb").write(out)


def main() -> None:
    os.makedirs(OUT, exist_ok=True)
    big = draw()
    for sz in (16, 32, 75, 128, 256):
        big.resize((sz, sz), Image.LANCZOS).save(os.path.join(OUT, f"appicon_{sz}.png"), optimize=True)
    write_ico(os.path.join(OUT, "appicon.ico"), big)
    print("icoane scrise în", OUT)


if __name__ == "__main__":
    main()
