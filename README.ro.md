# VERIGA — verificator de legături (clean-room)

[English](README.md) · **Română** · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Descărcare:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.0.0

Versiune: 1.0.0 · Ultima actualizare: 27.10.2026 · Pagina de prezentare: https://veriga.chiuta.ro

Aplicație 64-bit în Rust, pentru Windows, macOS și Linux (x86-64 și ARM64), care parcurge un site
sau un director local și verifică legăturile. Vine în trei forme cu același model de rezultate:

| Formă | Unde | Ce face |
|---|---|---|
| `veriga` (CLI) | terminal, CI, containere (binar static musl) | verificare completă, rapoarte, cod de ieșire |
| `veriga-gui` | desktop nativ (egui) | tabel sortabil/filtrabil, panou de detalii, pauză/stop, export |
| `web/veriga.html` | orice browser, un singur fișier, offline | analiză de folder local, site online (cu limitele CORS), listă de URL-uri |

Sesiunile (`.veriga`, JSON) sunt interschimbabile între cele trei forme.

## Ce verifică

- **Linkuri HTML**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (stil, pictograme, preload), `form[action]` (opțional), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` și `@import`, atât din fișiere `.css`, cât și din `<style>` și atribute `style`.
- **Protocoale**: HTTP/HTTPS (HEAD cu revenire la GET), `file://` și căi locale, FTP (autentificare, `SIZE`/`CWD`),
  `mailto:` (sintaxă).
- **Redirecționări**: lanț complet cu coduri, detecție de bucle, limită configurabilă, opțiune „redirecționările sunt erori”.
- **Ancore** `#fragment`: se verifică în paginile parcurse (`id` și `name`).
- **Fișiere orfane**: fișiere din directorul site-ului la care nu duce niciun link.
- **Diferențe de majuscule** în căi locale (merge pe Windows/macOS, se strică pe Linux).
- **robots.txt** (grupuri, `Allow`/`Disallow` cu `*` și `$`, `Crawl-delay`) și pornire din **sitemap.xml**.
- **Observații**: titluri duplicate/lipsă, răspunsuri lente, statistici pe tip MIME.
- Codificări de caractere (antet, BOM, `<meta charset>`) — inclusiv paginile vechi în Windows-1250.

## Opțiuni principale

Fire simultane și limită pe gazdă, timeout, reîncercări (cu `Retry-After`), adâncime maximă, limită de URL-uri,
linkuri externe da/nu, „rămâi în director”, exclude / nu-parcurge / doar-acestea (regex), gazde interne suplimentare,
coduri HTTP tratate ca OK (ex. 999), User-Agent, proxy (HTTP/SOCKS), autentificare Basic (doar către gazdele interne),
antete și cookie-uri, certificate invalide, pauză între cereri.

## Rapoarte

HTML autonom (se deschide oriunde), CSV, TSV, JSON, text, sitemap.xml, arbore al site-ului, sesiune `.veriga`.
**Reverificare a linkurilor rupte** dintr-o sesiune salvată (`--recheck`, buton în GUI/web).

## CLI

```bash
veriga https://exemplu.ro -o raport.html -o rupte.csv --only-broken
veriga ./site --orphans ./site --fragments -o raport.html          # director local + orfani
veriga https://exemplu.ro --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck sesiune.veriga -o raport.html                      # doar linkurile rupte
veriga https://exemplu.ro --ignore-status 999 --exclude '/logout' -o s.veriga
```

Cod de ieșire `2` dacă s-au găsit legături rupte (util în CI); `--no-fail` îl dezactivează. `veriga --help` listează tot. Cod de ieșire `3` dacă rularea a fost oprită sau tăiată de `--max-urls`.

