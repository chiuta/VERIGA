# VERIGA — verificatore di link (implementazione indipendente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.5.1

Versione: 1.5.1 · Ultimo aggiornamento: 2026-11-22 · Sito di presentazione: https://veriga.chiuta.ro

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

`--honor-nofollow` — i link contrassegnati con `rel="nofollow"` vengono comunque controllati (stato HTTP), ma la pagina di destinazione non viene scansionata alla ricerca di altri link; disattivato per impostazione predefinita. Con `--report-rel`, anche i token `rel` di ogni link (senza distinzione tra maiuscole e minuscole, univoci) sono riportati nel JSON (`rel`), nel CSV/TSV (colonna `Rel`) e accanto ai link interrotti nel report di testo; senza l'opzione, queste uscite sono esattamente quelle della v1.1.0.

`--check-schemes` — convalida la sintassi degli indirizzi `tel:`, `geo:` e `data:` (RFC 3966, RFC 5870, RFC 2397 con la tolleranza base64 di Fetch); non viene effettuata alcuna richiesta di rete. Quelli non validi sono segnalati come «URL non valido» con il motivo; gli indirizzi `data:` oltre 1 MiB sono segnalati come non verificati. Disattivato per impostazione predefinita (quegli schemi sono ignorati in silenzio).

`--check-link-names` — calcola il nome accessibile di ogni `<a href>` (aria-labelledby, aria-label, contenuto compreso l'`alt` delle immagini, poi `title`; il contenuto nascosto è ignorato) e segnala i link senza nome o con soli segni di punteggiatura (WCAG 2.4.4 / 4.1.2) nei report di testo e JSON (`a11y`). `--check-ambiguous-names` aggiunge una nota informativa quando nomi identici (senza distinzione tra maiuscole e minuscole) puntano a indirizzi diversi in una stessa pagina. Entrambi disattivati per impostazione predefinita.

`--check-link-header` — legge i campi di intestazione HTTP `Link` (RFC 8288) delle pagine scansionate e controlla ogni destinazione dichiarata come qualsiasi altro link (origine = la pagina richiesta, mostrata come «Link header: …»; i token `rel` sono conservati). I valori malformati sono saltati con un avviso (report di testo/JSON, `warnings`); per ogni risposta si leggono al massimo 100 link. Disattivato per impostazione predefinita.

`--check-seo-links` — controlla `rel="canonical"` (destinazione interrotta, destinazione reindirizzata segnalata a parte, più canonical diversi in una pagina; un canonical che punta alla pagina stessa è corretto, anche se letto dall'intestazione `Link` con `--check-link-header`) e i valori `hreflang` su `link rel=alternate` / `a` (solo sintassi BCP 47, senza il registro IANA; `x-default` è elencato come convenzione esterna, non come errore). I risultati sono nei report di testo e JSON (`seo`). Disattivato per impostazione predefinita.

`--check-mixed-content` — nelle pagine `https:`, segnala le sottorisorse caricate tramite `http:` (contenuto misto): immagini, audio e video sono «bloccabili facoltativamente»; script, fogli di stile, precaricamenti, iframe, oggetti, font, `@import` CSS e azioni dei moduli sono «bloccabili». I semplici link di navigazione non sono sottorisorse; `localhost`, `127.0.0.0/8` e `[::1]` non vengono mai segnalati; un'intestazione `Content-Security-Policy: upgrade-insecure-requests` aggiunge una nota invece di annullare il rilievo; viene segnalata anche una sottorisorsa `https:` che reindirizza a `http:`. I risultati sono nei report di testo e JSON (`mixed`). Disattivato per impostazione predefinita.

`--check-feeds` — legge i feed RSS 2.0 e Atom 1.0 trovati tramite `<link rel="alternate" type="application/rss+xml|application/atom+xml">` (o qualsiasi risposta XML ottenuta durante la scansione) e controlla i link che contengono: `link`, `comments`, `enclosure@url`, `source@url` del canale e degli elementi, `url`/`link` dell'immagine, un `guid` permanente, `link@href` di Atom (`rel` assente = alternate), `content@src` e i link nelle descrizioni HTML con escape/CDATA e nel contenuto Atom `html`/`xhtml`. Gli indirizzi relativi seguono `xml:base`, poi l'indirizzo del feed. Un XML malformato o un documento che non è un feed produce un avviso e nessun link. Disattivato per impostazione predefinita.

`--check-sri` — verifica l'attributo `integrity` di `<script src>` e di `<link rel=stylesheet|preload|modulepreload>`: sintassi (base64 standard, lunghezza del digest corretta), algoritmi riconosciuti (sha256/384/512: decide il più forte presente, può corrispondere uno qualsiasi dei suoi valori), il digest SHA-2 dei byte scaricati e CORS per le risorse di altra origine (attributo `crossorigin` più un `Access-Control-Allow-Origin` corrispondente). Esiti: corrisponde / non corrisponde / sintassi non valida / nessun algoritmo riconosciuto / niente CORS / non verificato; `integrity` su altri elementi è annotato come non applicabile. Le risorse sono scaricate per intero (GET) solo quando un link richiede il controllo. I risultati sono nei report di testo e JSON (`sri`). Disattivato per impostazione predefinita.

`--check-cert-expiry` / `--cert-warn-days N` (predefinito 14) — legge il certificato TLS foglia che ogni host:porta `https` presenta durante il controllo e segnala, una volta per host:porta con il numero di link controllati lì: scaduto, non ancora valido, scade tra N giorni (soglia; `0` = solo scaduti), illeggibile, o un nome che non corrisponde ad alcuna voce SAN. Funziona anche con `--insecure`. Un proxy che intercetta il TLS mostra il proprio certificato; un orologio di sistema sbagliato non può essere rilevato. I risultati sono nei report di testo e JSON (`certs`). Disattivato per impostazione predefinita.

## Proxy gratuiti ed etici

Scegli un preset con `--proxy-preset tor|tor-browser|warp` (elenco: `--list-proxy-presets`) oppure, nella GUI, da «Proxy gratuito etico…». Sono tutti endpoint SOCKS5 locali (`socks5h://127.0.0.1:…`): i nomi sono risolti dal proxy, non dal tuo sistema. Tor (volontari, senza scopo di lucro, open source) è la scelta consigliata; il controllo è limitato a 4 thread e 2 connessioni per server e alcuni siti bloccano le uscite Tor, quindi possono comparire falsi «link rotti». Cloudflare WARP è gratuito ma è un servizio commerciale con client chiuso. Gli elenchi di proxy pubblici aperti sono esclusi di proposito: sono spesso compromessi o intercettano il traffico. Il proxy personale resta disponibile con `--proxy`.

## Report

HTML autonomo (si apre ovunque), CSV, TSV, JSON, testo, sitemap.xml, albero del sito, sessione `.veriga`.
**Ricontrollo dei link rotti** da una sessione salvata (`--recheck`, un pulsante nella GUI/web).

**Indirizzi alias** (`--report-aliases`): le diverse grafie dello stesso indirizzo (maiuscole dell'host, porta predefinita, caratteri codificati con percentuale senza necessità, segmenti con punto) sono elencate come gruppi informativi nei report di testo e JSON (`aliases`); maiuscole di percorso/query, ordine dei parametri, barra finale e `www.` non vengono mai raggruppati. Il tracciamento ha un limite di memoria (100.000 indirizzi, 10 grafie per indirizzo, 20 pagine per grafia). `--canonical-urls` unisce inoltre tali grafie in un unico controllo (e riporta l'indirizzo normalizzato); per impostazione predefinita gli URL sono controllati e riportati esattamente come scritti, come nella v1.1.0. Un altro interruttore disattivato per impostazione predefinita: `--xml-encoding` (rispettare `<?xml encoding=…?>` quando non sono dati né intestazione né meta charset).

**SARIF 2.1.0** (`-o report.sarif`): i link interrotti diventano `error`, i reindirizzamenti / 401-403 / le risposte lente `warning`, gli elementi saltati o orfani `note`; i link buoni non vengono emessi. La posizione è la pagina (o il file) che contiene il link, come URI; `region.startLine` è scritto solo quando una riga è nota (i risultati della scansione per ora non hanno riga, quindi è omesso). L'output è deterministico e scritto in modo atomico.

**JUnit XML** (`-o report.junit`): un `testsuite` per pagina di origine, un `testcase` per link controllato (`name` = destinazione, `classname` = pagina di origine). I link interrotti sono `failure`; timeout / assenza di connessione sono `error`; gli elementi saltati o solo da rivedere sono `skipped`; i caratteri non consentiti in XML 1.0 diventano U+FFFD. I file orfani non sono link e restano fuori.

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # cartella locale + orfani
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # solo link rotti
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Codice di uscita `2` se vengono trovati link rotti (utile in CI); `--no-fail` lo disattiva. `veriga --help` elenca tutto. Codice di uscita `3` se l'esecuzione è stata interrotta o troncata da `--max-urls` / `--max-per-pattern` (trappole di scansione, predefinito 5000 pagine per modello di percorso); `3` ha la precedenza su `2`. `veriga check URL` equivale a `veriga URL`.

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

- **Protezione di rete:** il controllo degli indirizzi privati risolve l'host, poi il client HTTP lo risolve di nuovo (esiste una finestra di DNS rebinding); esegui VERIGA su siti non attendibili da una rete che non possa raggiungere gli intervalli privati.
- **`veriga watch`** legge per impostazione predefinita `./veriga-guardian.json` dalla cartella corrente, e quel file può indicare un comando `sendmail` e le sorgenti da riscrivere: non eseguirlo in una cartella di cui non ti fidi.
- Non invia report via e-mail (SMTP) e non rileva gli orfani via FTP, solo in una directory locale.
- I sitemap compressi `.xml.gz` vengono letti (gzip è riconosciuto dai primi byte, non dall'estensione); la dimensione decompressa è limitata a 50 MB e un sitemap è letto fino ai suoi primi 50.000 URL.
- Le pagine che costruiscono i link con JavaScript non vengono renderizzate (non c'è un motore di browser); si analizza l'HTML servito.
- L'interfaccia grafica è stata provata solo su un display virtuale (Linux/X11): scansione, filtri, pannello dei dettagli e Guardian. Le versioni per Windows e macOS sono prodotte dalla CI ma non sono state testate in modo interattivo.

## Sul "clean-room"

VERIGA è un'**implementazione indipendente**, non un progetto clean-room documentato. Non è stata applicata una procedura formale (team separati di analisi e implementazione, specifiche riviste) e la cronologia del repository inizia con un unico grande commit. Il codice è stato scritto a partire dal comportamento generale dei verificatori di link e da specifiche pubbliche (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Che non siano stati usati codice o eseguibile di un altro verificatore è una dichiarazione dell'autore e non è dimostrato: le sessioni di sviluppo sono state esaminate solo in parte, e l'assenza di tale uso nelle parti non esaminate non può essere provata. Il registro di provenienza e i punti aperti sono in
[docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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

## Standard e conformità (verificati il 29.10.2026)

| Area | Stato |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (core + CLI; le loro dipendenze dichiarano al massimo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` e `cargo fmt --check` senza avvisi; `--locked` in CI |
| Dipendenze | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, jiff 1.0, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 vulnerabilità; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: gruppi uniti, corrispondenza esatta sul product token, normalizzazione percent/UTF-8, `*` e `$`, vince la regola più lunga (Allow in caso di parità), limite di 500 KiB, 4xx ⇒ consentito, 5xx/rete ⇒ non disponibile, il controllo continua (un sito morto appare rotto, non «0 rotti»); la pagina iniziale non è mai bloccata da robots.txt |
| HTTP | RFC 9110: `Retry-After` (secondi/data) su 429/503, redirect 301/302/303/307/308, `Location` relativo, `HEAD` con ripiego su `GET` |
| URL | WHATWG URL / IDNA (UTS 46) tramite il crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violazioni, tema chiaro/scuro), schede WAI-ARIA APG con tastiera, `prefers-reduced-motion`, `forced-colors`, target ≥ 24 px, regioni `status`/`progressbar`; **CSP** con hash (niente `unsafe-inline` per gli script), `referrer: no-referrer`; i report esportati includono una CSP propria |
| Supply chain | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, release con somme SHA-256 e attestazione di provenienza (Sigstore/SLSA), nessuna action di terze parti nella pubblicazione |

Test web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Dopo ogni modifica a `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
