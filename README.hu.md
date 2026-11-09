# VERIGA — hivatkozásellenőrző (független megvalósítás)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · **Magyar**

**Letöltés:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Verzió: 1.1.0 · Utolsó frissítés: 2026.10.29. · Bemutatkozó oldal: https://veriga.chiuta.ro

Egy 64 bites Rust alkalmazás Windowsra, macOS-re és Linuxra (x86-64 és ARM64), amely bejár egy weboldalt
vagy egy helyi mappát, és ellenőrzi a hivatkozásait. Három formában érhető el, amelyek egyetlen közös eredménymodellen osztoznak:

| Form | Hol | Mit csinál |
|---|---|---|
| `veriga` (CLI) | terminál, CI, konténerek (statikus musl bináris) | teljes ellenőrzés, jelentések, kilépési kód |
| `veriga-gui` | natív asztali alkalmazás (egui) | rendezhető/szűrhető táblázat, részletek panel, szünet/leállítás, exportálás |
| `web/veriga.html` | bármely böngésző, egyetlen fájl, offline | helyi mappa elemzése, online oldal (a CORS korlátain belül), URL-lista |

A munkamenetek (`.veriga`, JSON) felcserélhetők a három forma között.

## A „clean-room” címkéről

A VERIGA **független megvalósítás**, nem dokumentált clean-room projekt. Nem alkalmaztak formális eljárást (külön elemző és megvalósító csapat, ellenőrzött specifikációk), és a tároló története egyetlen nagy commit-tal kezdődik. A kód a hivatkozás-ellenőrzők általános viselkedéséből és nyilvános specifikációkból (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org) készült. A szerző fejlesztési munkamenetei nem mutatják más hivatkozás-ellenőrző kódjának vagy futtatható állományának használatát. A származási nyilvántartás és a nyitott pontok: [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

## Biztonság és fenyegetésmodell

A VERIGA nem megbízható tartalmat dolgoz fel (weboldalak, exportált fájlok, konfigurációs fájlok). Alapértelmezett védelmek:

- **Anti-SSRF:** a privát/helyi célpontok (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) tiltottak, átirányítás után is. Kifejezett kikapcsolás: `--allow-private` (CLI) / a GUI jelölőnégyzete. *Korlát:* a DNS rebinding az ellenőrzés és a kapcsolódás között nincs teljesen lezárva.
- **Érzékeny fejlécek** (`Authorization`, `Cookie`, API-kulcsok) csak a belső origin felé kerülnek elküldésre, soha más gazdagépeknek vagy külső átirányítás után.
- **Injekciószűrő a javításokhoz:** az idézőjelet, `<`, `>` jelet, szóközt, backtick-et, visszaperjelet vagy Markdownban zárójelet tartalmazó új URL-t (pl. egy rosszindulatú `Location`) elutasítja (`unsafe_url_for` / `gUnsafeUrl`).
- **Biztonságos írás:** egyedi biztonsági mentés, atomi írás, kooperatív zárolás, a tartalom újraellenőrzése alkalmazás előtt (TOCTOU), jogosultságok/szimbolikus linkek megőrzése; az elavult javaslatok eldobásra kerülnek, nem alkalmazódnak rossz pozíciókon.
- **CSV-export** képletek ellen védve (`=`, `+`, `-`, `@`).
- **Értesítések:** a `sendmail` értékének abszolút útvonalnak kell lennie; ne használj nem megbízható forrásból származó konfigurációs fájlokat (egy választott bináris futtatását válthatják ki).
- **Ismert korlátok:** a WXR-ben escape-elt HTML nincs átvizsgálva (hamis negatívok); a webes kiadást a CORS korlátozza; a `lint` 2-es kóddal lép ki, ha vannak találatai.

## Szabványok és megfelelőség (ellenőrizve: 2026.10.29.)

| Terület | Állapot |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (mag + CLI; függőségeik legfeljebb 1.88-at deklarálnak), GUI **1.95** (egui 0.37); a `cargo clippy -- -D warnings` és a `cargo fmt --check` tiszta; `--locked` a CI-ben |
| Függőségek | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 sebezhetőség; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: összevont csoportok, pontos egyezés a termék-tokenre, százalék/UTF-8 normalizálás, `*` és `$`, a leghosszabb szabály nyer (egyenlőségnél Allow), 500 KiB korlát, 4xx ⇒ engedélyezett, 5xx/hálózati hiba ⇒ nem elérhető, az ellenőrzés folytatódik (egy halott oldal hibásként jelenik meg, nem „0 hibás”-ként); a kezdőoldalt a robots.txt soha nem blokkolja |
| HTTP | RFC 9110: `Retry-After` (másodperc/dátum) 429/503 esetén, 301/302/303/307/308 átirányítások, relatív `Location`, `HEAD` `GET`-re való visszalépéssel |
| URL | WHATWG URL / IDNA (UTS 46) az `url` crate-en keresztül (az `idna`-val) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 szabálysértés, világos/sötét téma), WAI-ARIA APG fülek billentyűzettel, `prefers-reduced-motion`, `forced-colors`, célméret ≥ 24 px, `status`/`progressbar` régiók; **CSP** hash-ekkel (szkriptekhez nincs `unsafe-inline`), `referrer: no-referrer`; az exportált jelentések saját CSP-t hordoznak |
| Ellátási lánc | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, kiadások SHA-256 összegekkel és származási igazolással (Sigstore/SLSA), harmadik féltől származó akciók nélkül a publikálásban |

Webes tesztek: `cd web/tests && npm install && npx playwright install chromium && npm test`. A `veriga.html` bármilyen módosítása után: `python3 web/stamp-csp.py web/veriga.html`.
