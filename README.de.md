# VERIGA — Link-Checker (unabhängige Implementierung)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.5.1

Version: 1.5.1 · Zuletzt aktualisiert: 2026-11-22 · Präsentationsseite: https://veriga.chiuta.ro

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

`--honor-nofollow` — Links mit `rel="nofollow"` werden weiterhin geprüft (HTTP-Status), die Zielseite wird aber nicht nach weiteren Links durchsucht; standardmäßig aus. Mit `--report-rel` werden zusätzlich die `rel`-Token jedes Links (ohne Beachtung der Groß-/Kleinschreibung, eindeutig) im JSON (`rel`), in CSV/TSV (Spalte `Rel`) und neben defekten Links im Textbericht ausgegeben; ohne diese Option entsprechen die Ausgaben genau denen von v1.1.0.

`--check-schemes` — prüft die Syntax von `tel:`-, `geo:`- und `data:`-Adressen (RFC 3966, RFC 5870, RFC 2397 mit der Base64-Toleranz von Fetch); es wird keine Netzwerkanfrage gesendet. Ungültige Adressen werden als „Ungültige URL“ mit Begründung gemeldet; `data:`-Adressen über 1 MiB gelten als nicht geprüft. Standardmäßig aus (diese Schemata werden dann stillschweigend ignoriert).

`--check-link-names` — ermittelt den zugänglichen Namen (accessible name) jedes `<a href>` (aria-labelledby, aria-label, Inhalt einschließlich des `alt` von Bildern, dann `title`; verborgener Inhalt wird ignoriert) und meldet Links ohne Namen oder nur mit Satzzeichen (WCAG 2.4.4 / 4.1.2) in den Text- und JSON-Berichten (`a11y`). `--check-ambiguous-names` fügt einen Hinweis hinzu, wenn gleiche Namen (ohne Beachtung der Groß-/Kleinschreibung) auf einer Seite auf unterschiedliche Adressen verweisen. Beide standardmäßig aus.

`--check-link-header` — liest die HTTP-Headerfelder `Link` (RFC 8288) der durchsuchten Seiten und prüft jedes angegebene Ziel wie jeden anderen Link (Quelle = die angefragte Seite, angezeigt als „Link header: …“; die `rel`-Token bleiben erhalten). Fehlerhafte Werte werden mit einer Warnung übersprungen (Text-/JSON-Berichte, `warnings`); pro Antwort werden höchstens 100 Links gelesen. Standardmäßig aus.

`--check-seo-links` — prüft `rel="canonical"` (defektes Ziel, umgeleitetes Ziel getrennt gemeldet, mehrere verschiedene Canonicals auf einer Seite; ein Canonical, der auf die Seite selbst zeigt, ist in Ordnung, auch aus dem `Link`-Header gelesen, mit `--check-link-header`) und `hreflang`-Werte bei `link rel=alternate` / `a` (nur BCP-47-Syntax, ohne IANA-Registry; `x-default` wird als externe Konvention aufgeführt, nicht als Fehler). Die Ergebnisse stehen in den Text- und JSON-Berichten (`seo`). Standardmäßig aus.

`--check-mixed-content` — meldet auf `https:`-Seiten Unterressourcen, die über `http:` geladen werden (Mixed Content): Bilder, Audio und Video sind „optional blockierbar“; Skripte, Stylesheets, Preloads, iframes, Objekte, Schriften, CSS-`@import` und Formularaktionen sind „blockierbar“. Einfache Navigationslinks sind keine Unterressourcen; `localhost`, `127.0.0.0/8` und `[::1]` werden nie gemeldet; ein Header `Content-Security-Policy: upgrade-insecure-requests` fügt einen Hinweis hinzu, statt den Befund aufzuheben; eine `https:`-Unterressource, die auf `http:` umleitet, wird ebenfalls gemeldet. Die Ergebnisse stehen in den Text- und JSON-Berichten (`mixed`). Standardmäßig aus.

`--check-feeds` — liest RSS-2.0- und Atom-1.0-Feeds, die über `<link rel="alternate" type="application/rss+xml|application/atom+xml">` gefunden werden (oder jede beim Durchsuchen erhaltene XML-Antwort), und prüft die darin enthaltenen Links: `link`, `comments`, `enclosure@url`, `source@url` von Kanal und Einträgen, `url`/`link` des Bildes, eine dauerhafte `guid`, Atom-`link@href` (fehlendes `rel` = alternate), `content@src` sowie Links in maskierten/CDATA-HTML-Beschreibungen und im Atom-Inhalt `html`/`xhtml`. Relative Adressen folgen `xml:base`, dann der Feed-Adresse. Fehlerhaftes XML oder ein Dokument, das kein Feed ist, führt zu einer Warnung und zu keinen Links. Standardmäßig aus.

