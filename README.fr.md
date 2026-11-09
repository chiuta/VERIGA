# VERIGA — vérificateur de liens (implémentation indépendante)

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Téléchargement:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Version : 1.1.0 · Dernière mise à jour : 30.10.2026 · Site de présentation : https://veriga.chiuta.ro

Une application Rust 64 bits pour Windows, macOS et Linux (x86-64 et ARM64) qui explore un site web
ou un dossier local et vérifie ses liens. Elle existe sous trois formes qui partagent un même modèle de résultats :

| Forme | Où | Ce qu'elle fait |
|---|---|---|
| `veriga` (CLI) | terminal, CI, conteneurs (binaire musl statique) | vérification complète, rapports, code de sortie |
| `veriga-gui` | bureau natif (egui) | tableau triable et filtrable, panneau de détails, pause/arrêt, export |
| `web/veriga.html` | tout navigateur, fichier unique, hors ligne | analyse d'un dossier local, d'un site en ligne (dans les limites de CORS), d'une liste d'URL |

Les sessions (`.veriga`, JSON) sont interchangeables entre les trois formes.

## À propos de l’étiquette « clean-room »

VERIGA est une **implémentation indépendante**, et non un projet clean-room documenté. Aucune procédure formelle (équipes d’analyse et d’implémentation séparées, spécifications relues) n’a été appliquée, et l’historique du dépôt commence par un seul gros commit. Le code a été écrit à partir du comportement général des vérificateurs de liens et de spécifications publiques (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Les sessions de développement de l’auteur ne montrent pas d’usage du code ou de l’exécutable d’un autre vérificateur. Le registre de provenance et les points ouverts sont dans [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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
