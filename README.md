# VERIGA — link checker (clean-room)

**English** · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Download:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Version: 1.1.0 · Last updated: 30.10.2026 · Presentation site: https://veriga.chiuta.ro

A 64-bit Rust application for Windows, macOS and Linux (x86-64 and ARM64) that crawls a website
or a local folder and checks its links. It comes in three forms that share one result model:

| Form | Where | What it does |
|---|---|---|
| `veriga` (CLI) | terminal, CI, containers (static musl binary) | full check, reports, exit code |
| `veriga-gui` | native desktop (egui) | sortable/filterable table, detail panel, pause/stop, export |
| `web/veriga.html` | any browser, single file, offline | local folder analysis, online site (within CORS limits), URL list |

Sessions (`.veriga`, JSON) are interchangeable between the three forms.

## What it checks

- **HTML links**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (style, icons, preload), `form[action]` (optional), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` and `@import`, from `.css` files as well as from `<style>` and `style` attributes.
- **Protocols**: HTTP/HTTPS (HEAD with fallback to GET), `file://` and local paths, FTP (authentication, `SIZE`/`CWD`),
  `mailto:` (syntax).
- **Redirects**: full chain with status codes, loop detection, configurable limit, a "redirects are errors" option.
- **`#fragment` anchors**: checked in the crawled pages (`id` and `name`).
- **Orphan files**: files in the site directory that no link points to.
- **Case mismatches** in local paths (works on Windows/macOS, breaks on Linux).
- **robots.txt** (groups, `Allow`/`Disallow` with `*` and `$`, `Crawl-delay`) and seeding from **sitemap.xml**.
- **Observations**: duplicate/missing titles, slow responses, statistics by MIME type.
- Character encodings (header, BOM, `<meta charset>`) — including old Windows-1250 pages.

## Main options

Concurrent threads and a per-host limit, timeout, retries (honouring `Retry-After`), maximum depth, URL limit,
external links yes/no, "stay in the directory", exclude / don't-follow / only-these (regex), extra internal hosts,
HTTP codes treated as OK (e.g. 999), User-Agent, proxy (HTTP/SOCKS), Basic authentication (internal hosts only),
headers and cookies, invalid certificates, delay between requests.

## Free, ethical proxies

Pick a preset with `--proxy-preset tor|tor-browser|warp` (list them with `--list-proxy-presets`), or in the GUI under "Free ethical proxy…". All presets are local SOCKS5 endpoints (`socks5h://127.0.0.1:…`), so names are resolved by the proxy, not by your system. Tor (volunteer, non-profit, open source) is the recommended choice; the check is capped at 4 threads and 2 connections per server and some sites block Tor exits, so expect false "broken links". Cloudflare WARP is free but a commercial service with a closed-source client. Lists of open public proxies are deliberately not included: they are often compromised or intercept traffic. Your own proxy is still available with `--proxy`.

## Reports

