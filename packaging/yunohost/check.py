#!/usr/bin/env python3
"""Verificare statică a pachetului YunoHost (fără instanță YunoHost): manifest, scripturi, sume de control, substituiri."""
import hashlib, os, re, subprocess, sys, tomllib

here = os.path.dirname(os.path.abspath(__file__))
err = []
m = tomllib.load(open(f"{here}/manifest.toml", "rb"))
tomllib.load(open(f"{here}/tests.toml", "rb"))
if m.get("packaging_format") != 2: err.append("packaging_format != 2")
if not re.fullmatch(r"\d+\.\d+\.\d+~ynh\d+", m["version"]): err.append("version")
root = os.path.abspath(f"{here}/../..")
cargo = re.search(r'^version = "([^"]+)"', open(f"{root}/Cargo.toml").read(), re.M).group(1)
if not m["version"].startswith(cargo + "~ynh"): err.append(f"versiunea {m['version']} ≠ Cargo {cargo}")
src = m["resources"]["sources"]["main"]
for a in ("amd64", "arm64"):
    if not re.fullmatch(r"[0-9a-f]{64}", src[a]["sha256"]): err.append(f"sha256 {a}")
    if f"/v{cargo}/" not in src[a]["url"]: err.append(f"url {a} nu indică v{cargo}")
for s in ("install", "remove", "upgrade", "backup", "restore", "change_url"):
    p = f"{here}/scripts/{s}"
    if not os.access(p, os.X_OK): err.append(f"{s} neexecutabil")
    if subprocess.run(["bash", "-n", p]).returncode: err.append(f"{s} sintaxă")
if subprocess.run(["bash", "-n", f"{here}/scripts/_common.sh"]).returncode: err.append("_common.sh")
for t in ("nginx.conf", "veriga-watch.service", "veriga-watch.timer", "guardian.json"):
    if not os.path.isfile(f"{here}/conf/{t}"): err.append(f"conf/{t} lipsește")
for f in os.listdir(f"{here}/conf"):
    for tok in re.findall(r"__[A-Z_]+__", open(f"{here}/conf/{f}").read()):
        if tok not in ("__PATH__", "__INSTALL_DIR__", "__DATA_DIR__", "__APP__", "__GUARDIAN_URL__"): err.append(f"{f}: {tok}")
for d in ("DESCRIPTION.md", "ADMIN.md"):
    if not os.path.isfile(f"{here}/doc/{d}"): err.append(d)
print("OK" if not err else "\n".join(err))
sys.exit(1 if err else 0)
