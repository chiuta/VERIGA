# VERIGA — verificador de ligações (clean-room)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versão: 1.0.0 · Última atualização: 27.10.2026 · Site de apresentação: https://veriga.chiuta.ro

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

## Relatórios

HTML autónomo (abre em qualquer sítio), CSV, TSV, JSON, texto, sitemap.xml, árvore do site, sessão `.veriga`.
**Nova verificação das ligações quebradas** a partir de uma sessão guardada (`--recheck`, um botão na GUI/web).

## CLI

```bash
veriga https://example.com -o report.html -o broken.csv --only-broken
veriga ./site --orphans ./site --fragments -o report.html          # pasta local + órfãos
veriga https://example.com --sitemap --stay-in-dir -d 4 -t 50
veriga --recheck session.veriga -o report.html                      # somente links quebrados
veriga https://example.com --ignore-status 999 --exclude '/logout' -o s.veriga
```

Código de saída `2` se houver ligações quebradas (útil em CI); `--no-fail` desativa isso. `veriga --help` lista tudo.

## Compilação

```bash
cargo build --release                 # veriga + veriga-gui
cargo build --release -p veriga-cli   # somente a CLI
cargo test --workspace
```

O workflow `.github/workflows/build.yml` gera pacotes para: Windows x64/ARM64, macOS Intel/Apple Silicon,
Linux x64/ARM64 (glibc, com GUI) e Linux x64/ARM64 (musl, CLI estática). O `Dockerfile` gera uma imagem mínima com a CLI.

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

- Não envia relatórios por e-mail (SMTP) e não deteta órfãos por FTP — apenas num diretório local.
- Sitemaps comprimidos `.xml.gz` não são lidos.
- Páginas que montam as suas ligações com JavaScript não são renderizadas (não há motor de navegador); o HTML servido é que é analisado.
- A interface gráfica foi compilada e iniciada num ecrã virtual (sem erros no arranque), mas não foi exercitada visualmente.

## Sobre "clean-room"

O código foi escrito do zero, com base no comportamento geral dos verificadores de ligações e em especificações públicas
(HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). Não contém código, recursos, texto ou design de nenhum outro produto e
não foi escrito a olhar para o código ou o executável de outro programa.

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

## Padrões e conformidade (verificado em 27.10.2026)

| Área | Estado |
|---|---|
| Rust | edição **2024**, MSRV **1.88** (core + CLI; as suas dependências declaram no máximo 1.88), GUI **1.95** (egui 0.37); `cargo clippy -- -D warnings` e `cargo fmt --check` sem avisos; `--locked` no CI |
| Dependências | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` com 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos mesclados, correspondência exata no product token, normalização percent/UTF-8, `*` e `$`, a regra mais longa vence (Allow em caso de empate), limite de 500 KiB, 4xx ⇒ permitido, 5xx/rede ⇒ proibido |
| HTTP | RFC 9110: `Retry-After` (segundos/data) em 429/503, redirecionamentos 301/302/303/307/308, `Location` relativo, `HEAD` com fallback para `GET` |
| URL | WHATWG URL / IDNA (UTS 46) através do crate `url` (com `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violações, tema claro/escuro), separadores WAI-ARIA APG com teclado, `prefers-reduced-motion`, `forced-colors`, alvos ≥ 24 px, regiões `status`/`progressbar`; **CSP** com hashes (sem `unsafe-inline` para scripts), `referrer: no-referrer`; os relatórios exportados incluem a sua própria CSP |
| Cadeia de abastecimento | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, releases com somas SHA-256 e atestado de proveniência (Sigstore/SLSA), sem actions de terceiros na publicação |

Testes web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Depois de qualquer alteração em `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
