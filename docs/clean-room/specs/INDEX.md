# Index specificații — lotul 1 (spec/batch-1)

Toate au statusul **candidat** (neaprobate; „Aprobată de mine” nebifat). Dimensiuni: S = mică, M = medie, L = mare.

| Ordine | Nr. | Slug | Status | Sursă publică principală | Mărime | Dependențe |
|---|---|---|---|---|---|---|
| 1 | 01 | sitemap-comprimat | candidat | sitemaps.org Protocol; RFC 1952 | S | — (extinde citirea sitemap existentă) |
| 2 | 13 | alias-adrese | candidat | RFC 3986 §6; RFC 9110 §4.2.3 | S | — |
| 3 | 05 | relatii-legatura | candidat | HTML Living Standard (link types) | S | — |
| 4 | 03 | raport-sarif | candidat | OASIS SARIF 2.1.0 | M | — |
| 5 | 04 | raport-junit | candidat | Apache Ant JUnitReport; W3C XML 1.0 | S | — |
| 6 | 11 | scheme-nonweb | candidat | RFC 3966, 5870, 2397; Fetch | S | — |
| 7 | 12 | nume-accesibil-legatura | candidat | WCAG 2.2; Accname 1.2 | M | — |
| 8 | 02 | antet-link | candidat | RFC 8288; RFC 9110 | M | — |
| 9 | 07 | canonica-hreflang | candidat | RFC 6596; RFC 5646 | M | 05 (tipuri rel), 02 (opțional: canonică în antet) |
| 10 | 06 | continut-mixt | candidat | W3C Mixed Content; Secure Contexts | M | — (CSS: legat de extragerea existentă) |
| 11 | 09 | legaturi-din-fluxuri | candidat | RSS 2.0; RFC 4287; XML Base | M | — |
| 12 | 08 | integritate-subresurse | candidat | W3C SRI | M | 06 (aceeași clasificare a subresurselor) |
| 13 | 10 | expirare-certificat | candidat | RFC 5280; RFC 9110; RFC 6125 | M | acces la certificatul frunză din clientul HTTP |

Verificat în README.md (v1.1.0) și prin căutare doar în citire în crates/ că funcțiile de mai sus nu există deja: nu există mențiuni pentru SARIF, JUnit, hreflang, integrity, gzip, antet Link, noopener, X-Robots-Tag, HSTS, expirare certificat. Atenție la suprapuneri parțiale: citirea sitemap-ului și a directivei Sitemap din robots (existente), verificarea sintaxei mailto (existentă), regula de amestec http/https din modulul de reparații (alt scop decât spec 06).
