#!/usr/bin/env python3
"""Generează paginile statice pe limbă (en/, fr/, it/, es/, pt/, de/, hu/) din index.html (sursa românească),
adaugă hreflang în toate și scrie sitemap.xml cu alternative.
Rulare:  python3 web/build-i18n.py [--check]        (apoi python3 web/stamp-site-csp.py pentru fiecare pagină)
Fiecare pagină are textul tradus direct în HTML (vizibil fără JavaScript și pentru motoare de căutare)."""
import html, json, os, pathlib, re, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__)); root = os.path.dirname(here)
CHECK = '--check' in sys.argv
SITE = 'https://veriga.chiuta.ro/'
LANGS = ['en', 'fr', 'it', 'es', 'pt', 'de', 'hu']
LOCALE = {'ro': 'ro_RO', 'en': 'en_US', 'fr': 'fr_FR', 'it': 'it_IT', 'es': 'es_ES', 'pt': 'pt_PT', 'de': 'de_DE', 'hu': 'hu_HU'}
src_path = os.path.join(root, 'index.html')
src = pathlib.Path(src_path).read_text(encoding='utf-8')

def url(l): return SITE if l == 'ro' else f'{SITE}{l}/'

# sursa: eliminăm hreflang vechi (idempotent)
src = re.sub(r'<link rel="alternate" hreflang="[^"]*" href="[^"]*">\n?', '', src)
alts = ''.join(f'<link rel="alternate" hreflang="{l}" href="{url(l)}">\n' for l in ['ro'] + LANGS) + f'<link rel="alternate" hreflang="x-default" href="{SITE}">\n'
CANON = f'<link rel="canonical" href="{SITE}">\n'
assert CANON in src
src = src.replace(CANON, CANON + alts, count=1)

STR = json.loads(re.search(r'^const STR = (\{.*\});$', src, re.M).group(1))
META = json.loads(re.search(r'^const META = (\{.*\});$', src, re.M).group(1))
EL = re.compile(r'(<([a-zA-Z0-9]+)\b[^>]*?\bdata-ro="([^"]*)"[^>]*?\bdata-en="([^"]*)"[^>]*>)([^<]*)(</\2>)')

def build(l):
    s = src
    def sub(m):
        ro, en = html.unescape(m.group(3)), html.unescape(m.group(4))
        t = en if l == 'en' else STR[l].get(ro, en)
        return m.group(1) + html.escape(t, quote=False) + m.group(6)
    s = EL.sub(sub, s)
    s = s.replace('<html lang="ro">', f'<html lang="{l}">', 1)
    title, desc = META[l]
    s = re.sub(r'<title>.*?</title>', lambda m: f'<title>{html.escape(title, quote=False)}</title>', s, count=1)
    s = re.sub(r'(<meta name="description" content=")[^"]*(")', lambda m: m.group(1) + html.escape(desc) + m.group(2), s, count=1)
    s = re.sub(r'(<meta property="og:title" content=")[^"]*(")', lambda m: m.group(1) + html.escape(title) + m.group(2), s, count=1)
    s = re.sub(r'(<meta property="og:description" content=")[^"]*(")', lambda m: m.group(1) + html.escape(desc) + m.group(2), s, count=1)
    s = s.replace('content="ro_RO"', f'content="{LOCALE[l]}"', 1)
    s = s.replace(f'<meta property="og:url" content="{SITE}">', f'<meta property="og:url" content="{url(l)}">', 1)
    s = s.replace(CANON, f'<link rel="canonical" href="{url(l)}">\n', 1)
    # căi relative: pagina stă într-un subdirector
    s = re.sub(r'(?<=["\'(])assets/', '../assets/', s)
    s = s.replace('href="web/veriga.html"', 'href="../web/veriga.html"').replace('href="app/"', 'href="../app/"')
    # limba este cea a paginii; selectorul duce la pagina limbii alese
    s, n = re.subn(r"let LANG = \(\(\) => \{.*?\}\)\(\);", f"let LANG = '{l}';", s, count=1, flags=re.S); assert n == 1
    s, n = re.subn(r"if \(LANGS\.some\(l => l\[0\] === s\)\) LANG = s;\n?", '', s, count=1); assert n == 1
    s, n = re.subn(r"\$\('#lang'\)\.addEventListener\('change', e => \{ LANG = e\.target\.value; lsSet\('veriga\.site\.lang', LANG\); applyLang\(\); \}\);",
                   "$('#lang').addEventListener('change', e => { const n = e.target.value; lsSet('veriga.site.lang', n); location.href = n === 'ro' ? '../' : (n === LANG ? '' : '../' + n + '/'); });", s, count=1); assert n == 1
    return s

out = {src_path: src}
for l in LANGS:
    out[os.path.join(root, l, 'index.html')] = build(l)
last = re.search(r'<lastmod>([^<]*)</lastmod>', pathlib.Path(root, 'sitemap.xml').read_text()).group(1)
sm = ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">']
for l in ['ro'] + LANGS:
    sm.append(f'  <url><loc>{url(l)}</loc><lastmod>{last}</lastmod>')
    for a in ['ro'] + LANGS:
        sm.append(f'    <xhtml:link rel="alternate" hreflang="{a}" href="{url(a)}"/>')
    sm.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{SITE}"/>')
    sm.append('  </url>')
sm.append('</urlset>')
out[os.path.join(root, 'sitemap.xml')] = '\n'.join(sm) + '\n'

stamp = os.path.join(here, 'stamp-site-csp.py')
def stamped(text):
    """CSP cu hash se calculează pe conținutul final al fiecărei pagini."""
    import tempfile
    with tempfile.NamedTemporaryFile('w', suffix='.html', delete=False, encoding='utf-8') as f:
        f.write(text); p = f.name
    subprocess.run([sys.executable, stamp, p], check=True, stdout=subprocess.DEVNULL)
    r = pathlib.Path(p).read_text(encoding='utf-8'); os.unlink(p); return r

bad = 0
for p, t in out.items():
    if p.endswith('.html'): t = stamped(t)
    if CHECK:
        cur = pathlib.Path(p).read_text(encoding='utf-8') if os.path.exists(p) else None
        if cur != t: print('învechit:', os.path.relpath(p, root)); bad += 1
    else:
        os.makedirs(os.path.dirname(p), exist_ok=True)
        pathlib.Path(p).write_text(t, encoding='utf-8')
print('OK' if not bad else f'{bad} fișiere învechite: rulează python3 web/build-i18n.py')
sys.exit(1 if bad else 0)
