# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/ro/1.1.0/) · versionare [SemVer](https://semver.org/lang/ro/).

## [Nepublicat / Unreleased]

## [Unreleased]

### Securitate
- Anti-SSRF: un punct de start local (`file://`) sau privat nu mai dezactivează protecția; se exceptează doar gazda privată dată explicit ca start.

### Reparat
- `veriga watch --once` iese cu cod ≠ 0 când rularea eșuează (systemd, CronJob, cron văd eșecul); un start gol este respins.
- Răspunsurile HTTP trunchiate sunt semnalate (mărime primită, nu cea declarată); erorile de conexiune apar în română.
- Linkurile din `<noscript>` sunt acum parcurse.
- Site: titlul și meta description urmează limba aleasă; insigna de teste actualizată; abaterea de la RFC 9309 (robots.txt cu 5xx) este menționată.
- GUI: `--help`/`--version`, mesaj curat fără afișaj, repaint mai rar în timpul scanării.
- Ambalare: serviciul `guardian` din compose pornește numai cu profil și cu config de exemplu; YunoHost (`ProtectHome=read-only`, binarul aparține root, instrucțiuni de instalare corecte); deb cu prerm/postrm și rpm cu scriptlet-uri; PWA reîncarcă shell-ul ocolind cache-ul HTTP și șterge doar propriile cache-uri; GHCR verifică eticheta față de Cargo.toml înainte de publicare; Termux cu autotest de rețea.

## [1.0.2] — 2026-10-29

Remedieri din recenzia adversarială a versiunii 1.0.0/1.0.1. Toate au teste de regresie (`crates/veriga-core/tests/regressions_102.rs`, `web/tests/core.test.js`).

### Fals succes / False success
- **robots.txt indisponibil (5xx, DNS, port închis) nu mai blochează verificarea.** Înainte, un site căzut ieșea „0 rupte”, cod 0 (verde în CI). Acum cererea reală raportează `DnsFailure` / `NoConnection` / `ServerError`, cod 2.
- **Pagina de start nu mai poate fi exclusă de robots.txt**: a fost cerută explicit.
- **Scheme greșite** (`htp:`, `htps:`, `hxxp:`, `mialto:`…) apar ca **URL invalid** (rupte), cu sugestia schemei corecte, în CLI, GUI și varianta web. Schemele non-web legitime (`tel:`, `sms:`, `skype:`, `geo:`…) se ignoră; orice altă schemă necunoscută apare ca „omisă”, nu dispare.
- **Intrare de start inexistentă**: `veriga ./sitee` sau `veriga check` (fără URL) dau eroare clară, nu `https://./sitee`.

### CLI
- Subcomanda **`veriga check URL`** (echivalentă cu `veriga URL`), folosită în Docker, docker-compose, YunoHost și documentație.
- Cod de ieșire **3 (incomplet) are prioritate față de 2** — o rulare tăiată nu se mai prezintă drept „doar linkuri rupte”.
- `--max-per-pattern N` (implicit 5000, 0 = dezactivat): protecție la capcane fără fund (paginări/calendare `/x/N`, segmente repetate `/a/a/a/a`). La atingerea limitei, linkurile se verifică dar nu se mai parcurg, iar rularea e marcată incompletă (cod 3).

### Guardian
- `fix` **refuză fișierele doar-citire** (read-only): sunt raportate „neatinse”, nu rescrise.
- Fișierele **non-UTF-8 (Windows-1250 / ISO-8859-2 / declarate prin `<meta charset>`)** sunt scanate de `lint`, reparate și re-scrise în **aceeași codificare**; decodarea trebuie să fie reversibilă, iar caracterele nereprezentabile duc la refuz (fișier neatins). `undo` folosește aceeași logică; o scriere eșuată la `undo` nu mai e raportată drept anulare reușită.

### GUI / Web
- GUI: filtrele „OK” și „Omise” afișează contorul.
- PWA (`/app/`): subtitlul descrie aplicația instalabilă, nu „single-file HTML”.

## [1.0.1] — 2026-10-29

Prima versiune cu pachetele `.deb`/`.rpm` și imaginea `ghcr.io/chiuta/veriga`; vezi „Împachetare” mai jos.

