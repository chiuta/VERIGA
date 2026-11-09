# VERIGA — Link-Checker (unabhängige Implementierung)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Version: 1.1.0 · Zuletzt aktualisiert: 30.10.2026 · Präsentationsseite: https://veriga.chiuta.ro

Eine 64-Bit-Rust-Anwendung für Windows, macOS und Linux (x86-64 und ARM64), die eine Website
oder einen lokalen Ordner durchsucht und deren Links prüft. Sie gibt es in drei Formen, die ein gemeinsames Ergebnismodell teilen:

| Form | Wo | Was sie tut |
|---|---|---|
| `veriga` (CLI) | Terminal, CI, Container (statische musl-Binärdatei) | vollständige Prüfung, Berichte, Exit-Code |
| `veriga-gui` | native Desktop-Anwendung (egui) | sortier-/filterbare Tabelle, Detailbereich, Pause/Stopp, Export |
| `web/veriga.html` | jeder Browser, einzelne Datei, offline | Analyse lokaler Ordner, Online-Site (im Rahmen von CORS), URL-Liste |

Sitzungen (`.veriga`, JSON) sind zwischen den drei Formen austauschbar.

## Zum Begriff „Clean-Room“

VERIGA ist eine **unabhängige Implementierung**, kein dokumentiertes Clean-Room-Projekt. Es wurde kein förmliches Clean-Room-Verfahren (getrennte Analyse- und Implementierungsteams, geprüfte Spezifikationen) angewandt, und der Verlauf des Repositorys beginnt mit einem einzigen großen Commit. Der Code entstand aus dem allgemeinen Verhalten von Link-Prüfern und aus öffentlichen Spezifikationen (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Die Entwicklungssitzungen des Autors zeigen keine Verwendung von Code oder ausführbarer Datei eines anderen Link-Prüfers. Herkunftsnachweis und offene Punkte: [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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
| Abhängigkeiten | reqwest 0.13 (rustls + Plattform-Verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 Schwachstellen; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: zusammengeführte Gruppen, exakter Abgleich mit dem Produkt-Token, Prozent-/UTF-8-Normalisierung, `*` und `$`, längste Regel gewinnt (Allow bei Gleichstand), 500-KiB-Grenze, 4xx ⇒ erlaubt, 5xx/Netzwerk ⇒ nicht verfügbar, die Prüfung läuft weiter (eine tote Website erscheint als defekt, nicht als „0 defekt“); die Startseite wird nie durch robots.txt blockiert |
| HTTP | RFC 9110: `Retry-After` (Sekunden/Datum) bei 429/503, Weiterleitungen 301/302/303/307/308, relatives `Location`, `HEAD` mit Rückfall auf `GET` |
| URL | WHATWG URL / IDNA (UTS 46) über die `url`-Crate (mit `idna`) |
| Sitemap | sitemaps.org 0.9 (Index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 Verstöße, helles/dunkles Design), WAI-ARIA-APG-Tabs mit Tastatur, `prefers-reduced-motion`, `forced-colors`, Ziele ≥ 24 px, `status`-/`progressbar`-Regionen; **CSP** mit Hashes (kein `unsafe-inline` für Skripte), `referrer: no-referrer`; exportierte Berichte tragen ihre eigene CSP |
| Lieferkette | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, Releases mit SHA-256-Summen und einer Provenienz-Attestierung (Sigstore/SLSA), keine Drittanbieter-Actions beim Veröffentlichen |

Web-Tests: `cd web/tests && npm install && npx playwright install chromium && npm test`. Nach jeder Änderung an `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
