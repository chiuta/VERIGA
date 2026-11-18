#!/usr/bin/env python3
"""După publicarea unui release: pune în repo valorile reale ale activelor lui.

  python3 packaging/release-sync.py v1.5.0           # actualizează fișierele
  python3 packaging/release-sync.py v1.5.0 --check   # doar arată ce s-ar schimba (cod de ieșire 1 dacă ceva diferă)

Ce face (aceleași două lucruri pe care le făceau, de mână, commiturile „site: mărimile vX; YunoHost: sume SHA-256 pentru vX”):
  1. packaging/yunohost/manifest.toml: sha256 pentru amd64 și arm64, din SHA256SUMS-ul release-ului;
  2. index.html: mărimea fiecărui fișier din lista „Descărcări directe” (octeți / 10^6, o zecimală), apoi
     regenerează paginile pe limbă (web/build-i18n.py) și hash-urile CSP (web/stamp-site-csp.py) și rulează
     packaging/yunohost/check.py.

Citește numai API-ul public GitHub (și adresa de descărcare a lui SHA256SUMS); nu scrie nimic în GitHub.
Variabila GITHUB_TOKEN, dacă există, e folosită doar ca să nu atingi limita de cereri anonime.
Cere ca release-ul să fie PUBLICAT (o ciornă nu apare la /releases/tags/…) și ca Cargo.toml să aibă aceeași versiune.
"""
import json, os, re, subprocess, sys, urllib.request

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.dirname(here)
REPO = "chiuta/VERIGA"
MANIFEST = os.path.join(here, "yunohost", "manifest.toml")
SITE = os.path.join(root, "index.html")
PAGES = ["index.html"] + [f"{l}/index.html" for l in ("en", "fr", "it", "es", "pt", "de", "hu")]
# arhiva pe care o descarcă YunoHost pentru fiecare arhitectură
ARCH_ASSET = {"amd64": "veriga-x86_64-unknown-linux-musl.tar.gz", "arm64": "veriga-aarch64-unknown-linux-musl.tar.gz"}


def fail(msg):
    print("EROARE:", msg, file=sys.stderr)
    sys.exit(2)


def get(url, accept=None):
    req = urllib.request.Request(url, headers={"User-Agent": "veriga-release-sync"})
    if accept: req.add_header("Accept", accept)
    if os.environ.get("GITHUB_TOKEN") and "api.github.com" in url:
        req.add_header("Authorization", "Bearer " + os.environ["GITHUB_TOKEN"])
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def mb(n):
    return f"{n / 1e6:.1f} MB"


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    check = "--check" in argv
    repo = REPO
    for a in argv:
        if a.startswith("--repo="): repo = a.split("=", 1)[1]
    if len(args) != 1 or not re.fullmatch(r"v\d+\.\d+\.\d+", args[0]):
        fail("folosire: python3 packaging/release-sync.py vX.Y.Z [--check] [--repo=proprietar/depozit]")
    tag = args[0]
    ver = tag[1:]
    cargo = re.search(r'^version = "([^"]+)"', open(os.path.join(root, "Cargo.toml"), encoding="utf-8").read(), re.M).group(1)
    if cargo != ver: fail(f"Cargo.toml are versiunea {cargo}, nu {ver}: rulează scriptul din commitul care a fost publicat")

    try:
        rel = json.loads(get(f"https://api.github.com/repos/{repo}/releases/tags/{tag}"))
    except Exception as e:
        fail(f"nu pot citi release-ul {tag} din {repo} ({e}); o ciornă sau un release nepublicat nu se vede aici")
    if rel.get("draft"): fail(f"release-ul {tag} e încă ciornă")
    assets = {a["name"]: a for a in rel["assets"]}
    if "SHA256SUMS" not in assets: fail("release-ul nu are SHA256SUMS")
    sums = {}
    for line in get(assets["SHA256SUMS"]["browser_download_url"]).decode("utf-8").splitlines():
        m = re.fullmatch(r"([0-9a-f]{64})\s+\*?(\S+)", line.strip())
        if m: sums[m.group(2)] = m.group(1)

    # 1) manifestul YunoHost
    man = open(MANIFEST, encoding="utf-8").read()
    new_man = man
    for arch, name in ARCH_ASSET.items():
        if name not in sums: fail(f"{name} lipsește din SHA256SUMS")
        url = re.search(rf'^\s*{arch}\.url = "([^"]+)"', man, re.M)
        if not url or not url.group(1).endswith(f"/{tag}/{name}"): fail(f"manifestul: adresa {arch} nu indică {tag}/{name}")
        new_man, n = re.subn(rf'^(\s*{arch}\.sha256 = ")[0-9a-f]{{64}}(")', rf"\g<1>{sums[name]}\g<2>", new_man, flags=re.M)
        if n != 1: fail(f"manifestul: linia {arch}.sha256 nu a fost găsită exact o dată")

    # 2) mărimile din lista de descărcări (doar rândurile al căror nume e un activ al release-ului)
    site = open(SITE, encoding="utf-8").read()
    changed = []
    seen = []

    def size_sub(m):
        name = m.group(2)
        if name not in assets: return m.group(0)
        seen.append(name)
        new = mb(assets[name]["size"])
        if new != m.group(3): changed.append(f"{name}: {m.group(3)} → {new}")
        return m.group(1) + new + m.group(4)

    new_site = re.sub(r'(<code>([^<]+)</code><span class="dlsz">)([^<]*)(</span>)', size_sub, site)
    if not seen: fail("nu am găsit niciun rând de descărcare cu mărime în index.html")
    listed = set(re.findall(r'releases/download/' + re.escape(tag) + r'/([^"]+)"', site))
    missing = sorted(n for n in listed if n not in assets)
    if missing: fail("fișiere din lista de pe site care nu există în release: " + ", ".join(missing))
    unsized = sorted(n for n in listed if n not in seen and n != "SHA256SUMS")
    if unsized: fail("fișiere din lista de pe site fără mărime afișată: " + ", ".join(unsized))

    man_changed = new_man != man
    if man_changed: print("manifest.toml: sume SHA-256 actualizate")
    for c in changed: print("index.html:", c)
    if not man_changed and not changed: print("nimic de schimbat: valorile din repo corespund deja release-ului", tag)
    if check: sys.exit(1 if (man_changed or changed) else 0)

    if man_changed: open(MANIFEST, "w", encoding="utf-8").write(new_man)
    if changed:
        open(SITE, "w", encoding="utf-8").write(new_site)
        run = lambda *a: subprocess.run([sys.executable, *a], cwd=root, check=True, stdout=subprocess.DEVNULL)
        run("web/build-i18n.py")
        for p in PAGES: run("web/stamp-site-csp.py", p)
        run("web/build-i18n.py", "--check")
    subprocess.run([sys.executable, os.path.join("packaging", "yunohost", "check.py")], cwd=root, check=True)
    print("gata; verifică cu `git diff --stat` și comite")


if __name__ == "__main__":
    main(sys.argv[1:])
