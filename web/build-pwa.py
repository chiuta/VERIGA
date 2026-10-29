#!/usr/bin/env python3
"""Generează varianta PWA instalabilă (app/) din web/veriga.html: manifest, service worker (offline),
pictograme (192/512/maskable/apple-touch) și CSP extins doar cu 'self' pentru manifest/worker/pwa.js.
Rulare:  python3 web/build-pwa.py [--check]     (necesită Pillow; --check compară fără a scrie)"""
import hashlib, io, json, os, re, sys
here = os.path.dirname(os.path.abspath(__file__)); root = os.path.dirname(here)
sys.path.insert(0, os.path.join(root, "portableapps"))
from make_icons import draw  # același desen ca restul proiectelor
from PIL import Image

OUT = os.path.join(root, "app")
src = open(os.path.join(here, "veriga.html"), encoding="utf-8").read()
ver = re.search(r'^version = "([^"]+)"', open(os.path.join(root, "Cargo.toml")).read(), re.M).group(1)

def png(im):
    b = io.BytesIO(); im.save(b, "PNG", optimize=True); return b.getvalue()

base = draw().convert("RGBA")
files = {}
files["icon-192.png"] = png(base.resize((192, 192), Image.LANCZOS))
files["icon-512.png"] = png(base.resize((512, 512), Image.LANCZOS))
files["apple-touch-icon.png"] = png(base.resize((180, 180), Image.LANCZOS))
# maskable: pictograma în zona sigură (80%), pe fundal plin
mk = Image.new("RGBA", (512, 512), (13, 22, 32, 255))
mk.alpha_composite(base.resize((384, 384), Image.LANCZOS), (64, 64))
files["icon-maskable-512.png"] = png(mk)

m = re.search(r"(<meta http-equiv=\"Content-Security-Policy\" content=\")([^\"]*)(\")", src)
csp = m.group(2)
csp = csp.replace("script-src ", "script-src 'self' ", 1).replace("img-src ", "img-src 'self' ", 1) + "; manifest-src 'self'; worker-src 'self'"
html = src[:m.start(2)] + csp + src[m.end(2):]
head = ('<link rel="manifest" href="manifest.webmanifest">'
        '<meta name="theme-color" content="#0d1620">'
        '<link rel="icon" type="image/png" sizes="192x192" href="icon-192.png">'
        '<link rel="apple-touch-icon" href="apple-touch-icon.png">')
html = html.replace("</head>", head + "</head>", 1)
# subtitlul aplicației instalabile (nu „single-file”)
html = html.replace('data-ro="Verificator de legături — variantă HTML single-file, 100% în browser" data-en="Link checker — single-file HTML edition, 100% in the browser"',
                    'data-ro="Verificator de legături — aplicație instalabilă (PWA), funcționează offline" data-en="Link checker — installable app (PWA), works offline"', 1)
i = html.rindex("</body>")  # ultimul: în JS există șabloane cu </body>
html = html[:i] + '<script src="pwa.js"></script>' + html[i:]
files["index.html"] = html.encode()
files["pwa.js"] = (b"if('serviceWorker' in navigator){addEventListener('load',function(){"
                   b"navigator.serviceWorker.register('sw.js',{scope:'./'}).catch(function(){})})}\n")
files["manifest.webmanifest"] = (json.dumps({
    "name": "VERIGA — verificator de legături / link checker", "short_name": "VERIGA",
    "description": "Link checker that runs entirely in your browser; works offline once installed.",
    "id": "./", "start_url": "./", "scope": "./", "display": "standalone",
    "background_color": "#0d1620", "theme_color": "#0d1620", "lang": "ro",
    "icons": [
        {"src": "icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any"},
        {"src": "icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any"},
        {"src": "icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable"}],
}, indent=2, ensure_ascii=False) + "\n").encode()

shell = ["./", "index.html", "pwa.js", "manifest.webmanifest", "icon-192.png", "icon-512.png", "icon-maskable-512.png", "apple-touch-icon.png"]
h = hashlib.sha256()
for k in sorted(files):
    if not k.endswith(".png"): h.update(k.encode() + files[k])  # PNG-urile diferă între versiuni Pillow
cache = f"veriga-{ver}-{h.hexdigest()[:10]}"
files["sw.js"] = f"""'use strict';
// Generat de web/build-pwa.py — nu editați manual.
const CACHE = '{cache}', SHELL = {json.dumps(shell)};
addEventListener('install', e => e.waitUntil(caches.open(CACHE).then(c => c.addAll(SHELL.map(u => new Request(u, {{ cache: 'reload' }})))).then(() => skipWaiting())));
addEventListener('activate', e => e.waitUntil(caches.keys().then(ks => Promise.all(ks.filter(k => k.startsWith('veriga-') && k !== CACHE).map(k => caches.delete(k)))).then(() => clients.claim())));
addEventListener('fetch', e => {{
  const u = new URL(e.request.url);
  // doar fișierele aplicației; verificările de linkuri (alte origini) merg direct la rețea
  if (e.request.method !== 'GET' || u.origin !== location.origin) return;
  e.respondWith(caches.match(e.request, {{ ignoreSearch: true }}).then(r => r || fetch(e.request)));
}});
""".encode()

bad = 0
for k, v in files.items():
    p = os.path.join(OUT, k)
    if "--check" in sys.argv:
        if not os.path.exists(p): print("lipsește:", k); bad = 1
        elif k.endswith(".png"):  # codarea PNG depinde de versiunea Pillow: comparăm doar dimensiunile
            if Image.open(p).size != Image.open(io.BytesIO(v)).size: print("dimensiune diferită:", k); bad = 1
        elif open(p, "rb").read() != v: print("diferă:", k); bad = 1
    else:
        os.makedirs(OUT, exist_ok=True); open(p, "wb").write(v)
print("OK" if not bad else "PWA învechit: rulați python3 web/build-pwa.py"); sys.exit(bad)
