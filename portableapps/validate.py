#!/usr/bin/env python3
"""Validează un folder VERIGAPortable față de PortableApps.com Format 3.9.
Rulare: python3 portableapps/validate.py <VERIGAPortable> [--package]   (--package cere și binarele aplicației)"""
import configparser, os, re, struct, sys

CATEGORIES = {"Accessibility", "Development", "Education", "Games", "Graphics & Pictures", "Internet", "Music & Video", "Office", "Security", "Utilities"}
root = sys.argv[1]
package = "--package" in sys.argv
errs = []


def need(rel, kind="file"):
    p = os.path.join(root, *rel.split("/"))
    ok = os.path.isfile(p) if kind == "file" else os.path.isdir(p)
    if not ok:
        errs.append(f"lipsește {rel}")
    return p


ini_path = need("App/AppInfo/appinfo.ini")
need("help.html")
need("App/AppInfo/appicon.ico")
for s in (16, 32, 75, 128, 256):
    p = need(f"App/AppInfo/appicon_{s}.png")
    if os.path.isfile(p):
        d = open(p, "rb").read(33)
        w, h, depth, ctype = struct.unpack(">IIBB", d[16:26])
        if (w, h) != (s, s):
            errs.append(f"appicon_{s}.png are {w}x{h}")
        if ctype != 6 or depth != 8:
            errs.append(f"appicon_{s}.png nu e True Color cu alfa")
need("App/DefaultData", "dir")
need("Other/Source/readme.txt")
need("Other/Source/LICENSE")

ico = os.path.join(root, "App", "AppInfo", "appicon.ico")
if os.path.isfile(ico):
    d = open(ico, "rb").read()
    n = struct.unpack("<H", d[4:6])[0]
    have = set()
    for i in range(n):
        e = d[6 + 16 * i : 22 + 16 * i]
        sz = e[0] or 256
        bpp = struct.unpack("<H", e[6:8])[0]
        have.add((sz, bpp))
    for want in [(16, 8), (32, 8), (48, 8), (16, 32), (32, 32), (48, 32), (256, 32)]:
        if want not in have:
            errs.append(f"appicon.ico fără {want[0]}px/{want[1]}-bit")

if os.path.isfile(ini_path):
    raw = open(ini_path, encoding="utf-8").read()
    if '"' in raw:
        errs.append("appinfo.ini conține ghilimele (interzise)")
    c = configparser.ConfigParser(interpolation=None)
    c.optionxform = str
    c.read_string(raw)
    g = lambda s, k, d=None: c.get(s, k, fallback=d)
    if g("Format", "Type") != "PortableApps.comFormat":
        errs.append("[Format] Type")
    if not re.fullmatch(r"3\.\d+", g("Format", "Version", "")):
        errs.append("[Format] Version")
    for k in ("Name", "AppID", "Publisher", "Homepage", "Category", "Description", "Language"):
        if not g("Details", k):
            errs.append(f"[Details] {k} lipsește")
    if not re.fullmatch(r"[A-Za-z0-9.+_-]+", g("Details", "AppID", "")):
        errs.append("AppID are caractere nepermise")
    if g("Details", "Category") not in CATEGORIES:
        errs.append("Category invalid")
    if len(g("Details", "Description", "")) > 512:
        errs.append("Description > 512 caractere")
    for k in ("Shareable", "OpenSource", "Freeware", "CommercialUse"):
        if g("License", k) not in ("true", "false"):
            errs.append(f"[License] {k}")
    if not re.fullmatch(r"\d+\.\d+\.\d+\.\d+", g("Version", "PackageVersion", "")):
        errs.append("PackageVersion trebuie să fie 1.2.3.4")
    if not g("Version", "DisplayVersion"):
        errs.append("DisplayVersion lipsește")
    start = g("Control", "Start", "")
    if not start or not start.lower().endswith(".exe"):
        errs.append("[Control] Start")
    if g("Control", "Icons") != "1":
        errs.append("[Control] Icons")
    if package:
        need(start)
        for sub in ("VERIGA",):
            need(f"App/{sub}/veriga-gui.exe")
            need(f"App/{sub}/veriga.exe")
        # dosar de nivel superior: doar lansatorul și help.html
        extra = [e for e in os.listdir(root) if e not in ("App", "Data", "Other", "help.html", start)]
        if extra:
            errs.append(f"fișiere în plus în rădăcină: {extra}")

if errs:
    print("PortableApps.com Format 3.9: PROBLEME")
    for e in errs:
        print(" -", e)
    sys.exit(1)
print("PortableApps.com Format 3.9: ok" + (" (pachet complet)" if package else " (șablon)"))
