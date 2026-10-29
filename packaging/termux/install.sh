#!/data/data/com.termux/files/usr/bin/sh
# Instalează VERIGA (CLI static aarch64) în Termux, cu verificarea sumei SHA-256.
#   curl -fsSL https://raw.githubusercontent.com/chiuta/VERIGA/main/packaging/termux/install.sh | sh
#   sau cu versiune fixată:  VERIGA_VERSION=v1.0.2 sh install.sh
set -eu
VER=${VERIGA_VERSION:-v1.0.2}
PREFIX=${PREFIX:-/data/data/com.termux/files/usr}
case "$(uname -m)" in
  aarch64|arm64) T=aarch64-unknown-linux-musl ;;
  x86_64)        T=x86_64-unknown-linux-musl ;;
  *) echo "Arhitectură neacceptată: $(uname -m) (necesită aarch64 sau x86_64)" >&2; exit 1 ;;
esac
BASE=https://github.com/chiuta/VERIGA/releases/download/$VER
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
cd "$TMP"
curl -fsSL -o "veriga-$T.tar.gz" "$BASE/veriga-$T.tar.gz"
curl -fsSL -o SHA256SUMS "$BASE/SHA256SUMS"
want=$(grep " veriga-$T.tar.gz\$" SHA256SUMS | cut -d' ' -f1)
[ -n "$want" ] || { echo "Suma lipsește din SHA256SUMS" >&2; exit 1; }
have=$(sha256sum "veriga-$T.tar.gz" | cut -d' ' -f1)
[ "$want" = "$have" ] || { echo "SUMĂ DE CONTROL INVALIDĂ — instalare oprită" >&2; exit 1; }
tar xzf "veriga-$T.tar.gz"
install -m 0755 "veriga-$T/veriga" "$PREFIX/bin/veriga"
mkdir -p "$HOME/veriga"; cp "veriga-$T/veriga.html" "$HOME/veriga/veriga.html"
echo "OK: $("$PREFIX/bin/veriga" --version)"
echo "Ediția web: $HOME/veriga/veriga.html (deschideți-o în browser sau instalați PWA de pe https://veriga.chiuta.ro/app/)"
echo "Programare zilnică: pkg install cronie termux-services && sv-enable crond && crontab -e"
echo "  17 4 * * * veriga watch --once --config \$HOME/veriga/guardian.json"
