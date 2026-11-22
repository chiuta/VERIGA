# VERIGA — verificador de ligações (implementação independente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Transferência:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.5.1

Versão: 1.5.1 · Última atualização: 2026-11-22 · Site de apresentação: https://veriga.chiuta.ro

Uma aplicação Rust de 64 bits para Windows, macOS e Linux (x86-64 e ARM64) que rastreia um site
ou uma pasta local e verifica as suas ligações. Vem em três formas que partilham o mesmo modelo de resultados:

| Forma | Onde | O que faz |
|---|---|---|
| `veriga` (CLI) | terminal, CI, contentores (binário estático musl) | verificação completa, relatórios, código de saída |
| `veriga-gui` | desktop nativo (egui) | tabela ordenável/filtrável, painel de detalhes, pausar/parar, exportar |
| `web/veriga.html` | qualquer navegador, ficheiro único, offline | análise de pasta local, site online (dentro dos limites do CORS), lista de URLs |

As sessões (`.veriga`, JSON) são intercambiáveis entre as três formas.

## O que ele verifica

- **Ligações HTML**: `a`, `area`, `img` (+`srcset`, `data-src`), `source`, `video`/`audio`/`track`, `embed`, `object`/`param`,
  `applet`, `iframe`/`frame`, `script`, `link` (estilos, ícones, preload), `form[action]` (opcional), `input[type=image]`,
  `background=`, `meta refresh`, `og:image`/`twitter:image`, `cite`, SVG `image`/`use`, `<base href>`.
- **CSS**: `url(...)` e `@import`, tanto de ficheiros `.css` quanto de `<style>` e atributos `style`.
- **Protocolos**: HTTP/HTTPS (HEAD com fallback para GET), `file://` e caminhos locais, FTP (autenticação, `SIZE`/`CWD`),
  `mailto:` (sintaxe).
- **Redirecionamentos**: cadeia completa com códigos de estado, deteção de loops, limite configurável, opção "redirecionamentos são erros".
- **Âncoras `#fragment`**: verificadas nas páginas rastreadas (`id` e `name`).
- **Ficheiros órfãos**: ficheiros no diretório do site para os quais nenhuma ligação aponta.
- **Diferenças de maiúsculas/minúsculas** em caminhos locais (funciona no Windows/macOS, falha no Linux).
- **robots.txt** (grupos, `Allow`/`Disallow` com `*` e `$`, `Crawl-delay`) e partida a partir do **sitemap.xml**.
- **Observações**: títulos duplicados/ausentes, respostas lentas, estatísticas por tipo MIME.
- Codificações de caracteres (cabeçalho, BOM, `<meta charset>`) — inclusive páginas antigas em Windows-1250.

## Principais opções

Threads simultâneas e limite por host, tempo limite, novas tentativas (respeitando `Retry-After`), profundidade máxima, limite de URLs,
ligações externas sim/não, "ficar no diretório", excluir / não seguir / apenas estes (regex), hosts internos extras,
códigos HTTP tratados como OK (por exemplo, 999), User-Agent, proxy (HTTP/SOCKS), autenticação Basic (apenas hosts internos),
cabeçalhos e cookies, certificados inválidos, intervalo entre pedidos.

`--honor-nofollow` — as ligações marcadas com `rel="nofollow"` continuam a ser verificadas (estado HTTP), mas a página de destino não é rastreada à procura de mais ligações; desativado por predefinição. Com `--report-rel`, os tokens `rel` de cada ligação (sem distinguir maiúsculas de minúsculas, únicos) também são indicados no JSON (`rel`), no CSV/TSV (coluna `Rel`) e junto às ligações quebradas no relatório de texto; sem a opção, essas saídas são exatamente as da v1.1.0.

`--check-schemes` — valida a sintaxe dos endereços `tel:`, `geo:` e `data:` (RFC 3966, RFC 5870, RFC 2397 com a tolerância de base64 do Fetch); não é feito nenhum pedido de rede. Os inválidos são indicados como «URL inválido» com o motivo; os endereços `data:` com mais de 1 MiB são indicados como não verificados. Desativado por predefinição (esses esquemas são ignorados em silêncio).

