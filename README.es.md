# VERIGA — verificador de enlaces (implementación independiente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Descarga:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Versión: 1.1.0 · Última actualización: 30.10.2026 · Sitio de presentación: https://veriga.chiuta.ro

Aplicación Rust de 64 bits para Windows, macOS y Linux (x86-64 y ARM64) que rastrea un sitio web
o una carpeta local y comprueba sus enlaces. Se presenta en tres formas que comparten un mismo modelo de resultados:

| Forma | Dónde | Qué hace |
|---|---|---|
| `veriga` (CLI) | terminal, CI, contenedores (binario musl estático) | comprobación completa, informes, código de salida |
| `veriga-gui` | escritorio nativo (egui) | tabla ordenable y filtrable, panel de detalle, pausa/parada, exportación |
| `web/veriga.html` | cualquier navegador, archivo único, sin conexión | análisis de carpeta local, sitio en línea (dentro de los límites de CORS), lista de URL |

Las sesiones (`.veriga`, JSON) son intercambiables entre las tres formas.

## Sobre «clean-room»

VERIGA es una **implementación independiente**, no un proyecto clean-room documentado. No se aplicó un procedimiento formal (equipos separados de análisis e implementación, especificaciones revisadas) y el historial del repositorio empieza con un único commit grande. El código se escribió a partir del comportamiento general de los verificadores de enlaces y de especificaciones públicas (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Las sesiones de desarrollo del autor no muestran el uso del código o del ejecutable de otro verificador. El registro de procedencia y los puntos abiertos están en [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

## Seguridad y modelo de amenazas

VERIGA procesa contenido no confiable (páginas web, archivos exportados, archivos de configuración). Medidas por defecto:

- **Anti-SSRF:** los destinos privados/locales (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) se bloquean, también tras una redirección. Desactivación explícita: `--allow-private` (CLI) / la casilla de la GUI. *Límite:* el DNS rebinding entre la comprobación y la conexión no queda totalmente cerrado.
- **Cabeceras sensibles** (`Authorization`, `Cookie`, claves de API) se envían solo al origen interno, nunca a otros hosts ni tras una redirección externa.
- **Filtro de inyección en reparaciones:** se rechaza una URL nueva (p. ej. un `Location` hostil) que contenga comillas, `<`, `>`, espacios, comillas invertidas, barras invertidas o corchetes en Markdown (`unsafe_url_for` / `gUnsafeUrl`).
- **Escrituras seguras:** copia de seguridad única, escrituras atómicas, bloqueo cooperativo, nueva comprobación del contenido antes de aplicar (TOCTOU), conservación de permisos y enlaces simbólicos; las propuestas obsoletas se descartan, no se aplican en posiciones erróneas.
- **Exportación CSV** protegida contra fórmulas (`=`, `+`, `-`, `@`).
- **Notificaciones:** `sendmail` debe ser una ruta absoluta; no use archivos de configuración de fuentes no confiables (podrían provocar la ejecución de un binario elegido por el atacante).
- **Límites conocidos:** el HTML escapado en WXR no se analiza (falsos negativos); la edición web está limitada por CORS; `lint` termina con código 2 cuando hay hallazgos.

## Estándares y cumplimiento (comprobado el 29.10.2026)

| Área | Estado |
|---|---|
| Rust | edición **2024**, MSRV **1.88** (núcleo + CLI; sus dependencias declaran como máximo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` y `cargo fmt --check` sin avisos; `--locked` en CI |
| Dependencias | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` con 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos fusionados, coincidencia exacta con el product token, normalización percent/UTF-8, `*` y `$`, prevalece la regla más larga (Allow en caso de empate), límite de 500 KiB, 4xx ⇒ permitido, 5xx/red ⇒ no disponible, la verificación continúa (un sitio caído aparece roto, no «0 rotos»); la página de inicio nunca la bloquea robots.txt |
| HTTP | RFC 9110: `Retry-After` (segundos/fecha) en 429/503, redirecciones 301/302/303/307/308, `Location` relativo, `HEAD` con respaldo a `GET` |
| URL | WHATWG URL / IDNA (UTS 46) mediante el crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (índice + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 infracciones, tema claro/oscuro), pestañas WAI-ARIA APG con teclado, `prefers-reduced-motion`, `forced-colors`, objetivos ≥ 24 px, regiones `status`/`progressbar`; **CSP** con hashes (sin `unsafe-inline` en scripts), `referrer: no-referrer`; los informes exportados llevan su propia CSP |
| Cadena de suministro | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, versiones con sumas SHA-256 y atestación de procedencia (Sigstore/SLSA), sin acciones de terceros en la publicación |

Pruebas web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Tras cualquier cambio en `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