`--check-sri` — prüft das Attribut `integrity` von `<script src>` und `<link rel=stylesheet|preload|modulepreload>`: Syntax (Standard-Base64, richtige Digest-Länge), erkannte Algorithmen (sha256/384/512 — der stärkste vorhandene entscheidet, jeder seiner Werte darf passen), den SHA-2-Digest der heruntergeladenen Bytes und CORS bei Ressourcen anderer Herkunft (Attribut `crossorigin` plus ein passendes `Access-Control-Allow-Origin`). Befunde: stimmt überein / stimmt nicht überein / ungültige Syntax / kein erkannter Algorithmus / kein CORS / nicht geprüft; `integrity` an anderen Elementen wird als nicht anwendbar vermerkt. Ressourcen werden nur dann vollständig geladen (GET), wenn ein Link die Prüfung verlangt. Die Ergebnisse stehen in den Text- und JSON-Berichten (`sri`). Standardmäßig aus.

`--check-cert-expiry` / `--cert-warn-days N` (Standard 14) — liest das TLS-Endzertifikat (Leaf-Zertifikat), das jeder `https`-Host:Port während der Prüfung vorlegt, und meldet einmal pro Host:Port mit der Zahl der dort geprüften Links: abgelaufen, noch nicht gültig, läuft in N Tagen ab (Schwelle; `0` = nur abgelaufen), nicht lesbar oder ein Name, der zu keinem SAN-Eintrag passt. Funktioniert auch mit `--insecure`. Ein TLS-abfangender Proxy zeigt sein eigenes Zertifikat; eine falsch gehende Systemuhr lässt sich nicht erkennen. Die Ergebnisse stehen in den Text- und JSON-Berichten (`certs`). Standardmäßig aus.

## Kostenlose, ethische Proxys

Wähle eine Voreinstellung mit `--proxy-preset tor|tor-browser|warp` (Liste: `--list-proxy-presets`) oder in der GUI unter „Kostenloser ethischer Proxy…“. Alle sind lokale SOCKS5-Endpunkte (`socks5h://127.0.0.1:…`): Namen löst der Proxy auf, nicht dein System. Tor (Freiwillige, gemeinnützig, Open Source) ist die empfohlene Wahl; die Prüfung ist auf 4 Threads und 2 Verbindungen pro Server begrenzt, und manche Seiten sperren Tor-Ausgänge, daher sind falsche „defekte Links“ möglich. Cloudflare WARP ist kostenlos, aber ein kommerzieller Dienst mit geschlossenem Client. Listen offener öffentlicher Proxys fehlen absichtlich: Sie sind oft kompromittiert oder fangen Verkehr ab. Ein eigener Proxy bleibt mit `--proxy` möglich.

## Berichte

Eigenständiges HTML (öffnet sich überall), CSV, TSV, JSON, Text, sitemap.xml, ein Site-Baum, eine `.veriga`-Sitzung.
**Erneute Prüfung defekter Links** aus einer gespeicherten Sitzung (`--recheck`, eine Schaltfläche in GUI/Web).

**Alias-Adressen** (`--report-aliases`): verschiedene Schreibweisen derselben Adresse (Groß-/Kleinschreibung des Hosts, Standardport, unnötig prozentkodierte Zeichen, Punktsegmente) werden in den Text- und JSON-Berichten als Informationsgruppen aufgeführt (`aliases`); Groß-/Kleinschreibung in Pfad/Query, Parameterreihenfolge, abschließender Schrägstrich und `www.` werden nie zusammengefasst. Die Erfassung ist speicherbegrenzt (100.000 Adressen, 10 Schreibweisen je Adresse, 20 Seiten je Schreibweise). `--canonical-urls` führt solche Schreibweisen zusätzlich zu einer einzigen Prüfung zusammen (und meldet die normalisierte Adresse); standardmäßig werden URLs genau so geprüft und gemeldet, wie sie geschrieben sind, wie in v1.1.0. Ein weiterer, standardmäßig ausgeschalteter Schalter: `--xml-encoding` (`<?xml encoding=…?>` beachten, wenn weder Header noch Meta-Charset angegeben ist).

**SARIF 2.1.0** (`-o report.sarif`): Defekte Links werden zu `error`, Weiterleitungen / 401-403 / langsame Antworten zu `warning`, übersprungene oder verwaiste Einträge zu `note`; gute Links werden nicht ausgegeben. Der Ort ist die Seite (oder Datei), die den Link enthält, als URI; `region.startLine` wird nur geschrieben, wenn eine Zeile bekannt ist (Crawl-Ergebnisse tragen derzeit keine Zeile, daher entfällt sie). Die Ausgabe ist deterministisch und wird atomar geschrieben.

**JUnit XML** (`-o report.junit`): ein `testsuite` je Quellseite, ein `testcase` je geprüftem Link (`name` = Ziel, `classname` = Quellseite). Defekte Links sind `failure`; Zeitüberschreitungen / keine Verbindung sind `error`; übersprungene oder nur zu prüfende Einträge sind `skipped`; in XML 1.0 unzulässige Zeichen werden zu U+FFFD. Verwaiste Dateien sind keine Links und bleiben außen vor.

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # lokaler Ordner + verwaiste Dateien
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # nur defekte Links
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Exit-Code `2`, wenn defekte Links gefunden wurden (nützlich in CI); `--no-fail` schaltet das ab. `veriga --help` listet alles auf. Exit-Code `3`, wenn der Lauf abgebrochen oder durch `--max-urls` / `--max-per-pattern` (Crawler-Fallen, Standard 5000 Seiten pro Pfadmuster) abgeschnitten wurde; `3` hat Vorrang vor `2`. `veriga check URL` entspricht `veriga URL`.

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

