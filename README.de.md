# VERIGA — Link-Checker (Clean-Room)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

Version: 1.0.0 · Zuletzt aktualisiert: 27.10.2026 · Präsentationsseite: https://veriga.chiuta.ro

Eine 64-Bit-Rust-Anwendung für Windows, macOS und Linux (x86-64 und ARM64), die eine Website
oder einen lokalen Ordner durchsucht und deren Links prüft. Sie gibt es in drei Formen, die ein gemeinsames Ergebnismodell teilen:

| Form | Wo | Was sie tut |
|---|---|---|
| `veriga` (CLI) | Terminal, CI, Container (statische musl-Binärdatei) | vollständige Prüfung, Berichte, Exit-Code |
| `veriga-gui` | native Desktop-Anwendung (egui) | sortier-/filterbare Tabelle, Detailbereich, Pause/Stopp, Export |
| `web/veriga.html` | jeder Browser, einzelne Datei, offline | Analyse lokaler Ordner, Online-Site (im Rahmen von CORS), URL-Liste |

Sitzungen (`.veriga`, JSON) sind zwischen den drei Formen austauschbar.

## Was geprüft wird

- **HTML-Links**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (Stile, Icons, Preload), `form[action]` (optional), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` und `@import`, sowohl aus `.css`-Dateien als auch aus `<style>` und `style`-Attributen.
- **Protokolle**: HTTP/HTTPS (HEAD mit Rückfall auf GET), `file://` und lokale Pfade, FTP (Authentifizierung, `SIZE`/`CWD`),
  `mailto:` (Syntax).
- **Weiterleitungen**: vollständige Kette mit Statuscodes, Schleifenerkennung, konfigurierbares Limit, Option „Weiterleitungen sind Fehler“.
- **`#fragment`-Anker**: werden in den durchsuchten Seiten geprüft (`id` und `name`).
- **Verwaiste Dateien**: Dateien im Site-Verzeichnis, auf die kein Link verweist.
- **Groß-/Kleinschreibungsabweichungen** in lokalen Pfaden (funktioniert unter Windows/macOS, bricht unter Linux).
- **robots.txt** (Gruppen, `Allow`/`Disallow` mit `*` und `$`, `Crawl-delay`) und Start aus der **sitemap.xml**.
- **Beobachtungen**: doppelte/fehlende Titel, langsame Antworten, Statistiken nach MIME-Typ.
- Zeichenkodierungen (Header, BOM, `<meta charset>`) — einschließlich alter Windows-1250-Seiten.

## Wichtige Optionen

Parallele Threads und ein Limit pro Host, Timeout, Wiederholungsversuche (unter Beachtung von `Retry-After`), maximale Tiefe, URL-Limit,
externe Links ja/nein, „im Verzeichnis bleiben“, ausschließen / nicht folgen / nur diese (Regex), zusätzliche interne Hosts,
als OK gewertete HTTP-Codes (z. B. 999), User-Agent, Proxy (HTTP/SOCKS), Basic-Authentifizierung (nur interne Hosts),
Header und Cookies, ungültige Zertifikate, Verzögerung zwischen Anfragen.

## Berichte