### Împachetare mobil și server / Mobile & server packaging
- PWA instalabilă offline (`app/`, generată de `web/build-pwa.py`, verificată în CI) și script Termux cu verificare SHA-256.
- Docker: imagine non-root (65532) cu volum `/data`, multi-arch (amd64+arm64) publicată pe GHCR la lansări; `docker-compose.yml`; CronJob Kubernetes.
- systemd: `veriga-watch.service` (oneshot, `DynamicUser`, consolidat) + `.timer`; pachete `.deb` și `.rpm` (amd64/arm64) construite în CI.
- Pachet **YunoHost** (format v2) în `packaging/yunohost/`: ediția HTML la un URL ales, CLI și Link Guardian opțional. Verificat static; încă nerulat prin `package_check` pe o instanță YunoHost.

## [1.0.0] — 2026-10-29

### Pagina web / Website
- Secțiune nouă „Descarcă” cu linkuri directe către toate fișierele release-ului v1.0.0 (Windows/macOS/Linux x64 și ARM64, CLI static musl, ediția portabilă, varianta HTML, SHA256SUMS), traduse în 8 limbi; JSON-LD cu `softwareVersion`.

### Remedieri din revizuirea independentă pre-lansare / Independent pre-release review fixes
- `undo` nu mai caută textul nou „la nimereală”: după o editare manuală a fișierului raportează conflict în loc să atingă un alt link (regresie testată).
- `fix --unlink` tratează corect `>` în valorile de atribute (`title="a > b"`).
- Atributul `lang` din rapoarte/pagini Guardian provine dintr-o listă închisă (`ro`/`en`): o sesiune `.veriga` modificată nu mai poate injecta HTML; link-urile „părinte” din raport sunt active doar pentru http/https/file/ftp.
- Credențialele din URL (`user:parolă@`) sunt scoase din toate rapoartele (HTML/CSV/TSV/TXT/JSON/sitemap/arbore), din sesiuni, din URL-urile de start și din mesajele de eroare.
- `log404 -o .htaccess` scrie reguli Apache (nu CSV).
- Ancorele cu directivă de text (`#:~:text=`) și rutele SPA (`#!/…`, `#/…`) nu mai sunt raportate drept „ancoră lipsă”.
- CI: eticheta de release trebuie să corespundă versiunii din `Cargo.toml`; rularea de release nu mai este anulată de un push ulterior.
- Limite cunoscute documentate: fereastra DNS (rebinding) între verificare și conectare; `veriga watch` citește `veriga-guardian.json` din directorul curent (nu-l rula în directoare necunoscute).

### Tutoriale video / Video tutorials
- 10 tutoriale video (GUI în RO și EN, CLI, varianta HTML, ediția portabilă), subtitrate în 8 limbi: `.mp4` cu 8 piste de subtitrare + `.vtt`, în `assets/tutorials/`; secțiune nouă pe site (subtitrarea urmează limba paginii; CSP cu `media-src 'self'`).