`--check-link-names` — calcula o nome acessível de cada `<a href>` (aria-labelledby, aria-label, conteúdo incluindo o `alt` das imagens, depois `title`; o conteúdo oculto é ignorado) e indica as ligações sem nome ou apenas com pontuação (WCAG 2.4.4 / 4.1.2) nos relatórios de texto e JSON (`a11y`). `--check-ambiguous-names` acrescenta uma nota informativa quando nomes idênticos (sem distinguir maiúsculas de minúsculas) apontam para endereços diferentes numa página. Ambos desativados por predefinição.

`--check-link-header` — lê os campos de cabeçalho HTTP `Link` (RFC 8288) das páginas rastreadas e verifica cada destino declarado como qualquer outra ligação (origem = a página pedida, apresentada como «Link header: …»; os tokens `rel` são mantidos). Os valores mal formados são ignorados com um aviso (relatórios de texto/JSON, `warnings`); lêem-se no máximo 100 ligações por resposta. Desativado por predefinição.

`--check-seo-links` — verifica `rel="canonical"` (destino quebrado, destino redirecionado indicado à parte, vários canónicos diferentes numa página; um canónico que aponta para a própria página está correto, também lido do cabeçalho `Link` com `--check-link-header`) e os valores `hreflang` em `link rel=alternate` / `a` (apenas sintaxe BCP 47, sem o registo IANA; `x-default` é listado como convenção externa, não como erro). Os resultados estão nos relatórios de texto e JSON (`seo`). Desativado por predefinição.

`--check-mixed-content` — nas páginas `https:`, indica os subrecursos carregados por `http:` (conteúdo misto): imagens, áudio e vídeo são «bloqueáveis opcionalmente»; scripts, folhas de estilo, pré-carregamentos, iframes, objetos, tipos de letra, `@import` de CSS e ações de formulário são «bloqueáveis». As ligações de navegação simples não são subrecursos; `localhost`, `127.0.0.0/8` e `[::1]` nunca são indicados; um cabeçalho `Content-Security-Policy: upgrade-insecure-requests` acrescenta uma nota em vez de anular o achado; um subrecurso `https:` que redireciona para `http:` também é indicado. Os resultados estão nos relatórios de texto e JSON (`mixed`). Desativado por predefinição.

`--check-feeds` — lê os feeds RSS 2.0 e Atom 1.0 encontrados através de `<link rel="alternate" type="application/rss+xml|application/atom+xml">` (ou qualquer resposta XML obtida ao rastrear) e verifica as ligações que contêm: `link`, `comments`, `enclosure@url`, `source@url` do canal e dos itens, `url`/`link` da imagem, um `guid` permanente, `link@href` do Atom (`rel` em falta = alternate), `content@src` e as ligações nas descrições HTML com escape/CDATA e no conteúdo Atom `html`/`xhtml`. Os endereços relativos seguem `xml:base` e depois o endereço do feed. Um XML mal formado ou um documento que não é um feed produz um aviso e nenhuma ligação. Desativado por predefinição.

`--check-sri` — verifica o atributo `integrity` de `<script src>` e de `<link rel=stylesheet|preload|modulepreload>`: sintaxe (base64 padrão, comprimento do resumo correto), algoritmos reconhecidos (sha256/384/512; decide o mais forte presente, qualquer um dos seus valores pode coincidir), o resumo SHA-2 dos bytes transferidos e CORS para recursos de outra origem (atributo `crossorigin` mais um `Access-Control-Allow-Origin` correspondente). Veredictos: coincide / não coincide / sintaxe inválida / nenhum algoritmo reconhecido / sem CORS / não verificado; `integrity` noutros elementos é anotado como não aplicável. Os recursos são transferidos por inteiro (GET) apenas quando uma ligação pede a verificação. Os resultados estão nos relatórios de texto e JSON (`sri`). Desativado por predefinição.