Eigenständiges HTML (öffnet sich überall), CSV, TSV, JSON, Text, sitemap.xml, ein Site-Baum, eine `.veriga`-Sitzung.
**Erneute Prüfung defekter Links** aus einer gespeicherten Sitzung (`--recheck`, eine Schaltfläche in GUI/Web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # lokaler Ordner + verwaiste Dateien
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # nur defekte Links
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Exit-Code `2`, wenn defekte Links gefunden wurden (nützlich in CI); `--no-fail` schaltet das ab. `veriga --help` listet alles auf. Exit-Code `3`, wenn der Lauf abgebrochen oder durch `--max-urls` abgeschnitten wurde.

## Erstellen

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # nur CLI
cargo test --workspace
```

Der Workflow `.github/workflows/build.yml` erzeugt Archive für: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, mit GUI) und Linux x64/ARM64 (musl, statische CLI). Das `Dockerfile` baut ein minimales Image mit der CLI.

## Video-Tutorials

Zehn kurze Tutorials, aufgenommen mit der echten Anwendung und in 8 Sprachen untertitelt (ro, en, fr, it, es, pt, de, hu): erster Scan, Filter, Linkdetails, Optionen, Export, Guardian (Korrekturen; Domains, Karte, Trends), Kommandozeile, HTML-Edition und portable Edition. Zu sehen auf https://veriga.chiuta.ro/#tutoriale oder in `assets/tutorials/`: Jede `.mp4` enthält alle 8 Untertitelspuren, die `.vtt`-Dateien liegen daneben. Ohne Ton; die Erklärungen stehen in den Untertiteln.

## Portable Ausgabe (PortableApps.com)

VERIGA gibt es auch als **PortableApps.com-Format-3.9**-Paket, `VERIGAPortable_<Version>_Multilingual.zip` (Release-Dateien). Entpacke es beliebig (USB-Stick, synchronisierter Ordner) und starte `VERIGAPortable.exe`.

- Alles, was die App erzeugt, bleibt in `Data\` (Guardian-Zustand in `Data\.veriga`); in die Registrierung oder nach `%APPDATA%` wird nichts geschrieben. `Data\` zu löschen ist unbedenklich: Der Ordner wird neu angelegt.
- Der Launcher wählt die x64- oder ARM64-Version nach der nativen Architektur; `VERIGAPortable.exe sitzung.veriga` öffnet eine gespeicherte Sitzung (Dateizuordnung `.veriga`).
- Die CLI ist `App\VERIGA\veriga.exe`. Die Struktur wird in der CI mit `portableapps/validate.py` geprüft; der Launcher-Quellcode liegt in `crates/veriga-launcher`.
- Der offizielle `.paf.exe`-Installer wird mit dem PortableApps.com Installer (Windows-Werkzeug) gebaut; der Ordner hat bereits die geforderte Struktur.

## Die Single-File-HTML-Edition

`web/veriga.html` läuft zu 100 % im Browser, ohne Server:

- **Lokaler Ordner** (den Site-Ordner hineinziehen oder „Ordner wählen“): vollständige Prüfung interner Links, CSS, Anker, verwaister Dateien,
  Groß-/Kleinschreibungsabweichungen, doppelter Titel. Hier gibt es keine Browser-Einschränkung.
- **Online-Site**: Ein Browser kann den HTTP-Status anderer Domains nicht lesen (CORS). Vollständige Ergebnisse, wenn die Datei von derselben
  Site aus geöffnet wird oder über einen eigenen CORS-Proxy (Vorlage `https://proxy/?u={url}`). Andernfalls werden externe Links als
  *nicht überprüfbar* markiert, wenn der Server antwortet, oder als *keine Verbindung*, wenn nicht — niemals ein falsches „OK“.
- **URL-Liste / eingefügtes HTML**.
- FTP wird nicht geprüft und robots.txt nicht gelesen (Browser-Einschränkungen). Dafür CLI/GUI verwenden.

## Guardian — Reparatur, Verlauf, Compliance

VERIGA enthält außerdem die Funktionen eines „Link-Guardians“ (inspiriert vom WordPress-Plugin *Trade-Free Link Guardian*), aber als unabhängiges Werkzeug, das auf **Dateien** arbeitet: HTML, Markdown, WordPress-Export (WXR `.xml`, Menüs eingeschlossen) und Sitemaps.

* **Deterministische Reparaturen, live verifiziert** — `scheme_typo` (`htp://`, `https//`), `unmangle` (doppeltes Schema, `%20`, abschließende Satzzeichen), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (Konsistenz), `redirect` (nur 301/308). Eine Reparatur wird **nur dann** automatisch angewendet, wenn die neue URL erfolgreich geantwortet hat; ohne Verifizierung bleibt sie ein „Vorschlag“.
* **Manuelle Reparaturen** — URL-Ersetzung, „Verlinkung entfernen“ (der Text bleibt), Wayback-Machine-Abfrage (Opt-in, nie automatisch aufgerufen).
* **Sicherheit** — eine Sicherung pro Lauf, ein Änderungsprotokoll, einzelnes oder lauf-weises **Rückgängigmachen** (Byte für Byte, `&amp;` eingeschlossen); es werden nur die Bytes im Bereich geschrieben.
* **401/403 = „einen zweiten Blick wert“**, kein defekter Link (eine Option, standardmäßig aktiv).
* **Ansichten und Berichte** — Domains, Gesundheitskarte pro Seite, „Buch“-Ansicht, Weiterleitungen (dauerhaft/temporär), Inhaltsdrift (Titel/H1 gegenüber dem vorherigen Lauf), neu/behoben, Trends aus dem Verlauf, ein **Compliance-Bericht** für einen Zeitraum, ein Auswirkungs-Badge (im Menü oder auf ≥ 3 Seiten). CSV-Exporte neutralisieren Formelzellen.
* **404-Fehler aus Zugriffsprotokollen** (Apache/nginx), mit Weiterleitungsvorschlägen und `.htaccess`-Export.
* **Planung** — `veriga wizard` (4 Schritte) schreibt `veriga-guardian.json`; `veriga watch --config … [--once]` läuft periodisch (oder per cron/Aufgabenplanung), führt den Verlauf, repariert die gewählten Regeln automatisch und benachrichtigt per Webhook oder `sendmail`.

```
veriga lint ./site                       # Offline-Analyse
veriga fix ./site --session s.veriga     # vorschlagen + live verifizieren (Probelauf)
veriga fix ./site --apply                # schreiben, mit Sicherung und Protokoll
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: die Schaltfläche **🛡 Guardian**. HTML-Edition: der Abschnitt **Guardian** (Änderungen erfolgen im Arbeitsspeicher; du lädst die Dateien herunter oder schreibst sie in einen Ordner, wo der Browser dies erlaubt; die Live-Verifizierung ist durch CORS eingeschränkt).

**Was sich von einem WordPress-Plugin nicht nachbilden lässt:** die Datenbanksuche nach dem „alten Slug“, der Speicher-Hook des Editors, WP-Cron, Multisite und `wp_mail`. Stattdessen gibt es: WXR-Import, Dateien auf der Festplatte, `watch`/cron und `sendmail`/Webhook. Die Funktionsliste wurde aus der öffentlichen Beschreibung und aus früheren Gesprächen rekonstruiert, nicht aus dem Quellcode des Plugins.

Unterstütze das Projekt: https://www.buymeacoffee.com/echo.of.the.strings

## Bekannte Grenzen

- Berichte werden nicht per E-Mail (SMTP) versendet, und verwaiste Dateien werden nicht über FTP erkannt — nur in einem lokalen Verzeichnis.
- Komprimierte `.xml.gz`-Sitemaps werden nicht gelesen.
- Seiten, die ihre Links mit JavaScript aufbauen, werden nicht gerendert (es gibt keine Browser-Engine); analysiert wird das ausgelieferte HTML.
- Die grafische Oberfläche wurde nur unter einem virtuellen Display (Linux/X11) erprobt: Crawl, Filter, Detailbereich und Guardian. Die Windows- und macOS-Builds entstehen in der CI, wurden aber nicht interaktiv getestet.

## Zum Begriff „Clean-Room“

Der Code ist von Grund auf neu geschrieben, auf Grundlage des allgemeinen Verhaltens von Link-Checkern und öffentlicher Spezifikationen
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Er enthält keinen Code, keine Ressourcen, Texte oder Designs eines anderen Produkts und
wurde nicht geschrieben, während der Code oder die ausführbare Datei eines anderen Programms eingesehen wurde.

Lizenz: MIT.

## Sicherheit und Bedrohungsmodell

VERIGA verarbeitet nicht vertrauenswürdige Inhalte (Webseiten, exportierte Dateien, Konfigurationsdateien). Standardmaßnahmen:

- **Anti-SSRF:** private/lokale Ziele (127.0.0.0/8, RFC1918, Link-Local, `localhost`, `.local`, IPv6 ULA) werden blockiert, auch nach einer Weiterleitung. Ausdrückliches Abschalten: `--allow-private` (CLI) / das Kontrollkästchen in der GUI. *Grenze:* DNS-Rebinding zwischen Prüfung und Verbindung ist nicht vollständig geschlossen.
- **Sensible Header** (`Authorization`, `Cookie`, API-Schlüssel) werden nur an den internen Origin gesendet, nie an andere Hosts oder nach einer externen Weiterleitung.
- **Injektionsfilter für Reparaturen:** Eine neue URL (z. B. ein feindliches `Location`), die Anführungszeichen, `<`, `>`, Leerzeichen, Backticks, Backslashes oder Klammern in Markdown enthält, wird abgelehnt (`unsafe_url_for` / `gUnsafeUrl`).
- **Sicheres Schreiben:** eindeutige Sicherung, atomares Schreiben, kooperative Sperren, erneute Prüfung des Inhalts vor dem Anwenden (TOCTOU), Beibehaltung von Berechtigungen/Symlinks; veraltete Vorschläge werden verworfen, nicht an falschen Positionen angewendet.
- **CSV-Export** gegen Formeln geschützt (`=`, `+`, `-`, `@`).
- **Benachrichtigungen:** `sendmail` muss ein absoluter Pfad sein; keine Konfigurationsdateien aus nicht vertrauenswürdigen Quellen verwenden (sie könnten die Ausführung einer gewählten Binärdatei auslösen).
- **Bekannte Grenzen:** maskiertes HTML in WXR wird nicht durchsucht (falsch negative Ergebnisse); die Web-Edition ist durch CORS eingeschränkt; `lint` endet mit Exit-Code 2, wenn es Befunde gibt.

## Standards und Compliance (geprüft am 27.10.2026)

| Bereich | Status |
|---|---|
| Rust | Edition **2024**, MSRV **1.88** (Kern + CLI; deren Abhängigkeiten deklarieren höchstens 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` und `cargo fmt --check` sauber; `--locked` in CI |
| Abhängigkeiten | reqwest 0.13 (rustls + Plattform-Verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 Schwachstellen; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: zusammengeführte Gruppen, exakter Abgleich mit dem Produkt-Token, Prozent-/UTF-8-Normalisierung, `*` und `$`, längste Regel gewinnt (Allow bei Gleichstand), 500-KiB-Grenze, 4xx ⇒ erlaubt, 5xx/Netzwerk ⇒ nicht erlaubt |
| HTTP | RFC 9110: `Retry-After` (Sekunden/Datum) bei 429/503, Weiterleitungen 301/302/303/307/308, relatives `Location`, `HEAD` mit Rückfall auf `GET` |
| URL | WHATWG URL / IDNA (UTS 46) über die `url`-Crate (mit `idna`) |
| Sitemap | sitemaps.org 0.9 (Index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 Verstöße, helles/dunkles Design), WAI-ARIA-APG-Tabs mit Tastatur, `prefers-reduced-motion`, `forced-colors`, Ziele ≥ 24 px, `status`-/`progressbar`-Regionen; **CSP** mit Hashes (kein `unsafe-inline` für Skripte), `referrer: no-referrer`; exportierte Berichte tragen ihre eigene CSP |
| Lieferkette | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, Releases mit SHA-256-Summen und einer Provenienz-Attestierung (Sigstore/SLSA), keine Drittanbieter-Actions beim Veröffentlichen |

Web-Tests: `cd web/tests && npm install && npx playwright install chromium && npm test`. Nach jeder Änderung an `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
