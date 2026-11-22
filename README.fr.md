# VERIGA — vérificateur de liens (implémentation indépendante)

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Téléchargement:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.5.1

Version : 1.5.1 · Dernière mise à jour : 2026-11-22 · Site de présentation : https://veriga.chiuta.ro

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

`--honor-nofollow` — les liens marqués `rel="nofollow"` sont toujours vérifiés (statut HTTP), mais la page cible n'est pas explorée pour y chercher d'autres liens ; désactivé par défaut. Avec `--report-rel`, les jetons `rel` de chaque lien (insensibles à la casse, uniques) sont aussi indiqués dans le JSON (`rel`), le CSV/TSV (colonne `Rel`) et à côté des liens cassés dans le rapport texte ; sans cette option, ces sorties sont exactement celles de la v1.1.0.

`--check-schemes` — valide la syntaxe des adresses `tel:`, `geo:` et `data:` (RFC 3966, RFC 5870, RFC 2397 avec la tolérance base64 de Fetch) ; aucune requête réseau n'est faite. Les adresses invalides sont signalées comme « URL invalide » avec la raison ; les adresses `data:` de plus de 1 Mio sont signalées comme non vérifiées. Désactivé par défaut (ces schémas sont alors ignorés silencieusement).

`--check-link-names` — calcule le nom accessible de chaque `<a href>` (aria-labelledby, aria-label, contenu y compris le `alt` des images, puis `title` ; contenu masqué ignoré) et signale les liens sans nom ou dont le nom n'est que de la ponctuation (WCAG 2.4.4 / 4.1.2) dans les rapports texte et JSON (`a11y`). `--check-ambiguous-names` ajoute une note informative lorsque des noms identiques (insensibles à la casse) mènent à des adresses différentes sur une même page. Les deux sont désactivés par défaut.

`--check-link-header` — lit les champs d'en-tête HTTP `Link` (RFC 8288) des pages explorées et vérifie chaque cible déclarée comme n'importe quel autre lien (source = la page demandée, affichée « Link header : … » ; les jetons `rel` sont conservés). Les valeurs mal formées sont ignorées avec un avertissement (rapports texte/JSON, `warnings`) ; 100 liens au plus par réponse sont lus. Désactivé par défaut.

