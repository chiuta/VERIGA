# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/ro/1.1.0/) · versionare [SemVer](https://semver.org/lang/ro/).

## [Nepublicat / Unreleased]

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
