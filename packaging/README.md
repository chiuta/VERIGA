# Packaging — mobile & server

| Target | Where | Status |
|---|---|---|
| **PWA** (Android/iOS/desktop, offline) | `app/` ← `python3 web/build-pwa.py`; served at https://veriga.chiuta.ro/app/ | tested: Chromium, service worker + offline reload |
| **Termux** (Android CLI) | `termux/install.sh` (SHA-256-verified) | script syntax-checked only; not run on a device |
| **Docker / GHCR** (amd64+arm64, non-root, scratch) | `../Dockerfile`, `docker/Dockerfile.release`, `docker/docker-compose.yml` | built and smoke-tested in CI; published on release tags |
| **systemd** (oneshot + timer, hardened) | `systemd/` | `systemd-analyze verify` in CI |
| **.deb / .rpm** (amd64+arm64) | `deb/build.sh` | `.deb` installed in CI; `.rpm` inspected only |
| **Kubernetes** CronJob | `k8s/cronjob.yaml` | YAML-parsed in CI; not applied to a cluster |
| **WordPress** plugin (WP ≥ 6.2, tested up to 7.1.4, PHP ≥ 7.4) | `wordpress/veriga-link-checker/` ← `python3 wordpress/build.py` (ZIP in `dist/`) | PHP-linted; logic tested with WP stubs against the real binary (`wordpress/tests/`) in CI; **not yet run inside a live WordPress** |
| **YunoHost** (package format v2) | `yunohost/` | structure checked (`yunohost/check.py`); **`package_check` not yet run on a live YunoHost** |

A native Android APK is **not** provided: use the PWA (install from the browser menu) or Termux.