`--check-cert-expiry` / `--cert-warn-days N` (14 por predefinição) — lê o certificado TLS folha que cada host:porta `https` apresenta durante a verificação e indica, uma vez por host:porta com o número de ligações verificadas aí: expirado, ainda não válido, expira em N dias (limiar; `0` = apenas expirados), ilegível, ou um nome que não corresponde a nenhuma entrada SAN. Funciona também com `--insecure`. Um proxy que intercepta o TLS mostra o seu próprio certificado; um relógio do sistema errado não pode ser detetado. Os resultados estão nos relatórios de texto e JSON (`certs`). Desativado por predefinição.

## Proxies gratuitos e éticos

Escolha uma predefinição com `--proxy-preset tor|tor-browser|warp` (lista: `--list-proxy-presets`) ou, na GUI, em «Proxy gratuito ético…». Todas são pontos SOCKS5 locais (`socks5h://127.0.0.1:…`): os nomes são resolvidos pelo proxy, não pelo seu sistema. O Tor (voluntários, sem fins lucrativos, código aberto) é a escolha recomendada; a verificação fica limitada a 4 threads e 2 ligações por servidor, e alguns sites bloqueiam as saídas Tor, por isso podem surgir falsos «links quebrados». O Cloudflare WARP é gratuito, mas é um serviço comercial com cliente fechado. As listas de proxies públicos abertos ficam de fora de propósito: costumam estar comprometidos ou intercetar o tráfego. O seu próprio proxy continua disponível com `--proxy`.

## Relatórios

HTML autónomo (abre em qualquer sítio), CSV, TSV, JSON, texto, sitemap.xml, árvore do site, sessão `.veriga`.
**Nova verificação das ligações quebradas** a partir de uma sessão guardada (`--recheck`, um botão na GUI/web).

**Endereços alias** (`--report-aliases`): as diferentes grafias do mesmo endereço (maiúsculas do host, porta predefinida, carateres codificados com percentagem sem necessidade, segmentos de ponto) são listadas como grupos informativos nos relatórios de texto e JSON (`aliases`); maiúsculas do caminho/consulta, ordem dos parâmetros, barra final e `www.` nunca são agrupados. O acompanhamento tem um limite de memória (100 000 endereços, 10 grafias por endereço, 20 páginas por grafia). `--canonical-urls` junta ainda essas grafias numa única verificação (e indica o endereço normalizado); por predefinição os URL são verificados e indicados exatamente como estão escritos, como na v1.1.0. Outro interruptor desativado por predefinição: `--xml-encoding` (respeitar `<?xml encoding=…?>` quando não há cabeçalho nem meta charset).

**SARIF 2.1.0** (`-o report.sarif`): as ligações quebradas passam a `error`, os redirecionamentos / 401-403 / respostas lentas a `warning`, os itens ignorados ou órfãos a `note`; as ligações boas não são emitidas. A localização é a página (ou ficheiro) que contém a ligação, como URI; `region.startLine` só é escrito quando se conhece uma linha (os resultados do rastreio ainda não trazem linha, por isso é omitido). A saída é determinista e escrita de forma atómica.

**JUnit XML** (`-o report.junit`): um `testsuite` por página de origem, um `testcase` por ligação verificada (`name` = destino, `classname` = página de origem). As ligações quebradas são `failure`; os tempos esgotados / sem ligação são `error`; os itens ignorados ou só para revisão são `skipped`; os carateres não permitidos em XML 1.0 passam a U+FFFD. Os ficheiros órfãos não são ligações e ficam de fora.

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # pasta local + órfãos
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # somente links quebrados
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Código de saída `2` se houver ligações quebradas (útil em CI); `--no-fail` desativa isso. `veriga --help` lista tudo. Código de saída `3` se a execução foi cancelada ou truncada por `--max-urls` / `--max-per-pattern` (armadilhas de rastreio, por omissão 5000 páginas por padrão de caminho); `3` tem prioridade sobre `2`. `veriga check URL` equivale a `veriga URL`.

