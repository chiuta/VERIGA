# Funcție: Export de raport în format SARIF 2.1.0

Sursă publică: «Static Analysis Results Interchange Format (SARIF) Version 2.1.0», OASIS Standard — https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html (JSON Schema: https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json) ; «RFC 8259 JSON» — https://www.rfc-editor.org/rfc/rfc8259 ; «RFC 3986» pentru uri.
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README.md enumeră rapoarte HTML/CSV/TSV/JSON/text, nu SARIF; SARIF este consumat de platforme de analiză a codului.

## Scop
Rezultatele verificării (legături rupte, redirecționări, avertismente) pot fi încărcate în instrumente care citesc SARIF.

## Intrări / ieșiri
- Intrare: setul de rezultate al unei rulări (adresă, pagină sursă, fișier și linie dacă sunt cunoscute, stare, cod, mesaj).
- Ieșire: un document JSON UTF-8 SARIF 2.1.0.

## Comportament observabil
1. Rădăcina conține proprietățile «version» cu valoarea «2.1.0» și «runs» (tablou) [observat, §3.13]; proprietatea «$schema» cu adresa schemei este recomandată [observat].
2. Un singur element în «runs», cu «tool.driver.name» obligatoriu [observat, §3.18, §3.19] și opțional «version».
3. «tool.driver.rules» listează regulile proprii (de pildă: legătură ruptă, redirecționare, avertisment de conținut) cu «id» și «shortDescription» [observat că rules există; setul de reguli este ales de implementare].
4. Fiecare rezultat din «results»: «ruleId», «level» din {none, note, warning, error} [observat, §3.27.10], «message.text» obligatoriu [observat], «locations».
5. Corespondență de niveluri: ruptă → error; redirecționare/lentă/avertisment → warning; informație → note [dedus]. Legăturile bune nu se emit implicit [dedus].
6. Locația: «physicalLocation.artifactLocation.uri» = fișierul sau pagina sursă ca referință URI; «region.startLine» ≥ 1 numai dacă linia este cunoscută [observat, §3.30.5]; altfel regiunea lipsește [dedus].
7. Adresa țintă și starea HTTP merg în «properties» (sac de proprietăți liber) [observat că properties există].
8. Fără rezultate: «results» este un tablou gol, nu lipsește [dedus].
9. Ordinea rezultatelor este deterministă (sursă, apoi linie, apoi țintă) [dedus].

## Formate / protocoale
JSON conform schemei oficiale; coduri de control în șiruri sunt scăpate ca în RFC 8259 [observat].

## Cazuri limită și erori
- Sursă fără cale (adresă web) → uri absolut al paginii.
- Căi locale → referință URI cu «file» sau relativă la «originalUriBaseIds»; alegerea este liberă, dar uri trebuie să fie valid [dedus].
- Caractere non-ASCII în adrese → rămân UTF-8 în JSON, procent-codate în uri [dedus].
- Scriere eșuată → cod de ieșire de eroare de I/O, fără fișier parțial [dedus].

## Teste de acceptanță
1. Rulare fără rezultate → JSON valid, version = 2.1.0, runs are un element, results = [].
2. O legătură ruptă pe linia 12 a unei pagini locale → un rezultat cu level = error și startLine = 12.
3. Legătură ruptă fără linie cunoscută → rezultat fără region (startLine absent, nu 0).
4. Document validat cu schema JSON oficială → fără erori, pentru un set cu toate nivelurile.
5. Mesaj cu ghilimele, backslash și linie nouă → JSON parsabil și textul identic după parsare.
6. Toate valorile «level» din document aparțin mulțimii {none, note, warning, error}.
7. Două rulări pe aceleași date → documente identice octet cu octet.

## Necunoscute
- [necunoscut] Ce reguli de grupare (fingerprints) cer platformele pentru deduplicare; câmpul partialFingerprints există în standard dar nu e cerut.

---

## Checklist de filtru (verificare.md)

Spec: 03-raport-sarif.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
