# VERIGA — verificador de enlaces (clean-room)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versión: 1.0.0 · Última actualización: 27.10.2026 · Sitio de presentación: https://veriga.chiuta.ro

Aplicación Rust de 64 bits para Windows, macOS y Linux (x86-64 y ARM64) que rastrea un sitio web
o una carpeta local y comprueba sus enlaces. Se presenta en tres formas que comparten un mismo modelo de resultados:

| Forma | Dónde | Qué hace |
|---|---|---|
| `veriga` (CLI) | terminal, CI, contenedores (binario musl estático) | comprobación completa, informes, código de salida |
| `veriga-gui` | escritorio nativo (egui) | tabla ordenable y filtrable, panel de detalle, pausa/parada, exportación |
| `web/veriga.html` | cualquier navegador, archivo único, sin conexión | análisis de carpeta local, sitio en línea (dentro de los límites de CORS), lista de URL |

Las sesiones (`.veriga`, JSON) son intercambiables entre las tres formas.

## Qué comprueba

- **Enlaces HTML**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (estilos, iconos, preload), `form[action]` (opcional), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` y `@import`, tanto en archivos `.css` como en `<style>` y atributos `style`.
- **Protocolos**: HTTP/HTTPS (HEAD con respaldo a GET), `file://` y rutas locales, FTP (autenticación, `SIZE`/`CWD`),
  `mailto:` (sintaxis).
- **Redirecciones**: cadena completa con códigos de estado, detección de bucles, límite configurable y opción «las redirecciones son errores».
- **Anclas `#fragment`**: se comprueban en las páginas rastreadas (`id` y `name`).
- **Archivos huérfanos**: archivos del directorio del sitio a los que no apunta ningún enlace.
- **Diferencias de mayúsculas** en rutas locales (funciona en Windows/macOS, falla en Linux).
- **robots.txt** (grupos, `Allow`/`Disallow` con `*` y `$`, `Crawl-delay`) y punto de partida desde **sitemap.xml**.
- **Observaciones**: títulos duplicados o ausentes, respuestas lentas, estadísticas por tipo MIME.
- Codificaciones de caracteres (cabecera, BOM, `<meta charset>`), incluidas páginas antiguas en Windows-1250.

## Opciones principales

Hilos concurrentes y límite por host, tiempo de espera, reintentos (respetando `Retry-After`), profundidad máxima, límite de URL,
enlaces externos sí/no, «permanecer en el directorio», excluir / no seguir / solo estos (regex), hosts internos adicionales,
códigos HTTP tratados como correctos (p. ej. 999), User-Agent, proxy (HTTP/SOCKS), autenticación Basic (solo hosts internos),
cabeceras y cookies, certificados no válidos, retardo entre solicitudes.

## Informes