## Compilare

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # doar CLI
cargo test --workspace
```

Fluxul `.github/workflows/build.yml` produce arhive pentru: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, cu GUI) și Linux x64/ARM64 (musl, CLI static). `Dockerfile` construiește o imagine minimă cu CLI-ul.

## Tutoriale video

Zece tutoriale scurte, înregistrate pe aplicația reală și subtitrate în 8 limbi (ro, en, fr, it, es, pt, de, hu): prima verificare, filtre, detaliile unui link, opțiuni, export, Guardian (reparații; domenii, hartă, tendințe), linia de comandă, varianta HTML și ediția portabilă. Le găsești pe https://veriga.chiuta.ro/#tutoriale sau în `assets/tutorials/`: fiecare `.mp4` conține toate cele 8 piste de subtitrare, iar fișierele `.vtt` sunt alăturate. Fără sunet; explicațiile sunt în subtitrări.

## Varianta portabilă (PortableApps.com)

VERIGA vine și ca pachet **PortableApps.com Format 3.9**, `VERIGAPortable_<versiune>_Multilingual.zip` (fișiere ale release-ului). Dezarhivează-l oriunde (stick USB, folder sincronizat) și pornește `VERIGAPortable.exe`.

- Tot ce produce aplicația rămâne în `Data\` (starea Guardian în `Data\.veriga`); nu se scrie nimic în registru sau în `%APPDATA%`. Ștergerea `Data\` este sigură: se recreează.
- Lansatorul alege versiunea x64 sau ARM64 după arhitectura nativă; `VERIGAPortable.exe sesiune.veriga` deschide o sesiune salvată (asociere de fișiere `.veriga`).
- CLI-ul este `App\VERIGA\veriga.exe`. Structura este verificată în CI cu `portableapps/validate.py`; sursa lansatorului este `crates/veriga-launcher`.
- Instalatorul oficial `.paf.exe` se construiește cu PortableApps.com Installer (unealtă pentru Windows); folderul are deja structura cerută.

## Varianta HTML single-file

`web/veriga.html` rulează 100% în browser, fără server:

- **Folder local** (trage folderul site-ului sau „Alege folder”): verificare completă a legăturilor interne, CSS, ancore, orfani,
  diferențe de majuscule, titluri duplicate. Aici nu există nicio limitare de browser.
- **Site online**: un browser nu poate citi codul HTTP al altor domenii (CORS). Rezultate complete când fișierul e deschis de pe
  același site, sau printr-un proxy CORS propriu (șablon `https://proxy/?u={url}`). Altfel, linkurile externe sunt marcate
  *neverificabile* dacă serverul răspunde, respectiv *fără conexiune* dacă nu răspunde — niciodată „OK” fals.
- **Listă de URL-uri / HTML lipit**.
- Nu verifică FTP și nu citește robots.txt (restricții ale browserului). Pentru acestea folosește CLI/GUI.

## Guardian — reparare, istoric, conformitate

VERIGA include și funcțiile unui „păzitor de linkuri” (inspirate de pluginul WordPress *Trade-Free Link Guardian*), dar ca unealtă independentă, care lucrează pe **fișiere**: HTML, Markdown, export WordPress (WXR `.xml`, inclusiv meniurile) și sitemap.

* **Reparații deterministe, verificate live** — `scheme_typo` (`htp://`, `https//`), `unmangle` (schemă dublă, `%20`, punctuație agățată), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (consecvență), `redirect` (doar 301/308). O reparație se aplică automat **numai dacă URL-ul nou a răspuns cu succes**; fără verificare rămâne „propunere”.
* **Reparații manuale** — înlocuire URL, „dezleagă” (păstrează textul), căutare Wayback Machine (opt-in, nu se apelează automat).
* **Siguranță** — copie de siguranță per rulare, jurnal de modificări, **anulare** individuală sau pe rulare (octet cu octet, inclusiv `&amp;`); se scriu doar octeții din interval.
* **401/403 = „merită o a doua privire”**, nu link rupt (opțiune, implicit activă).
* **Vederi și rapoarte** — domenii, hartă de sănătate pe pagini, vedere „carte”, redirecționări (permanente/temporare), derivă de conținut (titlu/H1 față de rularea anterioară), noi/rezolvate, tendințe din istoric, **raport de conformitate** pe perioadă, insignă de impact (în meniu sau pe ≥ 3 pagini). Exporturile CSV neutralizează celulele-formulă.
* **404 din jurnale de acces** Apache/nginx, cu sugestii de redirecționare și export `.htaccess`.
* **Programare** — `veriga wizard` (4 pași) scrie `veriga-guardian.json`; `veriga watch --config … [--once]` rulează periodic (sau din cron/Task Scheduler), ține istoricul, repară automat regulile alese și notifică prin webhook sau `sendmail`.

```
veriga lint ./site                       # analiză offline
veriga fix ./site --session s.veriga     # propune + verifică live (simulare)
veriga fix ./site --apply                # scrie, cu copie de siguranță și jurnal
veriga fix ./site --unlink https://mort.ro/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev veche.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o conformitate.html
```

GUI: butonul **🛡 Guardian**. Varianta HTML: secțiunea **Guardian** (modificările se fac în memorie; descarci fișierele sau le scrii într-un folder acolo unde browserul permite; verificarea live e limitată de CORS).

**Ce nu se poate replica dintr-un plugin WordPress:** căutarea de `old slug` din baza de date, cârligul din editor la salvare, cron-ul WP, multisite și `wp_mail`. În locul lor: import WXR, fișiere de pe disc, `watch`/cron și `sendmail`/webhook. Lista de funcții a fost reconstituită din descrierea publică și din conversațiile anterioare, nu din codul sursă al pluginului.

