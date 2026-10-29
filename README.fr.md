# VERIGA — vérificateur de liens (clean-room)

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Téléchargement:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.0.2

Version : 1.0.2 · Dernière mise à jour : 29.10.2026 · Site de présentation : https://veriga.chiuta.ro

Une application Rust 64 bits pour Windows, macOS et Linux (x86-64 et ARM64) qui explore un site web
ou un dossier local et vérifie ses liens. Elle existe sous trois formes qui partagent un même modèle de résultats :

| Forme | Où | Ce qu'elle fait |
|---|---|---|
| `veriga` (CLI) | terminal, CI, conteneurs (binaire musl statique) | vérification complète, rapports, code de sortie |
| `veriga-gui` | bureau natif (egui) | tableau triable et filtrable, panneau de détails, pause/arrêt, export |
| `web/veriga.html` | tout navigateur, fichier unique, hors ligne | analyse d'un dossier local, d'un site en ligne (dans les limites de CORS), d'une liste d'URL |

Les sessions (`.veriga`, JSON) sont interchangeables entre les trois formes.

## Ce qui est vérifié

- **Liens HTML** : `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (style, icônes, preload), `form[action]` (facultatif), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS** : `url(...)` et `@import`, dans les fichiers `.css` comme dans les balises `<style>` et les attributs `style`.
- **Protocoles** : HTTP/HTTPS (HEAD avec repli sur GET), `file://` et chemins locaux, FTP (authentification, `SIZE`/`CWD`),
  `mailto:` (syntaxe).
- **Redirections** : chaîne complète avec codes de statut, détection de boucles, limite configurable, option « les redirections sont des erreurs ».
- **Ancres `#fragment`** : vérifiées dans les pages explorées (`id` et `name`).
- **Fichiers orphelins** : fichiers du répertoire du site vers lesquels aucun lien ne pointe.
- **Différences de casse** dans les chemins locaux (fonctionne sous Windows/macOS, échoue sous Linux).
- **robots.txt** (groupes, `Allow`/`Disallow` avec `*` et `$`, `Crawl-delay`) et amorçage depuis **sitemap.xml**.
- **Observations** : titres en double ou manquants, réponses lentes, statistiques par type MIME.
- Encodages de caractères (en-tête, BOM, `<meta charset>`), y compris les anciennes pages en Windows-1250.

## Options principales

Threads concurrents et limite par hôte, délai d'attente, nouvelles tentatives (en respectant `Retry-After`), profondeur maximale, limite d'URL,
liens externes oui/non, « rester dans le répertoire », exclure / ne pas suivre / seulement ceux-ci (regex), hôtes internes supplémentaires,
codes HTTP considérés comme OK (p. ex. 999), User-Agent, proxy (HTTP/SOCKS), authentification Basic (hôtes internes uniquement),
en-têtes et cookies, certificats invalides, délai entre les requêtes.

## Proxys gratuits et éthiques

Choisissez un préréglage avec `--proxy-preset tor|tor-browser|warp` (liste : `--list-proxy-presets`) ou, dans l'interface, « Proxy gratuit éthique… ». Tous sont des points SOCKS5 locaux (`socks5h://127.0.0.1:…`) : les noms sont résolus par le proxy, pas par votre système. Tor (bénévole, à but non lucratif, open source) est le choix recommandé ; la vérification est limitée à 4 threads et 2 connexions par serveur, et certains sites bloquent les sorties Tor : de faux « liens cassés » sont possibles. Cloudflare WARP est gratuit mais reste un service commercial à client fermé. Les listes de proxys publics ouverts sont volontairement exclues : ils sont souvent compromis ou interceptent le trafic. Votre propre proxy reste disponible avec `--proxy`.

## Rapports

HTML autonome (s'ouvre partout), CSV, TSV, JSON, texte, sitemap.xml, arborescence du site, session `.veriga`.
**Revérification des liens cassés** à partir d'une session enregistrée (`--recheck`, un bouton dans la GUI/web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # dossier local + orphelins
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # liens cassés uniquement
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Code de sortie `2` si des liens cassés ont été trouvés (utile en CI) ; `--no-fail` le désactive. `veriga --help` liste tout. Code de sortie `3` si l'exécution a été interrompue ou tronquée par `--max-urls` / `--max-per-pattern` (pièges d'exploration, 5000 pages par modèle de chemin par défaut) ; `3` l'emporte sur `2`. `veriga check URL` équivaut à `veriga URL`.

