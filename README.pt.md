# VERIGA — verificador de ligações (implementação independente)

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

**Transferência:** https://veriga.chiuta.ro/#descarca · https://github.com/chiuta/VERIGA/releases/tag/v1.1.0

Versão: 1.1.0 · Última atualização: 30.10.2026 · Site de apresentação: https://veriga.chiuta.ro

Uma aplicação Rust de 64 bits para Windows, macOS e Linux (x86-64 e ARM64) que rastreia um site
ou uma pasta local e verifica as suas ligações. Vem em três formas que partilham o mesmo modelo de resultados:

| Forma | Onde | O que faz |
|---|---|---|
| `veriga` (CLI) | terminal, CI, contentores (binário estático musl) | verificação completa, relatórios, código de saída |
| `veriga-gui` | desktop nativo (egui) | tabela ordenável/filtrável, painel de detalhes, pausar/parar, exportar |
| `web/veriga.html` | qualquer navegador, ficheiro único, offline | análise de pasta local, site online (dentro dos limites do CORS), lista de URLs |

As sessões (`.veriga`, JSON) são intercambiáveis entre as três formas.

## Sobre a etiqueta "clean-room"

VERIGA é uma **implementação independente**, não um projeto clean-room documentado. Não foi aplicado um procedimento formal (equipas separadas de análise e implementação, especificações revistas) e o histórico do repositório começa com um único commit grande. O código foi escrito a partir do comportamento geral dos verificadores de ligações e de especificações públicas (HTML, CSS, HTTP, robots.txt/RFC 9309, FTP, sitemaps.org). As sessões de desenvolvimento do autor não mostram o uso do código ou do executável de outro verificador. O registo de proveniência e os pontos em aberto estão em [docs/clean-room/PROVENANCE.md](docs/clean-room/PROVENANCE.md).

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
| Dependências | reqwest 0.13 (rustls + platform verifier, HTTP/2, gzip/brotli/zstd), scraper 0.29, thiserror 2, eframe/egui 0.37, rfd 0.17; `cargo audit` com 0 vulnerabilidades; `cargo deny` (advisories, licenses, sources, bans) |
| robots.txt | **RFC 9309**: grupos mesclados, correspondência exata no product token, normalização percent/UTF-8, `*` e `$`, a regra mais longa vence (Allow em caso de empate), limite de 500 KiB, 4xx ⇒ permitido, 5xx/rede ⇒ indisponível, a verificação continua (um site em baixo aparece quebrado, não «0 quebrados»); a página inicial nunca é bloqueada pelo robots.txt |
| HTTP | RFC 9110: `Retry-After` (segundos/data) em 429/503, redirecionamentos 301/302/303/307/308, `Location` relativo, `HEAD` com fallback para `GET` |
| URL | WHATWG URL / IDNA (UTS 46) através do crate `url` (com `idna`) |
| Sitemap | sitemaps.org 0.9 (index + urlset) |
| Web (veriga.html) | **WCAG 2.2 AA** (axe-core 4.14: 0 violações, tema claro/escuro), separadores WAI-ARIA APG com teclado, `prefers-reduced-motion`, `forced-colors`, alvos ≥ 24 px, regiões `status`/`progressbar`; **CSP** com hashes (sem `unsafe-inline` para scripts), `referrer: no-referrer`; os relatórios exportados incluem a sua própria CSP |
| Cadeia de abastecimento | `LICENSE` (MIT), `SECURITY.md`, `deny.toml`, Dependabot, releases com somas SHA-256 e atestado de proveniência (Sigstore/SLSA), sem actions de terceiros na publicação |

Testes web: `cd web/tests && npm install && npx playwright install chromium && npm test`. Depois de qualquer alteração em `veriga.html`: `python3 web/stamp-csp.py web/veriga.html`.
