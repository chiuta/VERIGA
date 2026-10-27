# VERIGA — hivatkozásellenőrző (clean-room)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · **Magyar**

Verzió: 1.0.0 · Utolsó frissítés: 2026.10.27. · Bemutatkozó oldal: https://veriga.chiuta.ro

Egy 64 bites Rust alkalmazás Windowsra, macOS-re és Linuxra (x86-64 és ARM64), amely bejár egy weboldalt
vagy egy helyi mappát, és ellenőrzi a hivatkozásait. Három formában érhető el, amelyek egyetlen közös eredménymodellen osztoznak:

| Form | Where | What it does |
|---|---|---|
| `veriga` (CLI) | terminál, CI, konténerek (statikus musl bináris) | teljes ellenőrzés, jelentések, kilépési kód |
| `veriga-gui` | natív asztali alkalmazás (egui) | rendezhető/szűrhető táblázat, részletek panel, szünet/leállítás, exportálás |
| `web/veriga.html` | bármely böngésző, egyetlen fájl, offline | helyi mappa elemzése, online oldal (a CORS korlátain belül), URL-lista |

A munkamenetek (`.veriga`, JSON) felcserélhetők a három forma között.

## Mit ellenőriz

- **HTML-hivatkozások**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (stílus, ikonok, preload), `form[action]` (opcionális), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` és `@import`, `.css` fájlokból, valamint `<style>` elemekből és `style` attribútumokból.
- **Protokollok**: HTTP/HTTPS (HEAD, GET-re való visszalépéssel), `file://` és helyi útvonalak, FTP (hitelesítés, `SIZE`/`CWD`),
  `mailto:` (szintaxis).
- **Átirányítások**: teljes lánc állapotkódokkal, hurokészlelés, beállítható korlát, az „átirányítások hibák” opció.
- **`#fragment` horgonyok**: a bejárt oldalakban ellenőrzi (`id` és `name`).
- **Árva fájlok**: az oldal könyvtárában lévő fájlok, amelyekre egyetlen hivatkozás sem mutat.
- **Kis- és nagybetűs eltérések** a helyi útvonalakban (Windowson/macOS-en működik, Linuxon nem).
- **robots.txt** (csoportok, `Allow`/`Disallow` `*` és `$` jelekkel, `Crawl-delay`) és kiindulás a **sitemap.xml** fájlból.
- **Megfigyelések**: ismétlődő/hiányzó címek, lassú válaszok, statisztikák MIME-típus szerint.
- Karakterkódolások (fejléc, BOM, `<meta charset>`) — a régi Windows-1250 oldalakat is beleértve.

## Fő beállítások

Párhuzamos szálak és gazdagépenkénti korlát, időtúllépés, újrapróbálkozások (a `Retry-After` figyelembevételével), maximális mélység, URL-korlát,
külső hivatkozások igen/nem, „maradj a könyvtárban”, kizárás / ne kövesd / csak ezek (regex), további belső gazdagépek,
OK-nak tekintett HTTP-kódok (pl. 999), User-Agent, proxy (HTTP/SOCKS), Basic hitelesítés (csak belső gazdagépekhez),
fejlécek és sütik, érvénytelen tanúsítványok, késleltetés a kérések között.

## Jelentések

Önálló HTML (bárhol megnyitható), CSV, TSV, JSON, szöveg, sitemap.xml, oldalfa, egy `.veriga` munkamenet.
**A hibás hivatkozások újraellenőrzése** mentett munkamenetből (`--recheck`, gomb a GUI-ban/weben).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # helyi mappa + árva fájlok
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # csak a hibás hivatkozások
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

A kilépési kód `2`, ha hibás hivatkozásokat találtak (CI-ben hasznos); a `--no-fail` kikapcsolja. A `veriga --help` mindent felsorol.