## Pakete für Mobilgeräte und Server

PWA (installierbar, offline): <https://veriga.chiuta.ro/app/> · Termux: `packaging/termux/install.sh` · Docker: `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · systemd-Timer, `.deb`/`.rpm`, Kubernetes-CronJob und **YunoHost**-Paket: siehe [`packaging/`](packaging/README.md). Eine native Android-APK wird nicht bereitgestellt.

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

- **Netzwerkschutz:** Die Prüfung auf private Adressen löst den Host auf, danach löst ihn der HTTP-Client erneut auf (ein DNS-Rebinding-Fenster besteht); führen Sie VERIGA auf nicht vertrauenswürdigen Sites aus einem Netz aus, das private Bereiche nicht erreichen kann.
- **`veriga watch`** liest standardmäßig `./veriga-guardian.json` aus dem aktuellen Verzeichnis, und diese Datei kann einen `sendmail`-Befehl und die zu überschreibenden Quellen benennen: nicht in einem Verzeichnis ausführen, dem Sie nicht vertrauen.
- Berichte werden nicht per E-Mail (SMTP) versendet, und verwaiste Dateien werden nicht über FTP erkannt — nur in einem lokalen Verzeichnis.
- Komprimierte `.xml.gz`-Sitemaps werden gelesen (gzip wird an den ersten Bytes erkannt, nicht an der Endung); die entpackte Größe ist auf 50 MB begrenzt, und eine Sitemap wird bis zu ihren ersten 50.000 URLs gelesen.
- Seiten, die ihre Links mit JavaScript aufbauen, werden nicht gerendert (es gibt keine Browser-Engine); analysiert wird das ausgelieferte HTML.
- Die grafische Oberfläche wurde nur unter einem virtuellen Display (Linux/X11) erprobt: Crawl, Filter, Detailbereich und Guardian. Die Windows- und macOS-Builds entstehen in der CI, wurden aber nicht interaktiv getestet.

## Zum Begriff „Clean-Room“

VERIGA ist eine **unabhängige Implementierung**, kein dokumentiertes Clean-Room-Projekt. Es wurde kein förmliches Clean-Room-Verfahren (getrennte Analyse- und Implementierungsteams, geprüfte Spezifikationen) angewandt, und der Verlauf des Repositorys beginnt mit einem einzigen großen Commit. Der Code entstand aus dem allgemeinen Verhalten von Link-Prüfern und aus öffentlichen Spezifikationen (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Dass kein Code und keine ausführbare Datei eines anderen Link-Prüfers verwendet wurde, ist eine Erklärung des Autors und nicht bewiesen: Die Entwicklungssitzungen wurden nur teilweise geprüft, und das Fehlen einer solchen Verwendung in den ungeprüften Teilen lässt sich nicht belegen. Herkunftsnachweis und offene Punkte:
[docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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

## Standards und Compliance (geprüft am 29.10.2026)

| Bereich | Status |
|---|---|
| Rust | Edition **2024**, MSRV **1.88** (Kern + CLI; deren Abhängigkeiten deklarieren höchstens 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` und `cargo fmt --check` sauber; `--locked` in CI |
| Abhängigkeiten | reqwest 0.13 (rustls + Plattform-Verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, jiff 1.0, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 Schwachstellen; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: zusammengeführte Gruppen, exakter Abgleich mit dem Produkt-Token, Prozent-/UTF-8-Normalisierung, `*` und `$`, längste Regel gewinnt (Allow bei Gleichstand), 500-KiB-Grenze, 4xx ⇒ erlaubt, 5xx/Netzwerk ⇒ nicht verfügbar, die Prüfung läuft weiter (eine tote Website erscheint als defekt, nicht als „0 defekt“); die Startseite wird nie durch robots.txt blockiert |
| HTTP | RFC 9110: `Retry-After` (Sekunden/Datum) bei 429/503, Weiterleitungen 301/302/303/307/308, relatives `Location`, `HEAD` mit Rückfall auf `GET` |
| URL | WHATWG URL / IDNA (UTS 46) über die `url`-Crate (mit `idna`) |
| Sitemap | sitemaps.org 0.9 (Index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 Verstöße, helles/dunkles Design), WAI-ARIA-APG-Tabs mit Tastatur, `prefers-reduced-motion`, `forced-colors`, Ziele ≥ 24 px, `status`-/`progressbar`-Regionen; **CSP** mit Hashes (kein `unsafe-inline` für Skripte), `referrer: no-referrer`; exportierte Berichte tragen ihre eigene CSP |
| Lieferkette | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, Releases mit SHA-256-Summen und einer Provenienz-Attestierung (Sigstore/SLSA), keine Drittanbieter-Actions beim Veröffentlichen |

Web-Tests: `cd web/tests && npm install && npx playwright install chromium && npm test`. Nach jeder Änderung an `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
