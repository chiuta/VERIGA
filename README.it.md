# VERIGA — verificatore di link (implementazione indipendente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Versione: 1.1.0 · Ultimo aggiornamento: 30.10.2026 · Sito di presentazione: https://veriga.chiuta.ro

Un'applicazione Rust a 64 bit per Windows, macOS e Linux (x86-64 e ARM64) che analizza un sito web
o una cartella locale e ne verifica i link. Esiste in tre forme che condividono lo stesso modello di risultati:

| Forma | Dove | Cosa fa |
|---|---|---|
| `veriga` (CLI) | terminale, CI, container (binario statico musl) | verifica completa, report, codice di uscita |
| `veriga-gui` | desktop nativo (egui) | tabella ordinabile/filtrabile, pannello di dettaglio, pausa/stop, esportazione |
| `web/veriga.html` | qualsiasi browser, file singolo, offline | analisi di cartelle locali, siti online (nei limiti di CORS), elenco di URL |

Le sessioni (`.veriga`, JSON) sono intercambiabili tra le tre forme.

## Sull'etichetta "clean-room"

VERIGA è un'**implementazione indipendente**, non un progetto clean-room documentato. Non è stata applicata una procedura formale (team separati di analisi e implementazione, specifiche riviste) e la cronologia del repository inizia con un unico grande commit. Il codice è stato scritto a partire dal comportamento generale dei verificatori di link e da specifiche pubbliche (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Le sessioni di sviluppo dell'autore non mostrano l'uso del codice o dell'eseguibile di un altro verificatore. Il registro di provenienza e i punti aperti sono in [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

## Sicurezza e modello di minaccia

VERIGA elabora contenuti non attendibili (pagine web, file esportati, file di configurazione). Misure predefinite:

- **Anti-SSRF:** le destinazioni private/locali (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) sono bloccate, anche dopo un redirect. Esclusione esplicita: `--allow-private` (CLI) / la casella nella GUI. *Limite:* il DNS rebinding tra il controllo e la connessione non è chiuso del tutto.
- Gli **header sensibili** (`Authorization`, `Cookie`, chiavi API) vengono inviati solo all'origine interna, mai ad altri host né dopo un redirect esterno.
- **Filtro anti-injection per le riparazioni:** un nuovo URL (es. un `Location` ostile) che contenga virgolette, `<`, `>`, spazi, backtick, backslash o parentesi in Markdown viene rifiutato (`unsafe_url_for` / `gUnsafeUrl`).
- **Scritture sicure:** backup univoco, scritture atomiche, blocco cooperativo, nuova lettura del contenuto prima dell'applicazione (TOCTOU), conservazione di permessi/symlink; le proposte obsolete vengono scartate, non applicate in posizioni errate.
- **Esportazione CSV** protetta dalle formule (`=`, `+`, `-`, `@`).
- **Notifiche:** `sendmail` deve essere un percorso assoluto; non usare file di configurazione da fonti non attendibili (potrebbero causare l'esecuzione di un binario scelto da altri).
- **Limiti noti:** l'HTML con escape nei WXR non viene analizzato (falsi negativi); l'edizione web è limitata da CORS; `lint` termina con codice 2 quando trova problemi.

## Standard e conformità (verificati il 29.10.2026)

| Area | Stato |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (core + CLI; le loro dipendenze dichiarano al massimo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` e `cargo fmt --check` senza avvisi; `--locked` in CI |
| Dipendenze | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 vulnerabilità; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: gruppi uniti, corrispondenza esatta sul product token, normalizzazione percent/UTF-8, `*` e `$`, vince la regola più lunga (Allow in caso di parità), limite di 500 KiB, 4xx ⇒ consentito, 5xx/rete ⇒ non disponibile, il controllo continua (un sito morto appare rotto, non «0 rotti»); la pagina iniziale non è mai bloccata da robots.txt |
| HTTP | RFC 9110: `Retry-After` (secondi/data) su 429/503, redirect 301/302/303/307/308, `Location` relativo, `HEAD` con ripiego su `GET` |
| URL | WHATWG URL / IDNA (UTS 46) tramite il crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violazioni, tema chiaro/scuro), schede WAI-ARIA APG con tastiera, `prefers-reduced-motion`, `forced-colors`, target ≥ 24 px, regioni `status`/`progressbar`; **CSP** con hash (niente `unsafe-inline` per gli script), `referrer: no-referrer`; i report esportati includono una CSP propria |
| Supply chain | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, release con somme SHA-256 e attestazione di provenienza (Sigstore/SLSA), nessuna action di terze parti nella pubblicazione |

Test web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Dopo ogni modifica a `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