`--check-seo-links` — vérifie `rel="canonical"` (cible cassée, cible redirigée signalée à part, plusieurs canoniques différentes sur une même page ; une canonique qui pointe vers la page elle-même est correcte, y compris lue dans l'en-tête `Link` avec `--check-link-header`) et les valeurs `hreflang` sur `link rel=alternate` / `a` (syntaxe BCP 47 seulement, sans le registre IANA ; `x-default` est listé comme convention externe, pas comme une erreur). Les constats figurent dans les rapports texte et JSON (`seo`). Désactivé par défaut.

`--check-mixed-content` — sur les pages `https:`, signale les sous-ressources chargées en `http:` (contenu mixte) : images, audio et vidéo sont « optionnellement bloquables » ; scripts, feuilles de style, préchargements, iframes, objets, polices, `@import` CSS et actions de formulaire sont « bloquables ». Les simples liens de navigation ne sont pas des sous-ressources ; `localhost`, `127.0.0.0/8` et `[::1]` ne sont jamais signalés ; un en-tête `Content-Security-Policy: upgrade-insecure-requests` ajoute une note au lieu d'annuler le constat ; une sous-ressource `https:` qui redirige vers `http:` est aussi signalée. Les résultats figurent dans les rapports texte et JSON (`mixed`). Désactivé par défaut.

`--check-feeds` — lit les flux RSS 2.0 et Atom 1.0 trouvés via `<link rel="alternate" type="application/rss+xml|application/atom+xml">` (ou toute réponse XML rencontrée pendant l'exploration) et vérifie les liens qu'ils contiennent : `link`, `comments`, `enclosure@url`, `source@url` du canal et des éléments, `url`/`link` de l'image, un `guid` permanent, `link@href` d'Atom (`rel` absent = alternate), `content@src`, ainsi que les liens dans les descriptions HTML échappées/CDATA et dans le contenu Atom `html`/`xhtml`. Les adresses relatives suivent `xml:base`, puis l'adresse du flux. Un XML mal formé ou un document qui n'est pas un flux produit un avertissement et aucun lien. Désactivé par défaut.

`--check-sri` — vérifie l'attribut `integrity` de `<script src>` et de `<link rel=stylesheet|preload|modulepreload>` : syntaxe (base64 standard, longueur de condensat correcte), algorithmes reconnus (sha256/384/512 — le plus fort présent décide, n'importe laquelle de ses valeurs peut correspondre), condensat SHA-2 des octets téléchargés, et CORS pour les ressources d'origine différente (attribut `crossorigin` plus un `Access-Control-Allow-Origin` correspondant). Verdicts : correspond / ne correspond pas / syntaxe invalide / aucun algorithme reconnu / pas de CORS / non vérifié ; `integrity` sur d'autres éléments est noté comme non applicable. Les ressources sont téléchargées en entier (GET) uniquement lorsqu'un lien demande la vérification. Les résultats figurent dans les rapports texte et JSON (`sri`). Désactivé par défaut.

`--check-cert-expiry` / `--cert-warn-days N` (14 par défaut) — lit le certificat TLS feuille que présente chaque hôte:port `https` pendant la vérification et signale, une fois par hôte:port avec le nombre de liens vérifiés à cet endroit : expiré, pas encore valide, expire dans N jours (seuil ; `0` = expiré seulement), illisible, ou un nom qui ne correspond à aucune entrée SAN. Fonctionne aussi avec `--insecure`. Un proxy qui intercepte le TLS présente son propre certificat ; une horloge système erronée ne peut pas être détectée. Les résultats figurent dans les rapports texte et JSON (`certs`). Désactivé par défaut.

## Proxys gratuits et éthiques

Choisissez un préréglage avec `--proxy-preset tor|tor-browser|warp` (liste : `--list-proxy-presets`) ou, dans l'interface, « Proxy gratuit éthique… ». Tous sont des points SOCKS5 locaux (`socks5h://127.0.0.1:…`) : les noms sont résolus par le proxy, pas par votre système. Tor (bénévole, à but non lucratif, open source) est le choix recommandé ; la vérification est limitée à 4 threads et 2 connexions par serveur, et certains sites bloquent les sorties Tor : de faux « liens cassés » sont possibles. Cloudflare WARP est gratuit mais reste un service commercial à client fermé. Les listes de proxys publics ouverts sont volontairement exclues : ils sont souvent compromis ou interceptent le trafic. Votre propre proxy reste disponible avec `--proxy`.

## Rapports

HTML autonome (s'ouvre partout), CSV, TSV, JSON, texte, sitemap.xml, arborescence du site, session `.veriga`.
**Revérification des liens cassés** à partir d'une session enregistrée (`--recheck`, un bouton dans la GUI/web).

**Adresses alias** (`--report-aliases`) : les différentes écritures d'une même adresse (casse de l'hôte, port par défaut, caractères encodés en pourcentage sans nécessité, segments de point) sont listées comme groupes informatifs dans les rapports texte et JSON (`aliases`) ; la casse du chemin/de la requête, l'ordre des paramètres, la barre oblique finale et `www.` ne sont jamais regroupés. Le suivi est plafonné en mémoire (100 000 adresses, 10 écritures par adresse, 20 pages par écriture). `--canonical-urls` fusionne en plus ces écritures en une seule vérification (et indique l'adresse normalisée) ; par défaut, les URL sont vérifiées et rapportées exactement telles qu'écrites, comme dans la v1.1.0. Autre option désactivée par défaut : `--xml-encoding` (respecter `<?xml encoding=…?>` quand ni en-tête ni meta charset n'est donné).

**SARIF 2.1.0** (`-o report.sarif`) : les liens cassés deviennent `error`, les redirections / 401-403 / réponses lentes `warning`, les éléments ignorés ou orphelins `note` ; les bons liens ne sont pas émis. L'emplacement est la page (ou le fichier) qui contient le lien, sous forme d'URI ; `region.startLine` n'est écrit que si une ligne est connue (les résultats d'exploration ne portent pas encore de ligne, il est donc omis). La sortie est déterministe et écrite de manière atomique.

