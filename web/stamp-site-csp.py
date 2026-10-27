#!/usr/bin/env python3
"""Inserează/actualizează meta CSP (cu hash) în index.html: `python3 web/stamp-site-csp.py index.html [--check]`."""
import base64, hashlib, re, sys

path = sys.argv[1]
check = '--check' in sys.argv
s = open(path, encoding='utf-8').read()
s = re.sub(r'<meta http-equiv="Content-Security-Policy"[^>]*>\n?', '', s)
scripts = [m.group(1) for m in re.finditer(r'<script(?![^>]*\btype=)[^>]*>(.*?)</script>', s, re.S)]
styles = [m.group(1) for m in re.finditer(r'<style[^>]*>(.*?)</style>', s, re.S)]
h = lambda t: "'sha256-" + base64.b64encode(hashlib.sha256(t.encode('utf-8')).digest()).decode() + "'"
csp = ("default-src 'none'; script-src " + ' '.join(h(t) for t in scripts) + "; style-src " + ' '.join(h(t) for t in styles) +
       "; img-src 'self' data:; font-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; manifest-src 'none'")
meta = '<meta http-equiv="Content-Security-Policy" content="' + csp + '">\n'
new = s.replace('<meta charset="utf-8">\n', '<meta charset="utf-8">\n' + meta, 1)
if check:
    ok = new == open(path, encoding='utf-8').read()
    print('CSP ok' if ok else 'CSP învechit: rulează python3 web/stamp-site-csp.py index.html')
    sys.exit(0 if ok else 1)
open(path, 'w', encoding='utf-8').write(new)
print('CSP:', csp[:90], '…')
