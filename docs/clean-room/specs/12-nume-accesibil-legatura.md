# Funcție: Controlul numelui accesibil al legăturilor

Sursă publică: «Web Content Accessibility Guidelines (WCAG) 2.2», W3C Recommendation, criteriile 2.4.4 «Link Purpose (In Context)» și 4.1.2 — https://www.w3.org/TR/WCAG22/ ; «Accessible Name and Description Computation 1.2», W3C — https://www.w3.org/TR/accname-1.2/ ; «HTML Living Standard», alt și title — https://html.spec.whatwg.org/
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README menționează doar WCAG pentru interfața web a aplicației, nu o verificare a legăturilor analizate.

## Scop
Găsește legăturile pe care un cititor de ecran le-ar anunța fără nume (de pildă o imagine fără alt într-un link), semnalând astfel de legături ca avertismente de accesibilitate.

## Intrări / ieșiri
- Intrare: elementele a cu href dintr-o pagină HTML.
- Ieșire: pentru fiecare, numele accesibil calculat și, dacă este vid, o constatare; opțional o constatare de ambiguitate.

## Comportament observabil
1. Numele se calculează în ordinea: aria-labelledby (textul elementelor referite), aria-label nevid, conținutul (text și nume ale descendenților), apoi atributul title ca ultim recurs [observat, Accname 1.2 §4.3].
2. Un descendent img contribuie cu alt; alt="" îl face decorativ, fără contribuție [observat, HTML]. Imaginea fără atribut alt contribuie cu title, dacă există [dedus].
3. Conținutul cu aria-hidden="true" sau ascuns (atributul hidden) nu contribuie [observat, Accname 1.2].
4. Spațiile se normalizează: succesiunile devin un singur spațiu, capetele se taie [observat].
5. Nume vid → constatare «legătură fără nume» (se leagă de WCAG 2.4.4 și 4.1.2) [observat că criteriile cer scopul și numele determinabile].
6. Nume compus doar din semne de punctuație sau simboluri → constatare mai slabă «nume nesemnificativ» [dedus].
7. Ambiguitate (opțional): aceeași pagină are legături cu nume identic (fără distincție de majuscule) spre adrese diferite → notă informativă; WCAG 2.4.4 admite că contextul poate lămuri, deci nu e eroare [observat că contextul contează].
8. Elementele a fără href nu sunt legături și se omit [observat].

## Cazuri limită și erori
- Conținut generat de CSS (pseudo-elemente) nu este vizibil analizei; nota «nu s-a putut evalua» nu se emite [dedus].
- aria-labelledby spre id inexistent → se trece la pasul următor [observat].
- Referințe circulare în aria-labelledby → fiecare element contribuie cel mult o dată [observat].
- [necunoscut] Lista cuvintelor generice precum «aici» sau «mai mult» depinde de limbă; nu face parte din acest spec.

## Teste de acceptanță
1. a href cu text «Prețuri» → nume «Prețuri», fără constatare.
2. a href cu doar img alt="" → nume vid, constatare.
3. a href cu doar img alt="Acasă" → nume «Acasă».
4. a href cu aria-label="Contact" și text «→» → nume «Contact».
5. a href cu span aria-hidden="true" ca unic conținut și title="Ajutor" → nume «Ajutor» (title ca ultim recurs).
6. a href cu textul «  ...  » → nume nesemnificativ.
7. Două a cu textul «Detalii» spre /x și /y → o notă de ambiguitate; spre aceeași adresă → fără notă.

## Necunoscute
- [necunoscut] Tratamentul legăturilor cu content-visibility sau stil display:none fără atribut: depinde de CSS și nu este evaluat.

---

## Checklist de filtru (verificare.md)

Spec: 12-nume-accesibil-legatura.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