## Compilação

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # somente a CLI
cargo test --workspace
```

O workflow `.github/workflows/build.yml` gera pacotes para: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, com GUI) e Linux x64/ARM64 (musl, CLI estática). O `Dockerfile` gera uma imagem mínima com a CLI.

## Tutoriais em vídeo

Dez tutoriais curtos gravados na aplicação real e legendados em 8 idiomas (ro, en, fr, it, es, pt, de, hu): primeira verificação, filtros, detalhe de uma ligação, opções, exportação, Guardian (correções; domínios, mapa, tendências), linha de comandos, edição HTML e edição portátil. Em https://veriga.chiuta.ro/#tutoriale ou em `assets/tutorials/`: cada `.mp4` traz as 8 faixas de legendas e os ficheiros `.vtt` acompanham-nos. Sem áudio; as explicações estão nas legendas.

## Edição portátil (PortableApps.com)

O VERIGA também é distribuído como pacote **PortableApps.com Format 3.9**, `VERIGAPortable_<versão>_Multilingual.zip` (ficheiros da release). Descomprima-o em qualquer lado (pen USB, pasta sincronizada) e inicie o `VERIGAPortable.exe`.

- Tudo o que a aplicação produz fica em `Data\` (estado do Guardian em `Data\.veriga`); nada é escrito no registo nem em `%APPDATA%`. Apagar `Data\` é seguro: é recriada.
- O lançador escolhe a versão x64 ou ARM64 conforme a arquitetura nativa; `VERIGAPortable.exe sessao.veriga` abre uma sessão guardada (associação de ficheiros `.veriga`).
- A CLI é `App\VERIGA\veriga.exe`. A estrutura é verificada na CI com `portableapps/validate.py`; o código do lançador é `crates/veriga-launcher`.
- O instalador oficial `.paf.exe` constrói-se com o PortableApps.com Installer (ferramenta para Windows); a pasta já tem a estrutura exigida.

## Pacotes para telemóvel e servidor

PWA (instalável, offline): <https://veriga.chiuta.ro/app/> · Termux: `packaging/termux/install.sh` · Docker: `docker run --rm ghcr.io/chiuta/veriga check https://example.org` · temporizador systemd, `.deb`/`.rpm`, CronJob Kubernetes e pacote **YunoHost**: ver [`packaging/`](packaging/README.md). Não é fornecido um APK Android nativo.

## A edição HTML de ficheiro único

`web/veriga.html` corre a 100% no navegador, sem servidor:

- **Pasta local** (arrasta a pasta do site ou usa "Escolher pasta"): verificação completa de ligações internas, CSS, âncoras, órfãos,
  diferenças de maiúsculas/minúsculas e títulos duplicados. Aqui não há limitação do navegador.
- **Site online**: um navegador não consegue ler o estado HTTP de outros domínios (CORS). Resultados completos quando o ficheiro é aberto a partir do
  próprio site, ou através de um proxy CORS teu (modelo `https://proxy/?u={url}`). Caso contrário, as ligações externas são marcadas
  como *não verificáveis* se o servidor responder, ou *sem ligação* se não responder — nunca um falso "OK".
- **Lista de URLs / HTML colado**.
- Não verifica FTP nem lê robots.txt (restrições do navegador). Usa a CLI/GUI para isso.

## Guardian — reparação, histórico, conformidade

A VERIGA também inclui as funcionalidades de um "guardião de ligações" (inspirado no plugin *Trade-Free Link Guardian* para WordPress), mas como uma ferramenta independente que trabalha com **ficheiros**: HTML, Markdown, exportação do WordPress (WXR `.xml`, menus incluídos) e sitemaps.