Self-contained HTML (opens anywhere), CSV, TSV, JSON, text, sitemap.xml, a site tree, a `.veriga` session.
**Re-check of broken links** from a saved session (`--recheck`, a button in the GUI/web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # local folder + orphans
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # broken links only
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Exit code `2` if broken links were found (useful in CI); `--no-fail` turns it off. `veriga --help` lists everything. Exit code `3` if the run was cancelled or truncated by `--max-urls` / `--max-per-pattern` (crawler traps, default 5000 pages per path template); `3` takes precedence over `2`. `veriga check URL` is equivalent to `veriga URL`.

## Building

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # CLI only
cargo test --workspace
```

The `.github/workflows/build.yml` workflow produces archives for: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, with GUI) and Linux x64/ARM64 (musl, static CLI). The `Dockerfile` builds a minimal image with the CLI.

## Video tutorials

Ten short tutorials, recorded on the real application, subtitled in 8 languages (ro, en, fr, it, es, pt, de, hu): first scan, filters, link details, options, export, Guardian (fixes; domains, map, trends), command line, the HTML edition and the portable edition. Watch them on https://veriga.chiuta.ro/#tutoriale or take the files from `assets/tutorials/`: each `.mp4` carries all 8 subtitle tracks, and the `.vtt` files sit alongside. No audio; the explanations are in the subtitles.

## Portable edition (PortableApps.com)

VERIGA also ships as a **PortableApps.com Format 3.9** package, `VERIGAPortable_<version>_Multilingual.zip` (release assets). Unzip it anywhere (USB drive, synced folder) and start `VERIGAPortable.exe`.

- Everything the app produces stays in `Data\` (Guardian state in `Data\.veriga`); nothing is written to the registry or `%APPDATA%`. Deleting `Data\` is safe: it is recreated.
- The launcher picks the x64 or the ARM64 build by the native architecture; `VERIGAPortable.exe session.veriga` opens a saved session (file association `.veriga`).
- The CLI is `App\VERIGA\veriga.exe`. The layout is checked in CI with `portableapps/validate.py`; the launcher source is `crates/veriga-launcher`.
- The official `.paf.exe` installer is built with the PortableApps.com Installer (a Windows tool); the folder is already in the required layout.

## Packaging for mobile and server

PWA (installable, offline): <https://veriga.chiuta.ro/app/> · Termux: `packaging/termux/install.sh` · Docker: `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · systemd timer, `.deb`/`.rpm`, Kubernetes CronJob and a **YunoHost** package: see [`packaging/`](packaging/README.md). A native Android APK is not provided.

## The single-file HTML edition

`web/veriga.html` runs 100% in the browser, without a server:

- **Local folder** (drag the site folder in, or "Choose folder"): full check of internal links, CSS, anchors, orphans,
  case mismatches, duplicate titles. There is no browser limitation here.
- **Online site**: a browser cannot read the HTTP status of other domains (CORS). Full results when the file is opened from the
  same site, or through your own CORS proxy (template `https://proxy/?u={url}`). Otherwise external links are marked
  *unverifiable* if the server answers, or *no connection* if it does not — never a false "OK".
- **URL list / pasted HTML**.
- It does not check FTP and does not read robots.txt (browser restrictions). Use the CLI/GUI for those.

## Guardian — repair, history, compliance

VERIGA also includes the features of a "link guardian" (inspired by the *Trade-Free Link Guardian* WordPress plugin), but as an independent tool that works on **files**: HTML, Markdown, WordPress export (WXR `.xml`, menus included) and sitemaps.

* **Deterministic repairs, verified live** — `scheme_typo` (`htp://`, `https//`), `unmangle` (doubled scheme, `%20`, trailing punctuation), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (consistency), `redirect` (301/308 only). A repair is applied automatically **only if the new URL answered successfully**; without verification it stays a "proposal".
* **Manual repairs** — URL replacement, "unlink" (keeps the text), Wayback Machine lookup (opt-in, never called automatically).
* **Safety** — a backup per run, a change journal, individual or per-run **undo** (byte for byte, `&amp;` included); only the bytes in the span are written.
* **401/403 = "worth a second look"**, not a broken link (an option, on by default).
* **Views and reports** — domains, per-page health map, "book" view, redirects (permanent/temporary), content drift (title/H1 against the previous run), new/resolved, trends from the history, a period **compliance report**, an impact badge (in the menu or on ≥ 3 pages). CSV exports neutralise formula cells.
* **404s from access logs** (Apache/nginx), with redirect suggestions and `.htaccess` export.
* **Scheduling** — `veriga wizard` (4 steps) writes `veriga-guardian.json`; `veriga watch --config … [--once]` runs periodically (or from cron/Task Scheduler), keeps the history, repairs the chosen rules automatically and notifies by webhook or `sendmail`.

```
veriga lint ./site                       # offline analysis
veriga fix ./site --session s.veriga     # propose + verify live (dry run)
veriga fix ./site --apply                # write, with a backup and a journal
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: the **🛡 Guardian** button. HTML edition: the **Guardian** section (changes are made in memory; you download the files or write them into a folder where the browser allows it; live verification is limited by CORS).

**What cannot be replicated from a WordPress plugin:** the database `old slug` lookup, the editor save hook, WP-Cron, multisite and `wp_mail`. In their place: WXR import, files on disk, `watch`/cron and `sendmail`/webhook. The feature list was reconstructed from the public description and from earlier conversations, not from the plugin's source code.

Support the project: https://www.buymeacoffee.com/echo.of.the.strings

## Known limits

- **Network guard:** the private-address check resolves the host, then the HTTP client resolves it again (a DNS-rebinding window exists); run VERIGA on untrusted sites from a network that cannot reach private ranges.
- **`veriga watch`** reads `./veriga-guardian.json` from the current directory by default, and that file can name a `sendmail` command and the sources to rewrite: do not run it inside a directory you do not trust.
- It does not send reports by e-mail (SMTP) and does not detect orphans over FTP — only in a local directory.
- Compressed `.xml.gz` sitemaps are not read.
- Pages that build their links with JavaScript are not rendered (there is no browser engine); the served HTML is analysed.
- The graphical interface was exercised under a virtual display (Linux/X11) only: crawl, filters, detail panel and Guardian. The Windows and macOS builds are produced by CI but not tested interactively.

## About "clean-room"

The code is written from scratch, based on the general behaviour of link checkers and on public specifications
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). It contains no code, resources, text or design from any other product and
was not written while looking at the code or the executable of another program.

License: MIT.

## Security and threat model

VERIGA processes untrusted content (web pages, exported files, configuration files). Default measures:

- **Anti-SSRF:** private/local targets (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) are blocked, including after a redirect. Explicit opt-out: `--allow-private` (CLI) / the GUI checkbox. *Limit:* DNS rebinding between the check and the connection is not fully closed.
- **Sensitive headers** (`Authorization`, `Cookie`, API keys) are sent only to the internal origin, never to other hosts or after an external redirect.
- **Injection filter for repairs:** a new URL (e.g. a hostile `Location`) containing quotes, `<`, `>`, spaces, backticks, backslashes or brackets in Markdown is refused (`unsafe_url_for` / `gUnsafeUrl`).
- **Safe writes:** a unique backup, atomic writes, cooperative locking, re-checking the content before applying (TOCTOU), keeping permissions/symlinks; stale proposals are dropped, not applied at wrong positions.
- **CSV export** protected against formulas (`=`, `+`, `-`, `@`).
- **Notifications:** `sendmail` must be an absolute path; do not use configuration files from untrusted sources (they could trigger execution of a chosen binary).
- **Known limits:** escaped HTML in WXR is not scanned (false negatives); the web edition is limited by CORS; `lint` exits with code 2 when it has findings.

## Standards and compliance (checked 29.10.2026)

| Area | Status |
|---|---|
| Rust | edition **2024**, MSRV **1.88** (core + CLI; their dependencies declare at most 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` and `cargo fmt --check` clean; `--locked` in CI |
| Dependencies | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` 0 vulnerabilities; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: merged groups, exact match on the product token, percent/UTF-8 normalisation, `*` and `$`, longest rule wins (Allow on a tie), 500 KiB cap, 4xx ⇒ allowed, 5xx/network ⇒ unavailable, the check continues (a dead site shows as broken, not “0 broken”); the start page is never blocked by robots.txt |
| HTTP | RFC 9110: `Retry-After` (seconds/date) on 429/503, redirects 301/302/303/307/308, relative `Location`, `HEAD` with fallback to `GET` |
| URL | WHATWG URL / IDNA (UTS 46) through the `url` crate (with `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violations, light/dark theme), WAI-ARIA APG tabs with keyboard, `prefers-reduced-motion`, `forced-colors`, targets ≥ 24 px, `status`/`progressbar` regions; **CSP** with hashes (no `unsafe-inline` for scripts), `referrer: no-referrer`; exported reports carry their own CSP |
| Supply chain | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, releases with SHA-256 sums and a provenance attestation (Sigstore/SLSA), no third-party actions in publishing |

Web tests: `cd web/tests && npm install && npx playwright install chromium && npm test`. After any change to `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
