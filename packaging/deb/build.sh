#!/bin/bash
# Construiește .deb și .rpm din binarul musl static deja compilat.
# Utilizare: packaging/deb/build.sh <versiune> <rust-target> <binar> <outdir>
#   ex.: packaging/deb/build.sh 1.0.2 x86_64-unknown-linux-musl target/x86_64-unknown-linux-musl/release/veriga dist
set -euo pipefail
ver=$1; target=$2; bin=$3; out=$4
root=$(cd "$(dirname "$0")/../.." && pwd)
case $target in
  x86_64-*)  darch=amd64;  rarch=x86_64 ;;
  aarch64-*) darch=arm64;  rarch=aarch64 ;;
  *) echo "țintă neacceptată: $target" >&2; exit 2 ;;
esac
mkdir -p "$out"; out=$(cd "$out" && pwd)
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT

stage() {  # $1 = rădăcină de instalare
  install -Dm0755 "$bin" "$1/usr/bin/veriga"
  install -Dm0644 "$root/web/veriga.html" "$1/usr/share/veriga/veriga.html"
  install -Dm0644 "$root/LICENSE" "$1/usr/share/doc/veriga/copyright"
  install -Dm0644 "$root/README.md" "$1/usr/share/doc/veriga/README.md"
  install -Dm0644 "$root/packaging/systemd/veriga-watch.service" "$1/usr/lib/systemd/system/veriga-watch.service"
  install -Dm0644 "$root/packaging/systemd/veriga-watch.timer" "$1/usr/lib/systemd/system/veriga-watch.timer"
  install -Dm0644 "$root/packaging/systemd/guardian.example.json" "$1/usr/share/doc/veriga/guardian.example.json"
  install -d "$1/etc/veriga"
}

# ── .deb ──
d="$t/deb"; stage "$d"
mkdir -p "$d/DEBIAN"
cat > "$d/DEBIAN/control" <<CTL
Package: veriga
Version: $ver
Section: web
Priority: optional
Architecture: $darch
Maintainer: Alexio <alexio@trom.tf>
Homepage: https://veriga.chiuta.ro
Description: link checker (Xenu-like) with scheduled Link Guardian
 Finds broken links, redirects and slow pages; static binary, no dependencies.
 Enable the daily Guardian with: systemctl enable --now veriga-watch.timer
 after creating /etc/veriga/guardian.json (see /usr/share/doc/veriga).
CTL
cat > "$d/DEBIAN/postinst" <<'PI'
#!/bin/sh
set -e
[ -d /run/systemd/system ] && systemctl daemon-reload || true
exit 0
PI
cat > "$d/DEBIAN/prerm" <<'PR'
#!/bin/sh
set -e
# la dezinstalare oprim cronometrul Guardian (altfel ar rămâne activ fără binar)
if [ "$1" = remove ] || [ "$1" = purge ]; then
  [ -d /run/systemd/system ] && systemctl disable --now veriga-watch.timer 2>/dev/null || true
fi
exit 0
PR
cat > "$d/DEBIAN/postrm" <<'PO'
#!/bin/sh
set -e
[ -d /run/systemd/system ] && systemctl daemon-reload || true
exit 0
PO
chmod 0755 "$d/DEBIAN/postinst" "$d/DEBIAN/prerm" "$d/DEBIAN/postrm"
find "$d" -type d -exec chmod 0755 {} +
dpkg-deb --root-owner-group --build "$d" "$out/veriga_${ver}_${darch}.deb" >/dev/null
echo "$out/veriga_${ver}_${darch}.deb"

# ── .rpm ──
if command -v rpmbuild >/dev/null; then
  r="$t/rpm"; mkdir -p "$r"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,stage}
  stage "$r/stage"
  cat > "$r/SPECS/veriga.spec" <<SPEC
Name: veriga
Version: $ver
Release: 1
Summary: Link checker (Xenu-like) with scheduled Link Guardian
License: MIT
URL: https://veriga.chiuta.ro
BuildArch: $rarch
AutoReqProv: no
%global debug_package %{nil}
%global __strip /bin/true
%description
Finds broken links, redirects and slow pages; static binary, no dependencies.
%post
[ -d /run/systemd/system ] && systemctl daemon-reload >/dev/null 2>&1 || :
%preun
if [ "\$1" = 0 ] && [ -d /run/systemd/system ]; then systemctl disable --now veriga-watch.timer >/dev/null 2>&1 || :; fi
%postun
[ -d /run/systemd/system ] && systemctl daemon-reload >/dev/null 2>&1 || :
%install
cp -a $r/stage/. %{buildroot}/
%files
/usr/bin/veriga
/usr/share/veriga
/usr/share/doc/veriga
/usr/lib/systemd/system/veriga-watch.service
/usr/lib/systemd/system/veriga-watch.timer
%dir /etc/veriga
SPEC
  rpmbuild --target "$rarch" --define "_topdir $r" -bb "$r/SPECS/veriga.spec" >/dev/null
  cp "$r"/RPMS/*/veriga-"$ver"-1.*.rpm "$out/"
  ls "$out"/veriga-"$ver"-1.*.rpm
fi