* **Reparações determinísticas, verificadas em direto** — `scheme_typo` (`htp://`, `https//`), `unmangle` (esquema duplicado, `%20`, pontuação final), `missing_scheme` (`www.…`), `tracking` (`utm_*`, `fbclid`, `gclid`…), `protocol` (http→https), `www` (consistência), `redirect` (apenas 301/308). Uma reparação é aplicada automaticamente **apenas se a nova URL respondeu com sucesso**; sem verificação, permanece como "proposta".
* **Reparações manuais** — substituição de URL, "remover ligação" (mantém o texto), consulta ao Wayback Machine (opcional, nunca chamada automaticamente).
* **Segurança** — uma cópia de segurança por execução, um diário de alterações, **desfazer** individual ou por execução (byte a byte, inclusive `&amp;`); apenas os bytes do excerto são escritos.
* **401/403 = "merece uma segunda vista de olhos"**, não ligação quebrada (opção, ativada por predefinição).
* **Vistas e relatórios** — domínios, mapa de saúde por página, vista "livro", redirecionamentos (permanentes/temporários), deriva de conteúdo (título/H1 em relação à execução anterior), novos/resolvidos, tendências a partir do histórico, **relatório de conformidade** por período, um selo de impacto (no menu ou em ≥ 3 páginas). As exportações CSV neutralizam células com fórmulas.
* **404s de logs de acesso** (Apache/nginx), com sugestões de redirecionamento e exportação para `.htaccess`.
* **Agendamento** — `veriga wizard` (4 etapas) grava `veriga-guardian.json`; `veriga watch --config … [--once]` executa periodicamente (ou via cron/Agendador de Tarefas), mantém o histórico, repara automaticamente as regras escolhidas e notifica por webhook ou `sendmail`.

```
veriga lint ./site                       # análise offline
veriga fix ./site --session s.veriga     # propõe + verifica ao vivo (simulação)
veriga fix ./site --apply                # grava, com backup e diário
veriga fix ./site --unlink https://dead.example/x --apply
veriga undo --list ; veriga undo --run 20261027-173720
veriga log404 access.log --known s.veriga -o redirect.htaccess
veriga guardian s.veriga --prev old.veriga --record -o guardian.html
veriga compliance s.veriga --days 30 -o compliance.html
```

GUI: o botão **🛡 Guardian**. Edição HTML: a secção **Guardian** (as alterações são feitas em memória; transferes os ficheiros ou escreves-os numa pasta onde o navegador o permitir; a verificação em direto é limitada pelo CORS).

**O que não é possível replicar a partir de um plugin do WordPress:** a consulta do `old slug` na base de dados, o hook de gravação do editor, o WP-Cron, o multisite e o `wp_mail`. Em seu lugar: importação WXR, ficheiros em disco, `watch`/cron e `sendmail`/webhook. A lista de funcionalidades foi reconstruída a partir da descrição pública e de conversas anteriores, não do código-fonte do plugin.

Apoia o projeto: https://www.buymeacoffee.com/echo.of.the.strings

## Limites conhecidos

- **Proteção de rede:** a verificação de endereços privados resolve o host e depois o cliente HTTP resolve-o outra vez (existe uma janela de DNS rebinding); execute o VERIGA em sites não fiáveis a partir de uma rede que não alcance intervalos privados.
- **`veriga watch`** lê por predefinição `./veriga-guardian.json` da pasta atual, e esse ficheiro pode indicar um comando `sendmail` e as fontes a reescrever: não o execute numa pasta em que não confie.
- Não envia relatórios por e-mail (SMTP) e não deteta órfãos por FTP — apenas num diretório local.
- Os sitemaps comprimidos `.xml.gz` são lidos (o gzip é detetado pelos primeiros bytes, não pela extensão); o tamanho descomprimido está limitado a 50 MB e um sitemap é lido até aos seus primeiros 50 000 URL.
- Páginas que montam as suas ligações com JavaScript não são renderizadas (não há motor de navegador); o HTML servido é que é analisado.
- A interface gráfica só foi exercitada num ecrã virtual (Linux/X11): rastreio, filtros, painel de detalhes e Guardian. As versões para Windows e macOS são produzidas pela CI, mas não foram testadas interativamente.

