# VERIGA — verificatore di link (clean-room)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.0.1

Versione: 1.0.1 · Ultimo aggiornamento: 27.10.2026 · Sito di presentazione: https://veriga.chiuta.ro

Un'applicazione Rust a 64 bit per Windows, macOS e Linux (x86-64 e ARM64) che analizza un sito web
o una cartella locale e ne verifica i link. Esiste in tre forme che condividono lo stesso modello di risultati:

| Forma | Dove | Cosa fa |
|---|---|---|
| `veriga` (CLI) | terminale, CI, container (binario statico musl) | verifica completa, report, codice di uscita |
| `veriga-gui` | desktop nativo (egui) | tabella ordinabile/filtrabile, pannello di dettaglio, pausa/stop, esportazione |
| `web/veriga.html` | qualsiasi browser, file singolo, offline | analisi di cartelle locali, siti online (nei limiti di CORS), elenco di URL |

Le sessioni (`.veriga`, JSON) sono intercambiabili tra le tre forme.

## Cosa verifica

- **Link HTML**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (stili, icone, preload), `form[action]` (facoltativo), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` e `@import`, sia dai file `.css` sia da `<style>` e dagli attributi `style`.
- **Protocolli**: HTTP/HTTPS (HEAD con ripiego su GET), `file://` e percorsi locali, FTP (autenticazione, `SIZE`/`CWD`),
  `mailto:` (sintassi).
- **Redirect**: catena completa con codici di stato, rilevamento dei cicli, limite configurabile, opzione "i redirect sono errori".
- **Ancore `#fragment`**: verificate nelle pagine analizzate (`id` e `name`).
- **File orfani**: file nella directory del sito a cui nessun link rimanda.
- **Differenze di maiuscole/minuscole** nei percorsi locali (funziona su Windows/macOS, si rompe su Linux).
- **robots.txt** (gruppi, `Allow`/`Disallow` con `*` e `$`, `Crawl-delay`) e avvio da **sitemap.xml**.
- **Osservazioni**: titoli duplicati o mancanti, risposte lente, statistiche per tipo MIME.
- Codifiche dei caratteri (header, BOM, `<meta charset>`), comprese le vecchie pagine Windows-1250.

## Opzioni principali

Thread concorrenti e limite per host, timeout, tentativi (nel rispetto di `Retry-After`), profondità massima, limite di URL,
link esterni sì/no, "resta nella directory", escludi / non seguire / solo questi (regex), host interni aggiuntivi,
codici HTTP considerati OK (es. 999), User-Agent, proxy (HTTP/SOCKS), autenticazione Basic (solo host interni),
header e cookie, certificati non validi, ritardo tra le richieste.

## Report