### Remedieri după revizuirea de securitate / Security review fixes
- FTP: URL-urile cu CR/LF/NUL codificate (`%0D%0A`) în cale/utilizator/parolă sunt refuzate (injecție de comenzi); FTP respectă acum protecția anti-SSRF; răspunsurile FTP au lungime plafonată.
- O pagină la distanță (http/https) care trimite spre `file://` nu mai poate face programul să citească fișiere locale: linkul este raportat ca „Omis”.
- Sitemap-urile trec prin protecția anti-SSRF și sunt citite cu plafonul `max_body_bytes`.
- Fișierele locale se citesc doar dacă sunt fișiere obișnuite, cel mult `max_body_bytes`.
- Antete sensibile: orice antet al cărui nume conține auth/token/key/secret/cookie/password/session (plus Proxy-Authorization); antetele și autentificarea Basic pleacă doar spre aceeași schemă și gazdă (fără coborâre https → http).
- Sesiunile `.veriga` și raportul JSON nu mai salvează antete sensibile, autentificare sau `user:parolă` din proxy.
- Deschiderea unei sesiuni (GUI) și `--recheck` (CLI) nu mai preiau din fișier proxy, TLS nesigur, anti-SSRF, User-Agent sau antete: se folosesc valorile curente.
- Anti-SSRF: blocate și `::a.b.c.d` (IPv4-compatibil), NAT64 `64:ff9b::/96`, 6to4 `2002::/16` și `::ffff:0:a.b.c.d` cu IPv4 privat încorporat, `fec0::/10`.
- GUI (Windows): deschiderea linkurilor nu mai trece prin `cmd /C start` (injectabil); se deschid doar URL-uri http/https/file.
- `undo` nu mai golește un fișier care nu poate fi citit (ex. salvat între timp în CP1250); se rescriu doar fișierele în care s-a anulat efectiv ceva.
- Copiile de siguranță nu se mai suprascriu când două aplicări au același ID de rulare; ID-urile de rulare sunt unice (milisecunde + contor).
- Panici eliminate: URL-uri/texte cu diacritice în `fix`, `unlink` Markdown, scanarea surselor, extragerea CSS; `Crawl-delay` NaN/infinit/negativ ignorat; `--retries` plafonat la 10.
- `log404`: protecție CPU (căi > 512 caractere ignorate, prefiltru pe lungime, maximum 20 000 de căi distincte).
- GUI Guardian: statisticile (domenii, hartă, redirecționări, istoric) sunt calculate o singură dată, nu la fiecare cadru; propunerile manuale se calculează pe un fir separat.
### CLI
- Cod de ieșire **3** când verificarea a fost oprită sau trunchiată (`--max-urls`), chiar fără linkuri rupte (și chiar cu `--no-fail`). / Exit code 3 when the run was cancelled or truncated.
- `veriga --help` listează subcomenzile; un director numit ca o subcomandă se verifică prin `veriga ./lint` sau `veriga -- lint`.
- `veriga lint` fără căi dă eroare (ca `fix`).
### Limitări cunoscute / Known limitations
- Mesajele de stare produse de motor (coloana „Mesaj”) sunt încă doar în română; textul de stare din GUI nu se retraduce la schimbarea limbii.

### Securitate
- Anti-SSRF implicit (`--allow-private`), antete sensibile limitate la originea internă, filtru de injecție pentru URL-uri inserate, CSV fără injecție de formule, CSP cu hash în varianta web și în rapoartele HTML.
### Portabil (PortableApps.com Format 3.9)
- Pachet `VERIGAPortable_<versiune>_Multilingual.zip` în release: lansator Rust (`crates/veriga-launcher`) care păstrează totul în `Data\`, alege x64/ARM64 după arhitectura nativă și nu atinge registrul; `appinfo.ini`, pictograme (ico + png), `help.html` în 8 limbi, `installer.ini`.
- GUI: primul argument din linia de comandă deschide o sesiune `.veriga` (asociere de fișiere).
- CI: jobul `portable` asamblează și validează pachetul (`portableapps/validate.py`).
### Remedieri GUI
- Pictogramele din butoane apăreau ca pătrățele după portarea la egui 0.37 (fonturile de emoji sunt acum activate, iar săgețile → ↶ ↳ au font de rezervă).
- Clicul pe un rând din tabel nu selecta rândul (etichetele selectabile interceptau clicul): selecția și panoul de detalii funcționează din nou.
### Pagina de prezentare
- Capturi de ecran ale aplicației native (RO/EN), bară de meniu compactă și lipicioasă pe telefon, 0 încălcări axe-core.
### Standarde
- Rust edition 2024, MSRV 1.88 (GUI 1.95); `clippy -D warnings` curat.
- robots.txt conform RFC 9309 (grupuri îmbinate, normalizare procentuală, plafon 500 KiB, 5xx ⇒ interdicție totală).
- Dependențe actualizate: reqwest 0.13, scraper 0.29, thiserror 2, egui/eframe 0.37, rfd 0.17.
- Web: WCAG 2.2 AA (axe-core: 0 încălcări), tab-uri WAI-ARIA cu tastatură, `prefers-reduced-motion`, `forced-colors`.
- Lanț de aprovizionare: `cargo-audit` + `cargo-deny` (`deny.toml`), `LICENSE` (MIT), `SECURITY.md`, Dependabot.
