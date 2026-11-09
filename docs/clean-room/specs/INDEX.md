# Index specificații — lotul 1 (spec/batch-1)

> Aprobare filtru: **nebifată** în nicio specificație. Implementarea s-a făcut pe baza instrucțiunii generale a titularului din conversație (2026-11-09), nu a unei aprobări individuale conform listei de verificare din fiecare specificație.

Starea reală a tuturor specificațiilor: **implementat, teste trec; filtrul NU este aprobat conform specificației** (căsuțele „Aprobată de mine înainte de predare” și cea despre brevete rămân nebifate). Aprobarea individuală rămâne de făcut de titular. Dimensiuni: S = mică, M = medie, L = mare.

| Ordine | Nr. | Slug | Status | Sursă publică principală | Mărime | Dependențe |
|---|---|---|---|---|---|---|
| 1 | 01 | sitemap-comprimat | implementat (teste trec); aprobare individuală nebifată | sitemaps.org Protocol; RFC 1952 | S | — (extinde citirea sitemap existentă) |
| 2 | 13 | alias-adrese | implementat (teste trec); aprobare individuală nebifată | RFC 3986 §6; RFC 9110 §4.2.3 | S | — |
| 3 | 05 | relatii-legatura | implementat (teste trec); aprobare individuală nebifată | HTML Living Standard (link types) | S | — |
| 4 | 03 | raport-sarif | implementat (teste trec); aprobare individuală nebifată | OASIS SARIF 2.1.0 | M | — |
| 5 | 04 | raport-junit | implementat (teste trec); aprobare individuală nebifată | Apache Ant JUnitReport; W3C XML 1.0 | S | — |
| 6 | 11 | scheme-nonweb | implementat (teste trec); aprobare individuală nebifată | RFC 3966, 5870, 2397; Fetch | S | — |
| 7 | 12 | nume-accesibil-legatura | implementat (teste trec); aprobare individuală nebifată | WCAG 2.2; Accname 1.2 | M | — |
| 8 | 02 | antet-link | implementat (teste trec); aprobare individuală nebifată | RFC 8288; RFC 9110 | M | — |
| 9 | 07 | canonica-hreflang | implementat (teste trec); aprobare individuală nebifată | RFC 6596; RFC 5646 | M | 05 (tipuri rel), 02 (opțional: canonică în antet) |
| 10 | 06 | continut-mixt | implementat (teste trec); aprobare individuală nebifată | W3C Mixed Content; Secure Contexts | M | — (CSS: legat de extragerea existentă) |
| 11 | 09 | legaturi-din-fluxuri | implementat (teste trec); aprobare individuală nebifată | RSS 2.0; RFC 4287; XML Base | M | — |
| 12 | 08 | integritate-subresurse | implementat (teste trec); aprobare individuală nebifată | W3C SRI | M | 06 (aceeași clasificare a subresurselor) |
| 13 | 10 | expirare-certificat | implementat (teste trec); aprobare individuală nebifată | RFC 5280; RFC 9110; RFC 6125 | M | acces la certificatul frunză din clientul HTTP |

Verificat în README.md (v1.1.0) și prin căutare doar în citire în crates/ că funcțiile de mai sus nu există deja: nu există mențiuni pentru SARIF, JUnit, hreflang, integrity, gzip, antet Link, noopener, X-Robots-Tag, HSTS, expirare certificat. Atenție la suprapuneri parțiale: citirea sitemap-ului și a directivei Sitemap din robots (existente), verificarea sintaxei mailto (existentă), regula de amestec http/https din modulul de reparații (alt scop decât spec 06).
