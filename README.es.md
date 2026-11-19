# VERIGA — verificador de enlaces (implementación independiente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Descarga:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.5.1

Versión: 1.5.1 · Última actualización: 19.11.2026 · Sitio de presentación: https://veriga.chiuta.ro

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

`--honor-nofollow` — los enlaces marcados con `rel="nofollow"` se siguen comprobando (estado HTTP), pero la página de destino no se rastrea en busca de más enlaces; desactivado por defecto. Con `--report-rel`, los tokens `rel` de cada enlace (sin distinguir mayúsculas, únicos) también se indican en el JSON (`rel`), en el CSV/TSV (columna `Rel`) y junto a los enlaces rotos en el informe de texto; sin la opción, esas salidas son exactamente las de la v1.1.0.

`--check-schemes` — valida la sintaxis de las direcciones `tel:`, `geo:` y `data:` (RFC 3966, RFC 5870, RFC 2397 con la tolerancia de base64 de Fetch); no se hace ninguna petición de red. Las inválidas se indican como «URL no válida» con el motivo; las direcciones `data:` de más de 1 MiB se indican como no verificadas. Desactivado por defecto (esos esquemas se ignoran en silencio).

`--check-link-names` — calcula el nombre accesible de cada `<a href>` (aria-labelledby, aria-label, contenido incluido el `alt` de las imágenes, luego `title`; el contenido oculto se ignora) e indica los enlaces sin nombre o solo con signos de puntuación (WCAG 2.4.4 / 4.1.2) en los informes de texto y JSON (`a11y`). `--check-ambiguous-names` añade una nota informativa cuando nombres idénticos (sin distinguir mayúsculas) apuntan a direcciones distintas en una misma página. Ambos desactivados por defecto.

`--check-link-header` — lee los campos de cabecera HTTP `Link` (RFC 8288) de las páginas rastreadas y comprueba cada destino declarado como cualquier otro enlace (origen = la página solicitada, mostrado como «Link header: …»; se conservan los tokens `rel`). Los valores mal formados se omiten con una advertencia (informes de texto/JSON, `warnings`); se leen como máximo 100 enlaces por respuesta. Desactivado por defecto.

`--check-seo-links` — comprueba `rel="canonical"` (destino roto, destino redirigido indicado aparte, varios canónicos distintos en una página; un canónico que apunta a la propia página es correcto, también leído de la cabecera `Link` con `--check-link-header`) y los valores `hreflang` en `link rel=alternate` / `a` (solo sintaxis BCP 47, sin el registro IANA; `x-default` se lista como convención externa, no como error). Los hallazgos están en los informes de texto y JSON (`seo`). Desactivado por defecto.

`--check-mixed-content` — en las páginas `https:`, indica los subrecursos cargados por `http:` (contenido mixto): imágenes, audio y vídeo son «bloqueables opcionalmente»; scripts, hojas de estilo, precargas, iframes, objetos, fuentes, `@import` de CSS y acciones de formulario son «bloqueables». Los enlaces de navegación simples no son subrecursos; `localhost`, `127.0.0.0/8` y `[::1]` nunca se indican; una cabecera `Content-Security-Policy: upgrade-insecure-requests` añade una nota en lugar de anular el hallazgo; un subrecurso `https:` que redirige a `http:` también se indica. Los resultados están en los informes de texto y JSON (`mixed`). Desactivado por defecto.

`--check-feeds` — lee los feeds RSS 2.0 y Atom 1.0 encontrados mediante `<link rel="alternate" type="application/rss+xml|application/atom+xml">` (o cualquier respuesta XML obtenida al rastrear) y comprueba los enlaces que contienen: `link`, `comments`, `enclosure@url`, `source@url` del canal y de los elementos, `url`/`link` de la imagen, un `guid` permanente, `link@href` de Atom (`rel` ausente = alternate), `content@src` y los enlaces en descripciones HTML escapadas/CDATA y en el contenido Atom `html`/`xhtml`. Las direcciones relativas siguen `xml:base` y después la dirección del feed. Un XML mal formado o un documento que no es un feed produce una advertencia y ningún enlace. Desactivado por defecto.