## Sobre "clean-room"

VERIGA é uma **implementação independente**, não um projeto clean-room documentado. Não foi aplicado um procedimento formal (equipas separadas de análise e implementação, especificações revistas) e o histórico do repositório começa com um único commit grande. O código foi escrito a partir do comportamento geral dos verificadores de ligações e de especificações públicas (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Que não tenha sido usado código nem executável de outro verificador é uma declaração do autor e não está provado: as sessões de desenvolvimento foram revistas apenas em parte, e a ausência de tal uso nas partes não revistas não pode ser demonstrada. O registo de proveniência e os pontos em aberto estão em
[docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

Licença: MIT.

## Segurança e modelo de ameaças

A VERIGA processa conteúdo não fidedigno (páginas web, ficheiros exportados, ficheiros de configuração). Medidas predefinidas:

- **Anti-SSRF:** destinos privados/locais (127.0.0.0/8, RFC1918, link-local, `localhost`, `.local`, IPv6 ULA) são bloqueados, inclusive após um redirecionamento. Desativação explícita: `--allow-private` (CLI) / a caixa de seleção da GUI. *Limite:* o DNS rebinding entre a verificação e a ligação não está totalmente fechado.
- **Cabeçalhos sensíveis** (`Authorization`, `Cookie`, chaves de API) são enviados apenas à origem interna, nunca a outros hosts nem após um redirecionamento externo.
- **Filtro de injeção para reparações:** uma nova URL (por exemplo, um `Location` hostil) contendo aspas, `<`, `>`, espaços, crases, barras invertidas ou parênteses retos em Markdown é recusada (`unsafe_url_for` / `gUnsafeUrl`).
- **Escritas seguras:** cópia de segurança única, escritas atómicas, bloqueio cooperativo, nova verificação do conteúdo antes de aplicar (TOCTOU), preservação de permissões/symlinks; propostas obsoletas são descartadas, não aplicadas em posições erradas.
- **Exportação CSV** protegida contra fórmulas (`=`, `+`, `-`, `@`).
- **Notificações:** `sendmail` deve ser um caminho absoluto; não uses ficheiros de configuração de fontes não fidedignas (poderiam provocar a execução de um binário escolhido).
- **Limites conhecidos:** HTML escapado em WXR não é examinado (falsos negativos); a edição web é limitada pelo CORS; `lint` termina com código 2 quando há ocorrências.

## Padrões e conformidade (verificado em 29.10.2026)

| Área | Estado |
|---|---|
| Rust | edição **2024**, MSRV **1.88** (core + CLI; as suas dependências declaram no máximo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` e `cargo fmt --check` sem avisos; `--locked` no CI |
| Dependências | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, jiff 1.0, eframe/egui 0.37, rfd 0.17; `cargo audit` com 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos mesclados, correspondência exata no product token, normalização percent/UTF-8, `*` e `$`, a regra mais longa vence (Allow em caso de empate), limite de 500 KiB, 4xx ⇒ permitido, 5xx/rede ⇒ indisponível, a verificação continua (um site em baixo aparece quebrado, não «0 quebrados»); a página inicial nunca é bloqueada pelo robots.txt |
| HTTP | RFC 9110: `Retry-After` (segundos/data) em 429/503, redirecionamentos 301/302/303/307/308, `Location` relativo, `HEAD` com fallback para `GET` |
| URL | WHATWG URL / IDNA (UTS 46) através do crate `url` (com `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violações, tema claro/escuro), separadores WAI-ARIA APG com teclado, `prefers-reduced-motion`, `forced-colors`, alvos ≥ 24 px, regiões `status`/`progressbar`; **CSP** com hashes (sem `unsafe-inline` para scripts), `referrer: no-referrer`; os relatórios exportados incluem a sua própria CSP |
| Cadeia de abastecimento | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, releases com somas SHA-256 e atestado de proveniência (Sigstore/SLSA), sem actions de terceiros na publicação |

Testes web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Depois de qualquer alteração em `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