HTML autonomo (si apre ovunque), CSV, TSV, JSON, testo, sitemap.xml, albero del sito, sessione `.veriga`.
**Ricontrollo dei link rotti** da una sessione salvata (`--recheck`, un pulsante nella GUI/web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # cartella locale + orfani
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # solo link rotti
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Codice di uscita `2` se vengono trovati link rotti (utile in CI); `--no-fail` lo disattiva. `veriga --help` elenca tutto. Codice di uscita `3` se l'esecuzione è stata interrotta o troncata da `--max-urls`.

## Compilazione

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # solo CLI
cargo test --workspace
```

Il workflow `.github/workflows/build.yml` produce gli archivi per: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, con GUI) e Linux x64/ARM64 (musl, CLI statica). Il `Dockerfile` costruisce un'immagine minimale con la CLI.

## Tutorial video

Dieci brevi tutorial registrati sull’applicazione reale e sottotitolati in 8 lingue (ro, en, fr, it, es, pt, de, hu): prima scansione, filtri, dettagli di un link, opzioni, esportazione, Guardian (correzioni; domini, mappa, tendenze), riga di comando, edizione HTML ed edizione portatile. Su https://veriga.chiuta.ro/#tutoriale o in `assets/tutorials/`: ogni `.mp4` contiene le 8 tracce di sottotitoli, i file `.vtt` sono affiancati. Senza audio; le spiegazioni sono nei sottotitoli.

## Edizione portatile (PortableApps.com)

VERIGA è disponibile anche come pacchetto **PortableApps.com Format 3.9**, `VERIGAPortable_<versione>_Multilingual.zip` (file della release). Decomprimilo ovunque (chiavetta USB, cartella sincronizzata) e avvia `VERIGAPortable.exe`.

- Tutto ciò che l'app produce resta in `Data\` (stato di Guardian in `Data\.veriga`); nulla viene scritto nel registro o in `%APPDATA%`. Eliminare `Data\` è sicuro: viene ricreata.
- Il launcher sceglie la versione x64 o ARM64 in base all'architettura nativa; `VERIGAPortable.exe sessione.veriga` apre una sessione salvata (associazione file `.veriga`).
- La CLI è `App\VERIGA\veriga.exe`. La struttura è verificata in CI con `portableapps/validate.py`; il sorgente del launcher è `crates/veriga-launcher`.
- L'installer ufficiale `.paf.exe` si costruisce con PortableApps.com Installer (strumento per Windows); la cartella ha già la struttura richiesta.

## Pacchetti per mobile e server

PWA (installabile, offline): <https://veriga.chiuta.ro/app/> · Termux: `packaging/termux/install.sh` · Docker: `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · timer systemd, `.deb`/`.rpm`, CronJob Kubernetes e pacchetto **YunoHost**: vedi [`packaging/`](packaging/README.md). Non è fornito un APK Android nativo.

## L'edizione HTML a file singolo

`web/veriga.html` funziona al 100% nel browser, senza server:

- **Cartella locale** (trascina la cartella del sito, oppure "Scegli cartella"): verifica completa di link interni, CSS, ancore, orfani,
  differenze di maiuscole/minuscole, titoli duplicati. Qui il browser non pone alcun limite.
- **Sito online**: un browser non può leggere lo stato HTTP di altri domini (CORS). Risultati completi se il file viene aperto dallo
  stesso sito, oppure tramite un proxy CORS personale (modello `https://proxy/?u={url}`). Altrimenti i link esterni sono segnati
  *non verificabili* se il server risponde, o *nessuna connessione* se non risponde — mai un falso "OK".
- **Elenco di URL / HTML incollato**.
- Non verifica FTP e non legge robots.txt (restrizioni del browser). Per questi usa la CLI/GUI.

## Guardian — riparazione, cronologia, conformità

VERIGA include anche le funzioni di un "link guardian" (ispirato al plugin WordPress *Trade-Free Link Guardian*), ma come strumento indipendente che lavora sui **file**: HTML, Markdown, esportazione WordPress (WXR `.xml`, menu inclusi) e sitemap.

* **Riparazioni deterministiche, verificate live** — `scheme_typo` (`htp://`, `https//`), `unmangle` (schema doppio, `%20`, punteggiatura finale), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (coerenza), `redirect` (solo 301/308). Una riparazione viene applicata automaticamente **solo se il nuovo URL ha risposto con successo**; senza verifica resta una "proposta".
* **Riparazioni manuali** — sostituzione dell'URL, "rimuovi link" (mantiene il testo), ricerca nella Wayback Machine (su richiesta, mai chiamata automaticamente).
* **Sicurezza** — un backup per ogni esecuzione, un registro delle modifiche, **annullamento** singolo o per esecuzione (byte per byte, `&amp;` incluso); vengono scritti solo i byte dell'intervallo interessato.
* **401/403 = "da riesaminare"**, non un link rotto (opzione, attiva per impostazione predefinita).
* **Viste e report** — domini, mappa di salute per pagina, vista "libro", redirect (permanenti/temporanei), variazione dei contenuti (titolo/H1 rispetto all'esecuzione precedente), nuovi/risolti, tendenze dalla cronologia, **report di conformità** per periodo, un badge d'impatto (nel menu o su ≥ 3 pagine). Le esportazioni CSV neutralizzano le celle con formule.
* **404 dai log di accesso** (Apache/nginx), con suggerimenti di redirect ed esportazione `.htaccess`.
* **Pianificazione** — `veriga wizard` (4 passaggi) scrive `veriga-guardian.json`; `veriga watch --config … [--once]` viene eseguito periodicamente (o da cron/Utilità di pianificazione), conserva la cronologia, ripara automaticamente le regole scelte e notifica tramite webhook o `sendmail`.

```
veriga lint ./site                       # analisi offline
veriga fix ./site --session s.veriga     # propone + verifica live (dry run)
veriga fix ./site --apply                # scrive, con backup e registro
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: il pulsante **🛡 Guardian**. Edizione HTML: la sezione **Guardian** (le modifiche avvengono in memoria; scarichi i file o li scrivi in una cartella dove il browser lo consente; la verifica live è limitata da CORS).

**Cosa non si può replicare da un plugin WordPress:** la ricerca dei vecchi slug nel database, l'hook di salvataggio dell'editor, WP-Cron, multisite e `wp_mail`. Al loro posto: importazione WXR, file su disco, `watch`/cron e `sendmail`/webhook. L'elenco delle funzioni è stato ricostruito dalla descrizione pubblica e da conversazioni precedenti, non dal codice sorgente del plugin.

Sostieni il progetto: https://www.buymeacoffee.com/echo.of.the.strings

## Limiti noti

- Non invia report via e-mail (SMTP) e non rileva gli orfani via FTP, solo in una directory locale.
- Le sitemap compresse `.xml.gz` non vengono lette.
- Le pagine che costruiscono i link con JavaScript non vengono renderizzate (non c'è un motore di browser); si analizza l'HTML servito.
- L'interfaccia grafica è stata provata solo su un display virtuale (Linux/X11): scansione, filtri, pannello dei dettagli e Guardian. Le versioni per Windows e macOS sono prodotte dalla CI ma non sono state testate in modo interattivo.

## Sul "clean-room"

Il codice è scritto da zero, sulla base del comportamento generale dei verificatori di link e di specifiche pubbliche
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Non contiene codice, risorse, testi o design di alcun altro prodotto e
non è stato scritto guardando il codice o l'eseguibile di un altro programma.

Licenza: MIT.

## Sicurezza e modello di minaccia

VERIGA elabora contenuti non attendibili (pagine web, file esportati, file di configurazione). Misure predefinite:

- **Anti-SSRF:** le destinazioni private/locali (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) sono bloccate, anche dopo un redirect. Esclusione esplicita: `--allow-private` (CLI) / la casella nella GUI. *Limite:* il DNS rebinding tra il controllo e la connessione non è chiuso del tutto.
- Gli **header sensibili** (`Authorization`, `Cookie`, chiavi API) vengono inviati solo all'origine interna, mai ad altri host né dopo un redirect esterno.
- **Filtro anti-injection per le riparazioni:** un nuovo URL (es. un `Location` ostile) che contenga virgolette, `<`, `>`, spazi, backtick, backslash o parentesi in Markdown viene rifiutato (`unsafe_url_for` / `gUnsafeUrl`).
- **Scritture sicure:** backup univoco, scritture atomiche, blocco cooperativo, nuova lettura del contenuto prima dell'applicazione (TOCTOU), conservazione di permessi/symlink; le proposte obsolete vengono scartate, non applicate in posizioni errate.
- **Esportazione CSV** protetta dalle formule (`=`, `+`, `-`, `@`).
- **Notifiche:** `sendmail` deve essere un percorso assoluto; non usare file di configurazione da fonti non attendibili (potrebbero causare l'esecuzione di un binario scelto da altri).
- **Limiti noti:** l'HTML con escape nei WXR non viene analizzato (falsi negativi); l'edizione web è limitata da CORS; `lint` termina con codice 2 quando trova problemi.

## Standard e conformità (verificati il 27.10.2026)

| Area | Stato |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (core + CLI; le loro dipendenze dichiarano al massimo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` e `cargo fmt --check` senza avvisi; `--locked` in CI |
| Dipendenze | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 vulnerabilità; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: gruppi uniti, corrispondenza esatta sul product token, normalizzazione percent/UTF-8, `*` e `$`, vince la regola più lunga (Allow in caso di parità), limite di 500 KiB, 4xx ⇒ consentito, 5xx/rete ⇒ non consentito |
| HTTP | RFC 9110: `Retry-After` (secondi/data) su 429/503, redirect 301/302/303/307/308, `Location` relativo, `HEAD` con ripiego su `GET` |
| URL | WHATWG URL / IDNA (UTS 46) tramite il crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violazioni, tema chiaro/scuro), schede WAI-ARIA APG con tastiera, `prefers-reduced-motion`, `forced-colors`, target ≥ 24 px, regioni `status`/`progressbar`; **CSP** con hash (niente `unsafe-inline` per gli script), `referrer: no-referrer`; i report esportati includono una CSP propria |
| Supply chain | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, release con somme SHA-256 e attestazione di provenienza (Sigstore/SLSA), nessuna action di terze parti nella pubblicazione |

Test web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Dopo ogni modifica a `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