HTML autónomo (se abre en cualquier parte), CSV, TSV, JSON, texto, sitemap.xml, un árbol del sitio y una sesión `.veriga`.
**Nueva comprobación de enlaces rotos** a partir de una sesión guardada (`--recheck`, un botón en la GUI/web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # carpeta local + huérfanos
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # solo enlaces rotos
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Código de salida `2` si se encontraron enlaces rotos (útil en CI); `--no-fail` lo desactiva. `veriga --help` lo muestra todo. Código de salida `3` si la ejecución se canceló o se truncó por `--max-urls`.

## Compilación

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # solo CLI
cargo test --workspace
```

El flujo de trabajo `.github/workflows/build.yml` genera paquetes para: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, con GUI) y Linux x64/ARM64 (musl, CLI estático). El `Dockerfile` construye una imagen mínima con la CLI.

## Tutoriales en vídeo

Diez tutoriales breves grabados con la aplicación real y subtitulados en 8 idiomas (ro, en, fr, it, es, pt, de, hu): primer análisis, filtros, detalle de un enlace, opciones, exportación, Guardian (reparaciones; dominios, mapa, tendencias), línea de comandos, edición HTML y edición portátil. En https://veriga.chiuta.ro/#tutoriale o en `assets/tutorials/`: cada `.mp4` lleva las 8 pistas de subtítulos y los `.vtt` van al lado. Sin audio; las explicaciones están en los subtítulos.

## Edición portátil (PortableApps.com)

VERIGA también se distribuye como paquete **PortableApps.com Format 3.9**, `VERIGAPortable_<versión>_Multilingual.zip` (archivos de la release). Descomprímelo donde quieras (memoria USB, carpeta sincronizada) y ejecuta `VERIGAPortable.exe`.

- Todo lo que genera la aplicación permanece en `Data\` (estado de Guardian en `Data\.veriga`); no se escribe nada en el registro ni en `%APPDATA%`. Borrar `Data\` es seguro: se recrea.
- El lanzador elige la versión x64 o ARM64 según la arquitectura nativa; `VERIGAPortable.exe sesion.veriga` abre una sesión guardada (asociación de archivos `.veriga`).
- La CLI es `App\VERIGA\veriga.exe`. La estructura se comprueba en CI con `portableapps/validate.py`; el código del lanzador es `crates/veriga-launcher`.
- El instalador oficial `.paf.exe` se construye con PortableApps.com Installer (herramienta para Windows); la carpeta ya tiene la estructura requerida.

## La edición HTML de archivo único

`web/veriga.html` se ejecuta al 100 % en el navegador, sin servidor:

- **Carpeta local** (arrastre la carpeta del sitio o use «Elegir carpeta»): comprobación completa de enlaces internos, CSS, anclas, huérfanos,
  diferencias de mayúsculas y títulos duplicados. Aquí el navegador no impone ninguna limitación.
- **Sitio en línea**: un navegador no puede leer el estado HTTP de otros dominios (CORS). Los resultados son completos si el archivo se abre desde el
  mismo sitio o a través de su propio proxy CORS (plantilla `https://proxy/?u={url}`). En caso contrario, los enlaces externos se marcan
  como *no verificables* si el servidor responde, o *sin conexión* si no lo hace; nunca un falso «OK».
- **Lista de URL / HTML pegado**.
- No comprueba FTP ni lee robots.txt (restricciones del navegador). Para eso, use la CLI/GUI.

## Guardian — reparación, historial, cumplimiento

VERIGA incluye además las funciones de un «guardián de enlaces» (inspirado en el plugin *Trade-Free Link Guardian* de WordPress), pero como herramienta independiente que trabaja sobre **archivos**: HTML, Markdown, exportación de WordPress (WXR `.xml`, menús incluidos) y sitemaps.

* **Reparaciones deterministas, verificadas en vivo** — `scheme_typo` (`htp://`, `https//`), `unmangle` (esquema duplicado, `%20`, puntuación final), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (coherencia), `redirect` (solo 301/308). Una reparación se aplica automáticamente **solo si la nueva URL respondió correctamente**; sin verificación queda como «propuesta».
* **Reparaciones manuales** — sustitución de URL, «desenlazar» (conserva el texto) y consulta en Wayback Machine (opcional, nunca se invoca automáticamente).
* **Seguridad** — una copia de seguridad por ejecución, un diario de cambios y **deshacer** individual o por ejecución (byte a byte, `&amp;` incluido); solo se escriben los bytes del tramo afectado.
* **401/403 = «merece una segunda revisión»**, no un enlace roto (opción activada por defecto).
* **Vistas e informes** — dominios, mapa de salud por página, vista «libro», redirecciones (permanentes/temporales), deriva de contenido (título/H1 frente a la ejecución anterior), nuevos/resueltos, tendencias del historial, un **informe de cumplimiento** por periodo y una insignia de impacto (en el menú o en ≥ 3 páginas). Las exportaciones CSV neutralizan las celdas con fórmulas.
* **404 de los registros de acceso** (Apache/nginx), con sugerencias de redirección y exportación a `.htaccess`.
* **Programación** — `veriga wizard` (4 pasos) escribe `veriga-guardian.json`; `veriga watch --config … [--once]` se ejecuta periódicamente (o desde cron/Task Scheduler), conserva el historial, repara automáticamente las reglas elegidas y notifica por webhook o `sendmail`.

```
veriga lint ./site                       # análisis sin conexión
veriga fix ./site --session s.veriga     # proponer + verificar en vivo (simulacro)
veriga fix ./site --apply                # escribir, con copia de seguridad y diario
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: el botón **🛡 Guardian**. Edición HTML: la sección **Guardian** (los cambios se hacen en memoria; usted descarga los archivos o los escribe en una carpeta si el navegador lo permite; la verificación en vivo está limitada por CORS).

**Lo que no se puede replicar de un plugin de WordPress:** la búsqueda de `old slug` en la base de datos, el gancho de guardado del editor, WP-Cron, multisitio y `wp_mail`. En su lugar: importación WXR, archivos en disco, `watch`/cron y `sendmail`/webhook. La lista de funciones se reconstruyó a partir de la descripción pública y de conversaciones previas, no del código fuente del plugin.

Apoye el proyecto: https://www.buymeacoffee.com/echo.of.the.strings

## Límites conocidos

- No envía informes por correo electrónico (SMTP) ni detecta huérfanos por FTP; solo en un directorio local.
- No se leen los sitemaps comprimidos `.xml.gz`.
- Las páginas que construyen sus enlaces con JavaScript no se renderizan (no hay motor de navegador); se analiza el HTML servido.
- La interfaz gráfica solo se ha probado bajo una pantalla virtual (Linux/X11): rastreo, filtros, panel de detalles y Guardian. Las versiones para Windows y macOS las genera la CI, pero no se han probado de forma interactiva.

## Sobre «clean-room»

El código está escrito desde cero, a partir del comportamiento general de los verificadores de enlaces y de especificaciones públicas
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). No contiene código, recursos, texto ni diseño de ningún otro producto y
no se escribió observando el código ni el ejecutable de otro programa.

Licencia: MIT.

## Seguridad y modelo de amenazas

VERIGA procesa contenido no confiable (páginas web, archivos exportados, archivos de configuración). Medidas por defecto:

- **Anti-SSRF:** los destinos privados/locales (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) se bloquean, también tras una redirección. Desactivación explícita: `--allow-private` (CLI) / la casilla de la GUI. *Límite:* el DNS rebinding entre la comprobación y la conexión no queda totalmente cerrado.
- **Cabeceras sensibles** (`Authorization`, `Cookie`, claves de API) se envían solo al origen interno, nunca a otros hosts ni tras una redirección externa.
- **Filtro de inyección en reparaciones:** se rechaza una URL nueva (p. ej. un `Location` hostil) que contenga comillas, `<`, `>`, espacios, comillas invertidas, barras invertidas o corchetes en Markdown (`unsafe_url_for` / `gUnsafeUrl`).
- **Escrituras seguras:** copia de seguridad única, escrituras atómicas, bloqueo cooperativo, nueva comprobación del contenido antes de aplicar (TOCTOU), conservación de permisos y enlaces simbólicos; las propuestas obsoletas se descartan, no se aplican en posiciones erróneas.
- **Exportación CSV** protegida contra fórmulas (`=`, `+`, `-`, `@`).
- **Notificaciones:** `sendmail` debe ser una ruta absoluta; no use archivos de configuración de fuentes no confiables (podrían provocar la ejecución de un binario elegido por el atacante).
- **Límites conocidos:** el HTML escapado en WXR no se analiza (falsos negativos); la edición web está limitada por CORS; `lint` termina con código 2 cuando hay hallazgos.

## Estándares y cumplimiento (comprobado el 27.10.2026)

| Área | Estado |
|---|---|
| Rust | edición **2024**, MSRV **1.88** (núcleo + CLI; sus dependencias declaran como máximo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` y `cargo fmt --check` sin avisos; `--locked` en CI |
| Dependencias | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` con 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos fusionados, coincidencia exacta con el product token, normalización percent/UTF-8, `*` y `$`, prevalece la regla más larga (Allow en caso de empate), límite de 500 KiB, 4xx ⇒ permitido, 5xx/red ⇒ no permitido |
| HTTP | RFC 9110: `Retry-After` (segundos/fecha) en 429/503, redirecciones 301/302/303/307/308, `Location` relativo, `HEAD` con respaldo a `GET` |
| URL | WHATWG URL / IDNA (UTS 46) mediante el crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (índice + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 infracciones, tema claro/oscuro), pestañas WAI-ARIA APG con teclado, `prefers-reduced-motion`, `forced-colors`, objetivos ≥ 24 px, regiones `status`/`progressbar`; **CSP** con hashes (sin `unsafe-inline` en scripts), `referrer: no-referrer`; los informes exportados llevan su propia CSP |
| Cadena de suministro | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, versiones con sumas SHA-256 y atestación de procedencia (Sigstore/SLSA), sin acciones de terceros en la publicación |

Pruebas web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Tras cualquier cambio en `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
