#!/usr/bin/env python3
"""Construiește pluginul WordPress „VERIGA Link Checker”.

  python3 packaging/wordpress/build.py          # sincronizează assets/veriga.html + .mo și scrie dist/*.zip
  python3 packaging/wordpress/build.py --check  # verifică doar că totul e sincron (pentru CI)
"""
import hashlib, os, re, struct, sys, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
PLUGIN = os.path.join(HERE, 'veriga-link-checker')
SLUG = 'veriga-link-checker'


def version():
    m = re.search(r'^version\s*=\s*"([^"]+)"', open(os.path.join(ROOT, 'Cargo.toml'), encoding='utf-8').read(), re.M)
    return m.group(1)


def parse_po(path):
    """Mic parser .po (msgid / msgid_plural / msgstr / msgstr[n], linii continuate)."""
    entries, cur, field = [], {}, None

    def unq(s):
        return bytes(s[1:-1], 'utf-8').decode('unicode_escape').encode('latin-1').decode('utf-8') if '\\' in s else s[1:-1]

    for line in open(path, encoding='utf-8'):
        line = line.rstrip('\n')
        if not line.strip() or line.startswith('#'):
            if line.strip() == '' and cur:
                entries.append(cur); cur, field = {}, None
            continue
        m = re.match(r'(msgid_plural|msgid|msgstr(?:\[(\d+)\])?)\s+(".*")$', line)
        if m:
            key = m.group(1) if m.group(2) is None else 'str%s' % m.group(2)
            key = {'msgid': 'id', 'msgid_plural': 'plural', 'msgstr': 'str0'}.get(key, key.replace('msgstr[', 'str').replace(']', ''))
            if key == 'id' and 'id' in cur:
                entries.append(cur); cur = {}
            cur[key] = unq(m.group(3)); field = key
        elif line.startswith('"') and field:
            cur[field] += unq(line)
    if cur:
        entries.append(cur)
    return entries


def build_mo(po_path):
    msgs = {}
    for e in parse_po(po_path):
        strs = [e[k] for k in sorted((k for k in e if k.startswith('str')), key=lambda k: int(k[3:]))]
        if 'plural' in e:
            msgs[e['id'] + '\0' + e['plural']] = '\0'.join(strs)
        else:
            msgs[e['id']] = strs[0]
    msgs = {k: v for k, v in msgs.items() if v or k == ''}
    keys = sorted(msgs)
    ids = b''; strs = b''; offs = []
    for k in keys:
        kb, vb = k.encode('utf-8'), msgs[k].encode('utf-8')
        offs.append((len(ids), len(kb), len(strs), len(vb)))
        ids += kb + b'\0'; strs += vb + b'\0'
    n = len(keys); start = 7 * 4 + n * 16
    out = struct.pack('<7I', 0x950412de, 0, n, 28, 28 + n * 8, 0, 0)
    out += b''.join(struct.pack('<2I', l, start + o) for o, l, _, _ in offs)
    out += b''.join(struct.pack('<2I', l, start + len(ids) + o) for _, _, o, l in offs)
    return out + ids + strs


def targets():
    """Fișiere derivate: {cale: conținut-așteptat}."""
    t = {}
    t[os.path.join(PLUGIN, 'assets', 'veriga.html')] = open(os.path.join(ROOT, 'web', 'veriga.html'), 'rb').read()
    for po in os.listdir(os.path.join(PLUGIN, 'languages')):
        if po.endswith('.po'):
            t[os.path.join(PLUGIN, 'languages', po[:-3] + '.mo')] = build_mo(os.path.join(PLUGIN, 'languages', po))
    return t


def check_versions():
    v = version(); bad = []
    main = open(os.path.join(PLUGIN, SLUG + '.php'), encoding='utf-8').read()
    rd = open(os.path.join(PLUGIN, 'readme.txt'), encoding='utf-8').read()
    for name, pat, txt in [('Version (antet)', r'^\s*\*\s*Version:\s*(\S+)', main), ('VERIGA_LC_VERSION', r"define\(\s*'VERIGA_LC_VERSION',\s*'([^']+)'", main), ('Stable tag', r'^Stable tag:\s*(\S+)', rd)]:
        m = re.search(pat, txt, re.M)
        if not m or m.group(1) != v:
            bad.append('%s = %s (Cargo.toml: %s)' % (name, m.group(1) if m else '?', v))
    return v, bad


def main():
    check = '--check' in sys.argv
    v, bad = check_versions()
    for b in bad:
        print('VERSIUNE NESINCRONIZATĂ:', b); 
    t = targets(); stale = []
    for p, data in t.items():
        cur = open(p, 'rb').read() if os.path.exists(p) else None
        if cur != data:
            stale.append(os.path.relpath(p, ROOT))
            if not check:
                open(p, 'wb').write(data)
    if check:
        if bad or stale:
            for s in stale: print('NEACTUALIZAT:', s)
            print('Rulează: python3 packaging/wordpress/build.py'); sys.exit(1)
        print('OK'); return
    if bad: sys.exit(1)
    dist = os.path.join(ROOT, 'dist'); os.makedirs(dist, exist_ok=True)
    zp = os.path.join(dist, '%s-%s.zip' % (SLUG, v))
    files = []
    for d, _, fs in os.walk(PLUGIN):
        for f in fs:
            files.append(os.path.join(d, f))
    with zipfile.ZipFile(zp, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for f in sorted(files):
            if f.endswith('.po') and False: continue
            zi = zipfile.ZipInfo(SLUG + '/' + os.path.relpath(f, PLUGIN).replace(os.sep, '/'), (2026, 1, 1, 0, 0, 0))
            zi.compress_type = zipfile.ZIP_DEFLATED; zi.external_attr = 0o644 << 16
            z.writestr(zi, open(f, 'rb').read())
    print('%s  %d octeți  sha256 %s' % (os.path.relpath(zp, ROOT), os.path.getsize(zp), hashlib.sha256(open(zp, 'rb').read()).hexdigest()))


if __name__ == '__main__':
    main()