## Fordítás

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # csak a CLI
cargo test --workspace
```

A `.github/workflows/build.yml` munkafolyamat archívumokat készít a következőkhöz: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, GUI-val) és Linux x64/ARM64 (musl, statikus CLI). A `Dockerfile` minimális, CLI-t tartalmazó image-et épít.

## Az egyfájlos HTML-kiadás

A `web/veriga.html` 100%-ban a böngészőben fut, szerver nélkül:

- **Helyi mappa** (húzd be az oldal mappáját, vagy „Mappa kiválasztása”): a belső hivatkozások, a CSS, a horgonyok, az árva fájlok,
  a kis- és nagybetűs eltérések és az ismétlődő címek teljes ellenőrzése. Itt nincs böngészőkorlát.
- **Online oldal**: a böngésző nem olvashatja más domainek HTTP-állapotát (CORS). Teljes eredményt kapsz, ha a fájlt
  ugyanarról az oldalról nyitod meg, vagy saját CORS-proxyn keresztül (sablon: `https://proxy/?u={url}`). Egyébként a külső hivatkozások
  *nem ellenőrizhető* jelölést kapnak, ha a szerver válaszol, illetve *nincs kapcsolat* jelölést, ha nem — soha nem hamis „OK”-t.
- **URL-lista / beillesztett HTML**.
- Nem ellenőriz FTP-t, és nem olvassa a robots.txt fájlt (böngészőkorlátozások). Ezekhez használd a CLI-t/GUI-t.

## Guardian — javítás, előzmények, megfelelőség

A VERIGA egy „link guardian” funkcióit is tartalmazza (a *Trade-Free Link Guardian* WordPress-bővítmény ihlette), de önálló eszközként, amely **fájlokon** dolgozik: HTML, Markdown, WordPress-export (WXR `.xml`, menükkel együtt) és webhelytérképek.

* **Determinisztikus javítások, élőben ellenőrizve** — `scheme_typo` (`htp://`, `https//`), `unmangle` (duplázott séma, `%20`, záró írásjelek), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (egységesség), `redirect` (csak 301/308). A javítást csak akkor alkalmazza automatikusan, **ha az új URL sikeresen válaszolt**; ellenőrzés nélkül „javaslat” marad.
* **Kézi javítások** — URL cseréje, „hivatkozás eltávolítása” (a szöveg megmarad), Wayback Machine-keresés (opt-in, soha nem hívódik automatikusan).
* **Biztonság** — futásonkénti biztonsági mentés, módosítási napló, egyéni vagy futásonkénti **visszavonás** (bájtra pontosan, az `&amp;` is); csak a tartományban lévő bájtok íródnak.
* **401/403 = „érdemes újra megnézni”**, nem hibás hivatkozás (opció, alapból bekapcsolva).
* **Nézetek és jelentések** — domainek, oldalankénti állapottérkép, „könyv” nézet, átirányítások (végleges/ideiglenes), tartalmi eltérés (cím/H1 az előző futáshoz képest), új/megoldott, trendek az előzményekből, időszaki **megfelelőségi jelentés**, hatásjelvény (a menüben vagy ≥ 3 oldalon). A CSV-exportok semlegesítik a képletcellákat.
* **404-esek a hozzáférési naplókból** (Apache/nginx), átirányítási javaslatokkal és `.htaccess` exporttal.
* **Ütemezés** — a `veriga wizard` (4 lépés) megírja a `veriga-guardian.json` fájlt; a `veriga watch --config … [--once]` rendszeresen fut (vagy cronból/Feladatütemezőből), vezeti az előzményeket, a kiválasztott szabályokat automatikusan javítja, és webhookon vagy `sendmail`-en értesít.

