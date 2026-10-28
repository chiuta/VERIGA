#!/usr/bin/env python3
"""Asamblează pachetul VERIGAPortable (PortableApps.com Format 3.9) din arhivele Windows ale release-ului.

  python3 portableapps/build.py --launcher VERIGAPortable.exe --x64 veriga-x86_64-pc-windows-msvc.zip \\
      [--arm64 veriga-aarch64-pc-windows-msvc.zip] --out dist/VERIGAPortable_1.0.0_Multilingual.zip
"""
import argparse, os, re, shutil, subprocess, sys, tempfile, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
TEMPLATE = os.path.join(HERE, "VERIGAPortable")
KEEP = {"veriga.exe", "veriga-gui.exe", "README.md", "LICENSE", "SECURITY.md", "veriga.html"}


def extract(zpath, dest):
    os.makedirs(dest, exist_ok=True)
    with zipfile.ZipFile(zpath) as z:
        for m in z.infolist():
            name = os.path.basename(m.filename)
            if m.is_dir() or name not in KEEP:
                continue
            with z.open(m) as src, open(os.path.join(dest, name), "wb") as dst:
                shutil.copyfileobj(src, dst)
    for need in ("veriga.exe", "veriga-gui.exe"):
        if not os.path.isfile(os.path.join(dest, need)):
            sys.exit(f"{zpath}: lipsește {need}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--launcher", required=True)
    ap.add_argument("--x64", required=True)
    ap.add_argument("--arm64")
    ap.add_argument("--out", required=True)
    ap.add_argument("--version", help="PackageVersion 1.2.3.4 (implicit: din appinfo.ini)")
    a = ap.parse_args()
    with tempfile.TemporaryDirectory() as t:
        pkg = os.path.join(t, "VERIGAPortable")
        shutil.copytree(TEMPLATE, pkg)
        shutil.copy(a.launcher, os.path.join(pkg, "VERIGAPortable.exe"))
        extract(a.x64, os.path.join(pkg, "App", "VERIGA"))
        if a.arm64:
            extract(a.arm64, os.path.join(pkg, "App", "VERIGAARM64"))
        else:
            # fără ARM64 nativ, lansatorul folosește x64 (rulează emulat): eliminăm BaseAppIDARM64
            ini = os.path.join(pkg, "App", "AppInfo", "appinfo.ini")
            s = open(ini, encoding="utf-8").read()
            open(ini, "w", encoding="utf-8").write(re.sub(r"^BaseAppIDARM64=.*\n", "", s, flags=re.M))
        # versiunea vine din Cargo.toml (workspace): 1.2.3 -> PackageVersion 1.2.3.0, DisplayVersion 1.2.3
        ver = a.version
        if not ver:
            cargo = open(os.path.join(HERE, "..", "Cargo.toml"), encoding="utf-8").read()
            ver = re.search(r'^version = "(\d+\.\d+\.\d+)"', cargo, re.M).group(1) + ".0"
        ini = os.path.join(pkg, "App", "AppInfo", "appinfo.ini")
        s = open(ini, encoding="utf-8").read()
        s = re.sub(r"^PackageVersion=.*$", f"PackageVersion={ver}", s, flags=re.M)
        s = re.sub(r"^DisplayVersion=.*$", f"DisplayVersion={'.'.join(ver.split('.')[:3])}", s, flags=re.M)
        open(ini, "w", encoding="utf-8").write(s)
        os.makedirs(os.path.join(pkg, "Other", "Help", "Images"), exist_ok=True)
        os.makedirs(os.path.join(pkg, "Data"), exist_ok=True)
        os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
        with zipfile.ZipFile(a.out, "w", zipfile.ZIP_DEFLATED) as z:
            for d, _, files in os.walk(pkg):
                rel = os.path.relpath(d, t)
                if not files:
                    z.writestr(rel.replace(os.sep, "/") + "/", "")
                for f in sorted(files):
                    z.write(os.path.join(d, f), os.path.join(rel, f).replace(os.sep, "/"))
        subprocess.run([sys.executable, os.path.join(HERE, "validate.py"), pkg, "--package"], check=True)
    print("scris:", a.out)


if __name__ == "__main__":
    main()