## Compilation

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # CLI uniquement
cargo test --workspace
```

Le workflow `.github/workflows/build.yml` produit des archives pour : Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, avec GUI) et Linux x64/ARM64 (musl, CLI statique). Le `Dockerfile` construit une image minimale avec la CLI.

## Tutoriels vidéo

Dix courts tutoriels enregistrés sur la vraie application et sous-titrés en 8 langues (ro, en, fr, it, es, pt, de, hu) : première analyse, filtres, détail d’un lien, options, export, Guardian (corrections ; domaines, carte, tendances), ligne de commande, édition HTML et édition portable. Voir https://veriga.chiuta.ro/#tutoriale ou `assets/tutorials/` : chaque `.mp4` contient les 8 pistes de sous-titres, les fichiers `.vtt` sont à côté. Sans son ; les explications sont dans les sous-titres.

## Édition portable (PortableApps.com)

VERIGA existe aussi sous forme de paquet **PortableApps.com Format 3.9**, `VERIGAPortable_<version>_Multilingual.zip` (fichiers de la release). Décompressez-le n'importe où (clé USB, dossier synchronisé) et lancez `VERIGAPortable.exe`.

- Tout ce que produit l'application reste dans `Data\` (état de Guardian dans `Data\.veriga`) ; rien n'est écrit dans le registre ni dans `%APPDATA%`. Supprimer `Data\` est sans risque : il est recréé.
- Le lanceur choisit la version x64 ou ARM64 selon l'architecture native ; `VERIGAPortable.exe session.veriga` ouvre une session enregistrée (association de fichiers `.veriga`).
- La CLI est `App\VERIGA\veriga.exe`. L'arborescence est vérifiée en CI avec `portableapps/validate.py` ; le code du lanceur est `crates/veriga-launcher`.
- L'installateur officiel `.paf.exe` se construit avec PortableApps.com Installer (outil Windows) ; le dossier a déjà la structure requise.

## Paquets pour mobile et serveur

PWA (installable, hors ligne) : <https://veriga.chiuta.ro/app/> · Termux : `packaging/termux/install.sh` · Docker : `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · minuterie systemd, `.deb`/`.rpm`, CronJob Kubernetes et paquet **YunoHost** : voir [`packaging/`](packaging/README.md). Aucun APK Android natif n'est fourni.

## L'édition HTML à fichier unique

`web/veriga.html` fonctionne à 100 % dans le navigateur, sans serveur :

- **Dossier local** (glissez le dossier du site, ou « Choisir un dossier ») : vérification complète des liens internes, du CSS, des ancres, des orphelins,
  des différences de casse, des titres en double. Aucune limitation du navigateur ici.
- **Site en ligne** : un navigateur ne peut pas lire le statut HTTP d'autres domaines (CORS). Résultats complets si le fichier est ouvert depuis le
  même site, ou via votre propre proxy CORS (modèle `https://proxy/?u={url}`). Sinon, les liens externes sont marqués
  *non vérifiables* si le serveur répond, ou *pas de connexion* s'il ne répond pas — jamais un faux « OK ».
- **Liste d'URL / HTML collé**.
- Elle ne vérifie pas le FTP et ne lit pas robots.txt (restrictions du navigateur). Utilisez la CLI/GUI pour cela.

## Guardian — réparation, historique, conformité

VERIGA reprend aussi les fonctions d'un « gardien de liens » (inspiré du plugin WordPress *Trade-Free Link Guardian*), mais sous la forme d'un outil indépendant qui travaille sur des **fichiers** : HTML, Markdown, export WordPress (WXR `.xml`, menus inclus) et sitemaps.