**JUnit XML** (`-o report.junit`) : un `testsuite` par page source, un `testcase` par lien vérifié (`name` = cible, `classname` = page source). Les liens cassés sont `failure` ; les délais dépassés / absences de connexion sont `error` ; les éléments ignorés ou à revoir seulement sont `skipped` ; les caractères interdits en XML 1.0 deviennent U+FFFD. Les fichiers orphelins ne sont pas des liens et sont exclus.

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

- **Garde réseau :** la vérification des adresses privées résout l'hôte, puis le client HTTP le résout à nouveau (une fenêtre de DNS rebinding existe) ; exécutez VERIGA sur des sites non fiables depuis un réseau qui ne peut pas atteindre les plages privées.
- **`veriga watch`** lit par défaut `./veriga-guardian.json` dans le répertoire courant, et ce fichier peut désigner une commande `sendmail` et les sources à réécrire : ne l'exécutez pas dans un répertoire auquel vous ne faites pas confiance.
- Il n'envoie pas de rapports par e-mail (SMTP) et ne détecte pas les orphelins via FTP — uniquement dans un répertoire local.
- Les sitemaps compressés `.xml.gz` sont lus (gzip est détecté d'après les premiers octets, non l'extension) ; la taille décompressée est plafonnée à 50 Mo et un sitemap est lu jusqu'à ses 50 000 premières URL.
- Les pages qui construisent leurs liens en JavaScript ne sont pas interprétées (il n'y a pas de moteur de navigateur) ; c'est le HTML servi qui est analysé.
- L'interface graphique n'a été éprouvée que sous un affichage virtuel (Linux/X11) : exploration, filtres, panneau de détails et Guardian. Les versions Windows et macOS sont produites par la CI mais n'ont pas été testées de façon interactive.

## À propos de « clean-room »

VERIGA est une **implémentation indépendante**, et non un projet clean-room documenté. Aucune procédure formelle (équipes d’analyse et d’implémentation séparées, spécifications relues) n’a été appliquée, et l’historique du dépôt commence par un seul gros commit. Le code a été écrit à partir du comportement général des vérificateurs de liens et de spécifications publiques (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Le fait qu’aucun code ni exécutable d’un autre vérificateur n’ait été utilisé est une déclaration de l’auteur, non une preuve : les sessions de développement n’ont été examinées qu’en partie, et l’absence d’un tel usage dans les parties non examinées ne peut pas être démontrée. Le registre de provenance et les points ouverts sont dans
[docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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
| Dépendances | reqwest 0.13 (rustls + vérificateur de la plateforme, HTTP/2, gzip/brotli/zstd), scraper 0.29, jiff 1.0, eframe/egui 0.37, rfd 0.17 ; `cargo audit` 0 vulnérabilité ; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309** : groupes fusionnés, correspondance exacte sur le jeton de produit, normalisation pourcentage/UTF-8, `*` et `$`, la règle la plus longue l'emporte (Allow en cas d'égalité), plafond de 500 KiB, 4xx ⇒ autorisé, 5xx/réseau ⇒ indisponible, la vérification continue (un site mort apparaît cassé, pas « 0 cassé ») ; la page de départ n'est jamais bloquée par robots.txt |
| HTTP | RFC 9110 : `Retry-After` (secondes/date) sur 429/503, redirections 301/302/303/307/308, `Location` relatif, `HEAD` avec repli sur `GET` |
| URL | WHATWG URL / IDNA (UTS 46) via la crate `url` (avec `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14 : 0 violation, thème clair/sombre), onglets WAI-ARIA APG au clavier, `prefers-reduced-motion`, `forced-colors`, cibles ≥ 24 px, régions `status`/`progressbar` ; **CSP** avec hachages (pas de `unsafe-inline` pour les scripts), `referrer: no-referrer` ; les rapports exportés embarquent leur propre CSP |
| Chaîne d'approvisionnement | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, publications avec sommes SHA-256 et attestation de provenance (Sigstore/SLSA), aucune action tierce dans la publication |

Tests web : `cd web/tests && npm install && npx playwright install chromium && npm test`. Après toute modification de `veriga.html` : `python3 web/stamp-csp.py web/veriga.html`.