```
veriga lint ./site                       # offline elemzés
veriga fix ./site --session s.veriga     # javaslat + élő ellenőrzés (próbafuttatás)
veriga fix ./site --apply                # írás, biztonsági mentéssel és naplóval
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: a **🛡 Guardian** gomb. HTML-kiadás: a **Guardian** szakasz (a módosítások a memóriában történnek; letöltöd a fájlokat, vagy ott írod őket egy mappába, ahol a böngésző engedi; az élő ellenőrzést a CORS korlátozza).

**Ami nem replikálható egy WordPress-bővítményből:** az adatbázisban az `old slug` keresés, a szerkesztő mentési hookja, a WP-Cron, a multisite és a `wp_mail`. Helyettük: WXR-import, lemezen lévő fájlok, `watch`/cron és `sendmail`/webhook. A funkciólista a nyilvános leírásból és korábbi beszélgetésekből lett rekonstruálva, nem a bővítmény forráskódjából.

A projekt támogatása: https://www.buymeacoffee.com/echo.of.the.strings

## Ismert korlátok

- Nem küld jelentéseket e-mailben (SMTP), és nem észleli az árva fájlokat FTP-n — csak helyi könyvtárban.
- A tömörített `.xml.gz` webhelytérképeket nem olvassa.
- A JavaScripttel felépített hivatkozású oldalakat nem rendereli (nincs böngészőmotor); a kiszolgált HTML-t elemzi.
- A grafikus felület virtuális kijelzőn lefordult és elindult (indítási hibák nélkül), de vizuálisan nem lett kipróbálva.

## A „clean-room” jelentéséről

A kód az alapoktól íródott, a hivatkozásellenőrzők általános viselkedése és nyilvános specifikációk
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org) alapján. Nem tartalmaz kódot, erőforrást, szöveget vagy dizájnt semmilyen más termékből, és
nem egy másik program kódjának vagy futtatható fájljának nézegetése közben íródott.

Licenc: MIT.

## Biztonság és fenyegetésmodell

A VERIGA nem megbízható tartalmat dolgoz fel (weboldalak, exportált fájlok, konfigurációs fájlok). Alapértelmezett védelmek:

- **Anti-SSRF:** a privát/helyi célpontok (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) tiltottak, átirányítás után is. Kifejezett kikapcsolás: `--allow-private` (CLI) / a GUI jelölőnégyzete. *Korlát:* a DNS rebinding az ellenőrzés és a kapcsolódás között nincs teljesen lezárva.
- **Érzékeny fejlécek** (`Authorization`, `Cookie`, API-kulcsok) csak a belső origin felé kerülnek elküldésre, soha más gazdagépeknek vagy külső átirányítás után.
- **Injekciószűrő a javításokhoz:** az idézőjelet, `<`, `>` jelet, szóközt, visszaidézőjelet, visszaperjelet vagy Markdownban zárójelet tartalmazó új URL-t (pl. egy rosszindulatú `Location`) elutasítja (`unsafe_url_for` / `gUnsafeUrl`).
- **Biztonságos írás:** egyedi biztonsági mentés, atomi írás, kooperatív zárolás, a tartalom újraellenőrzése alkalmazás előtt (TOCTOU), jogosultságok/szimbolikus linkek megőrzése; az elavult javaslatok eldobásra kerülnek, nem alkalmazódnak rossz pozíciókon.
- **CSV-export** képletek ellen védve (`=`, `+`, `-`, `@`).
- **Értesítések:** a `sendmail` abszolút útvonal kell legyen; ne használj nem megbízható forrásból származó konfigurációs fájlokat (egy választott bináris futtatását válthatják ki).
- **Ismert korlátok:** a WXR-ben escape-elt HTML nincs átvizsgálva (hamis negatívok); a webes kiadást a CORS korlátozza; a `lint` 2-es kóddal lép ki, ha vannak találatai.

## Szabványok és megfelelőség (ellenőrizve: 2026.10.27.)

| Area | Status |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (mag + CLI; függőségeik legfeljebb 1.88-at deklarálnak), GUI **1.95** (egui 0.37); a `cargo clippy -- -D warnings` és a `cargo fmt --check` tiszta; `--locked` a CI-ben |
| Dependencies | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 sebezhetőség; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: összevont csoportok, pontos egyezés a termék-tokenre, százalék/UTF-8 normalizálás, `*` és `$`, a leghosszabb szabály nyer (egyenlőségnél Allow), 500 KiB korlát, 4xx ⇒ engedélyezett, 5xx/hálózati hiba ⇒ tiltott |
| HTTP | RFC 9110: `Retry-After` (másodperc/dátum) 429/503 esetén, 301/302/303/307/308 átirányítások, relatív `Location`, `HEAD` `GET`-re való visszalépéssel |
| URL | WHATWG URL / IDNA (UTS 46) az `url` crate-en keresztül (az `idna`-val) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 szabálysértés, világos/sötét téma), WAI-ARIA APG fülek billentyűzettel, `prefers-reduced-motion`, `forced-colors`, célméret ≥ 24 px, `status`/`progressbar` régiók; **CSP** hash-ekkel (szkriptekhez nincs `unsafe-inline`), `referrer: no-referrer`; az exportált jelentések saját CSP-t hordoznak |
| Supply chain | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, kiadások SHA-256 összegekkel és származási igazolással (Sigstore/SLSA), harmadik féltől származó akciók nélkül a publikálásban |

Webes tesztek: `cd web/tests && npm install && npx playwright install chromium && npm test`. A `veriga.html` bármilyen módosítása után: `python3 web/stamp-csp.py web/veriga.html`.