* **Réparations déterministes, vérifiées en direct** — `scheme_typo` (`htp://`, `https//`), `unmangle` (schéma doublé, `%20`, ponctuation finale), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (cohérence), `redirect` (301/308 uniquement). Une réparation n'est appliquée automatiquement que **si la nouvelle URL a répondu avec succès** ; sans vérification, elle reste une « proposition ».
* **Réparations manuelles** — remplacement d'URL, « supprimer le lien » (conserve le texte), recherche dans la Wayback Machine (sur demande, jamais appelée automatiquement).
* **Sécurité** — une sauvegarde par exécution, un journal des modifications, **annulation** individuelle ou par exécution (octet pour octet, `&amp;` compris) ; seuls les octets de la plage concernée sont écrits.
* **401/403 = « à revoir »**, et non un lien cassé (option activée par défaut).
* **Vues et rapports** — domaines, carte de santé par page, vue « livre », redirections (permanentes/temporaires), dérive du contenu (titre/H1 par rapport à l'exécution précédente), nouveaux/résolus, tendances issues de l'historique, **rapport de conformité** sur une période, badge d'impact (dans le menu ou sur ≥ 3 pages). Les exports CSV neutralisent les cellules à formule.
* **404 issus des journaux d'accès** (Apache/nginx), avec suggestions de redirections et export `.htaccess`.
* **Planification** — `veriga wizard` (4 étapes) écrit `veriga-guardian.json` ; `veriga watch --config … [--once]` s'exécute périodiquement (ou depuis cron/Task Scheduler), conserve l'historique, répare automatiquement les règles choisies et notifie par webhook ou `sendmail`.

```
veriga lint ./site                       # analyse hors ligne
veriga fix ./site --session s.veriga     # proposer + vérifier en direct (simulation)
veriga fix ./site --apply                # écrire, avec sauvegarde et journal
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI : le bouton **🛡 Guardian**. Édition HTML : la section **Guardian** (les modifications se font en mémoire ; vous téléchargez les fichiers ou les écrivez dans un dossier là où le navigateur le permet ; la vérification en direct est limitée par CORS).

**Ce qui ne peut pas être reproduit depuis un plugin WordPress :** la recherche d'anciens slugs en base de données, le hook de sauvegarde de l'éditeur, WP-Cron, le multisite et `wp_mail`. À la place : import WXR, fichiers sur disque, `watch`/cron et `sendmail`/webhook. La liste des fonctions a été reconstituée à partir de la description publique et de conversations antérieures, et non à partir du code source du plugin.

Soutenir le projet : https://www.buymeacoffee.com/echo.of.the.strings

## Limites connues

- Il n'envoie pas de rapports par e-mail (SMTP) et ne détecte pas les orphelins via FTP — uniquement dans un répertoire local.
- Les sitemaps compressés `.xml.gz` ne sont pas lus.
- Les pages qui construisent leurs liens en JavaScript ne sont pas interprétées (il n'y a pas de moteur de navigateur) ; c'est le HTML servi qui est analysé.
- L'interface graphique n'a été éprouvée que sous un affichage virtuel (Linux/X11) : exploration, filtres, panneau de détails et Guardian. Les versions Windows et macOS sont produites par la CI mais n'ont pas été testées de façon interactive.

## À propos de « clean-room »

Le code est écrit de zéro, à partir du comportement général des vérificateurs de liens et de spécifications publiques
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Il ne contient ni code, ni ressources, ni texte, ni design d'aucun autre produit et
n'a pas été écrit en consultant le code ou l'exécutable d'un autre programme.

Licence : MIT.

## Sécurité et modèle de menaces

VERIGA traite du contenu non fiable (pages web, fichiers exportés, fichiers de configuration). Mesures par défaut :

- **Anti-SSRF :** les cibles privées/locales (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) sont bloquées, y compris après une redirection. Désactivation explicite : `--allow-private` (CLI) / la case à cocher de la GUI. *Limite :* le DNS rebinding entre la vérification et la connexion n'est pas totalement fermé.
- **En-têtes sensibles** (`Authorization`, `Cookie`, clés d'API) : envoyés uniquement à l'origine interne, jamais à d'autres hôtes ni après une redirection externe.
- **Filtre d'injection pour les réparations :** une nouvelle URL (p. ex. un `Location` hostile) contenant des guillemets, `<`, `>`, des espaces, des accents graves, des barres obliques inverses ou des crochets en Markdown est refusée (`unsafe_url_for` / `gUnsafeUrl`).
- **Écritures sûres :** sauvegarde unique, écritures atomiques, verrouillage coopératif, revérification du contenu avant application (TOCTOU), conservation des permissions et des liens symboliques ; les propositions périmées sont abandonnées, pas appliquées à de mauvaises positions.
- **Export CSV** protégé contre les formules (`=`, `+`, `-`, `@`).
- **Notifications :** `sendmail` doit être un chemin absolu ; n'utilisez pas de fichiers de configuration de source non fiable (ils pourraient déclencher l'exécution d'un binaire choisi).
- **Limites connues :** le HTML échappé dans les WXR n'est pas analysé (faux négatifs) ; l'édition web est limitée par CORS ; `lint` se termine avec le code 2 lorsqu'il trouve des problèmes.

## Normes et conformité (vérifié le 29.10.2026)

| Domaine | État |
|---|---|
| Rust | édition **2024**, MSRV **1.88** (cœur + CLI ; leurs dépendances déclarent au plus 1.88), GUI **1.95** (egui 0.37) ; `cargo clippy -- -D warnings` et `cargo fmt --check` sans avertissement ; `--locked` en CI |
| Dépendances | reqwest 0.13 (rustls + vérificateur de la plateforme, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17 ; `cargo audit` 0 vulnérabilité ; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309** : groupes fusionnés, correspondance exacte sur le jeton de produit, normalisation pourcentage/UTF-8, `*` et `$`, la règle la plus longue l'emporte (Allow en cas d'égalité), plafond de 500 KiB, 4xx ⇒ autorisé, 5xx/réseau ⇒ indisponible, la vérification continue (un site mort apparaît cassé, pas « 0 cassé ») ; la page de départ n'est jamais bloquée par robots.txt |
| HTTP | RFC 9110 : `Retry-After` (secondes/date) sur 429/503, redirections 301/302/303/307/308, `Location` relatif, `HEAD` avec repli sur `GET` |
| URL | WHATWG URL / IDNA (UTS 46) via la crate `url` (avec `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14 : 0 violation, thème clair/sombre), onglets WAI-ARIA APG au clavier, `prefers-reduced-motion`, `forced-colors`, cibles ≥ 24 px, régions `status`/`progressbar` ; **CSP** avec hachages (pas de `unsafe-inline` pour les scripts), `referrer: no-referrer` ; les rapports exportés embarquent leur propre CSP |
| Chaîne d'approvisionnement | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, publications avec sommes SHA-256 et attestation de provenance (Sigstore/SLSA), aucune action tierce dans la publication |

Tests web : `cd web/tests && npm install && npx playwright install chromium && npm test`. Après toute modification de `veriga.html` : `python3 web/stamp-csp.py web/veriga.html`.
