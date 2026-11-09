# Funcție: Extragerea legăturilor din fluxuri RSS și Atom

Sursă publică: «RSS 2.0 Specification», RSS Advisory Board — https://www.rssboard.org/rss-specification ; «RFC 4287 The Atom Syndication Format» — https://www.rfc-editor.org/rfc/rfc4287 ; «XML Base (Second Edition)», W3C Recommendation — https://www.w3.org/TR/xmlbase/
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README nu citește fluxuri de sindicalizare (apar doar în exporturi WXR din modulul de reparații).

## Scop
Un flux găsit în site (de pildă prin link rel="alternate" cu tip application/rss+xml sau application/atom+xml) își are legăturile verificate.

## Intrări / ieșiri
- Intrare: octeții unui flux RSS 2.0 sau Atom 1.0 și adresa lui.
- Ieșire: lista de legături cu rolul lor (pagină, enclosure, imagine, comentarii, sursă) și adresa absolută.

## Comportament observabil
1. RSS 2.0: se extrag channel/link, channel/image/url, channel/image/link, item/link, item/comments, item/enclosure@url, item/source@url [observat].
2. item/guid: este o legătură doar dacă isPermaLink nu este «false» (implicit «true») și valoarea e o adresă [observat].
3. item/description și content:encoded pot conține HTML scăpat sau în CDATA; legăturile din el se extrag ca din HTML, relative la item/link sau channel/link [dedus; specificația spune că descrierea poate fi HTML scăpat].
4. Atom: elementele link cu atributul href; rel lipsă înseamnă «alternate» [observat, RFC 4287 §4.2.7.2]; rel «enclosure», «self», «related», «replies» se extrag la fel; legăturile entry/content@src [observat].
5. Atom: conținutul de tip «html» (scăpat) sau «xhtml» conține HTML ale cărui legături se extrag ca la pct. 3 [observat că tipurile există; extragerea este dedusă].
6. Adresele relative se rezolvă după xml:base cel mai apropiat din ascendenți [observat, XML Base], apoi față de adresa fluxului [observat, RFC 4287 §2]; în RSS, adresele relative nu sunt definite și se rezolvă față de adresa fluxului [dedus; necunoscut în standard].
7. Un document care nu e RSS sau Atom (altă rădăcină) → nicio legătură, avertisment [dedus].

## Cazuri limită și erori
- XML prost format → eroare de analiză, legăturile din fluxul întreg se omit [dedus].
- Atom cu rel în formă de adresă IRI → se tratează ca tip necunoscut dar se verifică [dedus].
- Codare declarată în declarația XML → respectată; lipsă → UTF-8 [observat în XML 1.0].
- Entități HTML în text (de pildă &amp;) → decodate o dată de XML; HTML scăpat se decodează încă o dată la analiza lui [dedus].

## Teste de acceptanță
1. RSS cu un item/link și un enclosure@url → 2 legături (pagină, enclosure).
2. item/guid isPermaLink="false" cu valoarea abc123 → nicio legătură din guid.
3. Atom cu link fără rel și href="/a" pe flux https://ex.org/f.xml → https://ex.org/a, rol «alternate».
4. Atom cu xml:base="https://cdn.ex/z/" pe feed și link href="p.html" → https://cdn.ex/z/p.html.
5. RSS item/description cu CDATA conținând a href="http://x.ex/q" → legătura apare în rezultat.
6. Document cu rădăcina html → 0 legături, avertisment.
7. Atom entry/content src="http://x.ex/doc.pdf" → legătură extrasă.

## Necunoscute
- [necunoscut] Rezolvarea adreselor relative în RSS: specificația nu o definește.
- [necunoscut] RSS 0.9x/1.0 (RDF) nu sunt acoperite de sursele citite.

---

## Checklist de filtru (verificare.md)

Spec: 09-legaturi-din-fluxuri.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