`--check-sri` — verifica el atributo `integrity` de `<script src>` y de `<link rel=stylesheet|preload|modulepreload>`: sintaxis (base64 estándar, longitud de resumen correcta), algoritmos reconocidos (sha256/384/512; decide el más fuerte presente, cualquiera de sus valores puede coincidir), el resumen SHA-2 de los bytes descargados y CORS para los recursos de otro origen (atributo `crossorigin` más un `Access-Control-Allow-Origin` coincidente). Veredictos: coincide / no coincide / sintaxis no válida / ningún algoritmo reconocido / sin CORS / no verificado; `integrity` en otros elementos se anota como no aplicable. Los recursos se descargan completos (GET) solo cuando un enlace pide la comprobación. Los resultados están en los informes de texto y JSON (`sri`). Desactivado por defecto.

`--check-cert-expiry` / `--cert-warn-days N` (14 por defecto) — lee el certificado TLS hoja que presenta cada host:puerto `https` durante la comprobación e indica, una vez por host:puerto con el número de enlaces comprobados allí: caducado, aún no válido, caduca en N días (umbral; `0` = solo caducados), ilegible, o un nombre que no coincide con ninguna entrada SAN. Funciona también con `--insecure`. Un proxy que intercepta TLS muestra su propio certificado; un reloj del sistema equivocado no se puede detectar. Los resultados están en los informes de texto y JSON (`certs`). Desactivado por defecto.

## Proxies gratuitos y éticos

Elige un preajuste con `--proxy-preset tor|tor-browser|warp` (lista: `--list-proxy-presets`) o, en la GUI, en «Proxy gratuito ético…». Todos son puntos SOCKS5 locales (`socks5h://127.0.0.1:…`): los nombres los resuelve el proxy, no tu sistema. Tor (voluntarios, sin ánimo de lucro, código abierto) es la opción recomendada; la comprobación se limita a 4 hilos y 2 conexiones por servidor, y algunos sitios bloquean las salidas de Tor, así que pueden aparecer falsos «enlaces rotos». Cloudflare WARP es gratuito, pero es un servicio comercial con cliente cerrado. Las listas de proxies públicos abiertos se excluyen a propósito: suelen estar comprometidos o interceptar el tráfico. Tu propio proxy sigue disponible con `--proxy`.

## Informes

HTML autónomo (se abre en cualquier parte), CSV, TSV, JSON, texto, sitemap.xml, un árbol del sitio y una sesión `.veriga`.
**Nueva comprobación de enlaces rotos** a partir de una sesión guardada (`--recheck`, un botón en la GUI/web).

**Direcciones alias** (`--report-aliases`): las distintas grafías de una misma dirección (mayúsculas del host, puerto por defecto, caracteres codificados con porcentaje sin necesidad, segmentos de punto) se listan como grupos informativos en los informes de texto y JSON (`aliases`); las mayúsculas de la ruta/consulta, el orden de los parámetros, la barra final y `www.` nunca se agrupan. El seguimiento tiene un tope de memoria (100 000 direcciones, 10 grafías por dirección, 20 páginas por grafía). `--canonical-urls` además fusiona esas grafías en una sola comprobación (e indica la dirección normalizada); por defecto las URL se comprueban e informan exactamente como están escritas, como en la v1.1.0. Otro interruptor desactivado por defecto: `--xml-encoding` (respetar `<?xml encoding=…?>` cuando no se da cabecera ni meta charset).

**SARIF 2.1.0** (`-o report.sarif`): los enlaces rotos pasan a `error`, las redirecciones / 401-403 / respuestas lentas a `warning`, los elementos omitidos u huérfanos a `note`; los enlaces buenos no se emiten. La ubicación es la página (o archivo) que contiene el enlace, como URI; `region.startLine` solo se escribe cuando se conoce una línea (los resultados del rastreo aún no llevan línea, por eso se omite). La salida es determinista y se escribe de forma atómica.

**JUnit XML** (`-o report.junit`): un `testsuite` por página de origen, un `testcase` por enlace comprobado (`name` = destino, `classname` = página de origen). Los enlaces rotos son `failure`; los tiempos agotados / sin conexión son `error`; los elementos omitidos o solo para revisar son `skipped`; los caracteres no permitidos en XML 1.0 pasan a U+FFFD. Los archivos huérfanos no son enlaces y se dejan fuera.

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # carpeta local + huérfanos
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # solo enlaces rotos
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Código de salida `2` si se encontraron enlaces rotos (útil en CI); `--no-fail` lo desactiva. `veriga --help` lo muestra todo. Código de salida `3` si la ejecución se canceló o se truncó por `--max-urls` / `--max-per-pattern` (trampas de rastreo, por defecto 5000 páginas por patrón de ruta); `3` tiene prioridad sobre `2`. `veriga check URL` equivale a `veriga URL`.

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

