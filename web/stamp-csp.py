#!/usr/bin/env python3
"""Calculează hash-urile SHA-256 ale blocului <script> și <style> din veriga.html și
le scrie în meta CSP (fără 'unsafe-inline' pentru cod; atributele style rămân permise).
Rulează după fiecare modificare:  python3 web/stamp-csp.py web/veriga.html"""
import base64, hashlib, re, sys
p = sys.argv[1] if len(sys.argv) > 1 else "veriga.html"
s = open(p, encoding="utf-8").read()
def h(txt): return "'sha256-" + base64.b64encode(hashlib.sha256(txt.encode()).digest()).decode() + "'"
head, body = s.split("<body>", 1)
sty = re.search(r"<style>(.*?)</style>", head, re.S).group(1)
def inline_scripts(doc):
    """Toate scripturile inline executabile (și stratul fff-compat / modulul fff-imp din <head>, adăugate de tools/compat/lower.mjs)."""
    out = []
    for m in re.finditer(r"<script\b([^>]*)>(.*?)</script>", doc, re.S):
        if re.search(r"\bsrc\s*=", m.group(1)): continue
        t = re.search(r"\btype\s*=\s*[\"']?([^\"'\s>]*)", m.group(1))
        if t and t.group(1).lower() not in ("", "text/javascript", "application/javascript", "module"): continue
        out.append(m.group(2))
    return out
scripts = inline_scripts(s)
assert scripts, "lipsesc blocurile <script>"
csp = ("default-src 'none'; script-src " + " ".join(h(x) for x in scripts) + "; style-src-elem " + h(sty) +
       "; style-src-attr 'unsafe-inline'; img-src data: blob:; connect-src http: https: blob: data:; "
       "base-uri 'none'; form-action 'none'; object-src 'none'")
s2, n = re.subn(r'(<meta http-equiv="Content-Security-Policy" content=")[^"]*(")', lambda m: m.group(1) + csp + m.group(2), s, count=1)
assert n == 1, "lipsește meta CSP"
open(p, "w", encoding="utf-8").write(s2)
print("CSP actualizat")