Susține proiectul: https://www.buymeacoffee.com/echo.of.the.strings

## Limite cunoscute

- **Protecția de rețea:** verificarea adreselor private rezolvă gazda, apoi clientul HTTP o rezolvă din nou (există o fereastră de DNS rebinding); rulează VERIGA pe site-uri nesigure dintr-o rețea care nu ajunge la intervalele private.
- **`veriga watch`** citește implicit `./veriga-guardian.json` din directorul curent, iar acel fișier poate indica o comandă `sendmail` și sursele de rescris: nu-l rula într-un director în care nu ai încredere.
- Nu trimite rapoarte prin e-mail (SMTP) și nu detectează orfani prin FTP — doar prin director local.
- Sitemap-urile comprimate `.xml.gz` nu sunt citite.
- Paginile care își construiesc linkurile din JavaScript nu sunt randate (nu există motor de browser); se analizează HTML-ul servit.
- Interfața grafică a fost exersată doar sub un ecran virtual (Linux/X11): parcurgere, filtre, panou de detalii și Guardian. Variantele pentru Windows și macOS sunt produse de CI, dar nu au fost testate interactiv.

## Despre „clean-room”

Codul este scris de la zero, pe baza comportamentului general al verificatoarelor de legături și a specificațiilor publice
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Nu conține cod, resurse, texte sau designul vreunui alt produs și
nu a fost scris privind codul sau executabilul altui program.

Licență: MIT.

## Securitate și model de amenințare / Security & threat model

VERIGA procesează conținut necontrolat (pagini web, fișiere exportate, fișiere de configurare). Măsuri implicite:

- **Anti-SSRF:** țintele private/locale (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) sunt blocate, inclusiv după redirect. Dezactivare explicită: `--allow-private` (CLI) / bifa din GUI. *Limită:* rebinding-ul DNS între verificare și conectare nu este închis complet.
- **Anteturi sensibile** (`Authorization`, `Cookie`, chei API) se trimit doar către originea internă, niciodată către alte gazde sau după redirect extern.
- **Filtru de injecție la remedieri:** un URL nou (ex. `Location` ostil) cu ghilimele, `<`, `>`, spații, backtick, backslash sau paranteze în Markdown este refuzat (`unsafe_url_for` / `gUnsafeUrl`).
- **Scriere sigură:** backup unic, scriere atomică, blocare cooperativă, re-verificarea conținutului înainte de aplicare (TOCTOU), păstrarea permisiunilor/symlink-urilor; propunerile vechi sunt eliminate, nu aplicate pe poziții greșite.
- **Export CSV** protejat împotriva formulelor (`=`, `+`, `-`, `@`).
- **Notificări:** `sendmail` trebuie să fie cale absolută; nu folosiți fișiere de configurare din surse nesigure (pot declanșa execuția unui binar ales).
- **Limite cunoscute:** HTML escapat în WXR nu este scanat (fals-negative); varianta web este limitată de CORS; `lint` iese cu cod 2 când are constatări.

## Standarde și conformitate / Standards & compliance (verificat 27.10.2026)

| Domeniu | Stare |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (core + CLI; dependențele lor declară cel mult 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` și `cargo fmt --check` curate; `--locked` în CI |
| Dependențe | reqwest 0.13 (rustls + verificator de platformă, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 vulnerabilități; `cargo deny` (advisories, licențe, surse, ban-uri) |
| robots.txt | **RFC 9309**: grupuri îmbinate, potrivire exactă pe tokenul de produs, normalizare procentuală/UTF-8, `*` și `$`, cea mai lungă regulă câștigă (Allow la egalitate), plafon 500 KiB, 4xx ⇒ permis, 5xx/rețea ⇒ interzis |
| HTTP | RFC 9110: `Retry-After` (secunde/dată) la 429/503, redirecturi 301/302/303/307/308, `Location` relativ, `HEAD` cu revenire la `GET` |
| URL | WHATWG URL / IDNA (UTS 46) prin crate-ul `url` (cu `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 încălcări, temă clară/întunecată), tab-uri WAI-ARIA APG cu tastatură, `prefers-reduced-motion`, `forced-colors`, ținte ≥ 24 px, regiuni `status`/`progressbar`; **CSP** cu hash-uri (fără `unsafe-inline` pentru scripturi), `referrer: no-referrer`; rapoartele exportate au CSP propriu |
| Lanț de aprovizionare | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, release cu sume SHA-256 și atestare de proveniență (Sigstore/SLSA), fără acțiuni terțe în publicare |

Teste web: `cd web/tests && npm install && npx playwright install chromium && npm test`. După orice modificare a `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