## Paquetes para móvil y servidor

PWA (instalable, sin conexión): <https://veriga.chiuta.ro/app/> · Termux: `packaging/termux/install.sh` · Docker: `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · temporizador systemd, `.deb`/`.rpm`, CronJob de Kubernetes y paquete **YunoHost**: ver [`packaging/`](packaging/README.md). No se ofrece un APK nativo de Android.

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

- **Protección de red:** la comprobación de direcciones privadas resuelve el host y luego el cliente HTTP lo resuelve otra vez (existe una ventana de DNS rebinding); ejecute VERIGA sobre sitios no fiables desde una red que no pueda alcanzar rangos privados.
- **`veriga watch`** lee por defecto `./veriga-guardian.json` del directorio actual, y ese archivo puede nombrar un comando `sendmail` y las fuentes que se reescriben: no lo ejecute dentro de un directorio en el que no confíe.
- No envía informes por correo electrónico (SMTP) ni detecta huérfanos por FTP; solo en un directorio local.
- Los sitemaps comprimidos `.xml.gz` se leen (gzip se detecta por los primeros bytes, no por la extensión); el tamaño descomprimido está limitado a 50 MB y un sitemap se lee hasta sus primeras 50 000 URL.
- Las páginas que construyen sus enlaces con JavaScript no se renderizan (no hay motor de navegador); se analiza el HTML servido.
- La interfaz gráfica solo se ha probado bajo una pantalla virtual (Linux/X11): rastreo, filtros, panel de detalles y Guardian. Las versiones para Windows y macOS las genera la CI, pero no se han probado de forma interactiva.

## Sobre «clean-room»

VERIGA es una **implementación independiente**, no un proyecto clean-room documentado. No se aplicó un procedimiento formal (equipos separados de análisis e implementación, especificaciones revisadas) y el historial del repositorio empieza con un único commit grande. El código se escribió a partir del comportamiento general de los verificadores de enlaces y de especificaciones públicas (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Que no se usara código ni ejecutable de otro verificador es una declaración del autor y no está demostrado: las sesiones de desarrollo solo se revisaron en parte, y la ausencia de ese uso en las partes no revisadas no puede probarse. El registro de procedencia y los puntos abiertos están en
[docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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

## Estándares y cumplimiento (comprobado el 29.10.2026)

| Área | Estado |
|---|---|
| Rust | edición **2024**, MSRV **1.88** (núcleo + CLI; sus dependencias declaran como máximo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` y `cargo fmt --check` sin avisos; `--locked` en CI |
| Dependencias | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, jiff 1.0, eframe/egui 0.37, rfd 0.17; `cargo audit` con 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos fusionados, coincidencia exacta con el product token, normalización percent/UTF-8, `*` y `$`, prevalece la regla más larga (Allow en caso de empate), límite de 500 KiB, 4xx ⇒ permitido, 5xx/red ⇒ no disponible, la verificación continúa (un sitio caído aparece roto, no «0 rotos»); la página de inicio nunca la bloquea robots.txt |
| HTTP | RFC 9110: `Retry-After` (segundos/fecha) en 429/503, redirecciones 301/302/303/307/308, `Location` relativo, `HEAD` con respaldo a `GET` |
| URL | WHATWG URL / IDNA (UTS 46) mediante el crate `url` (con `idna`) |
| Sitemap | sitemaps.org 0.9 (índice + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 infracciones, tema claro/oscuro), pestañas WAI-ARIA APG con teclado, `prefers-reduced-motion`, `forced-colors`, objetivos ≥ 24 px, regiones `status`/`progressbar`; **CSP** con hashes (sin `unsafe-inline` en scripts), `referrer: no-referrer`; los informes exportados llevan su propia CSP |
| Cadena de suministro | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, versiones con sumas SHA-256 y atestación de procedencia (Sigstore/SLSA), sin acciones de terceros en la publicación |

Pruebas web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Tras cualquier cambio en `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
