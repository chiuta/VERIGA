# Funcție: Verificarea atributului de integritate al subresurselor

Sursă publică: «Subresource Integrity», W3C Recommendation — https://www.w3.org/TR/SRI/ ; «HTML Living Standard», atributele integrity și crossorigin — https://html.spec.whatwg.org/multipage/semantics.html#attr-link-integrity ; «RFC 4648 Base Encodings» — https://www.rfc-editor.org/rfc/rfc4648
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: standard public; README nu menționează integrity.

## Scop
Verifică, pentru script și link (stylesheet, preload, modulepreload) cu atributul integrity, că sintaxa e corectă și că octeții descărcați corespund rezumatului declarat.

## Intrări / ieșiri
- Intrare: valoarea integrity, atributul crossorigin, originea paginii și a resursei, octeții resursei descărcate.
- Ieșire: «corespunde», «nu corespunde», «sintaxă invalidă», «fără CORS» sau «niciun algoritm recunoscut».

## Comportament observabil
1. Metadatele = expresii de rezumat separate prin spații ASCII; fiecare = algoritm, cratimă, valoare base64 standard, opțional «?» urmat de opțiuni [observat, SRI §3.2].
2. Algoritmi recunoscuți: sha256, sha384, sha512 [observat]; alții se ignoră [observat]. Dacă niciun algoritm nu e recunoscut, resursa nu e supusă verificării [observat] → rezultat «niciun algoritm recunoscut».
3. Dintre algoritmii recunoscuți se folosește cel mai puternic (sha512 > sha384 > sha256) [observat]; resursa corespunde dacă oricare dintre valorile acelui algoritm egalează rezumatul octeților [observat].
4. Alfabetul base64 este cel standard cu «+», «/» și «=» ca umplutură, nu varianta pentru adrese [observat, RFC 4648 §4].
5. Resursa dintr-o altă origine are nevoie de răspuns permis prin CORS; dacă elementul nu are crossorigin sau serverul nu permite, rezultatul este «fără CORS» [observat].
6. Octeții comparați sunt corpul răspunsului după eliminarea codărilor de transfer și de conținut (gzip etc.) [dedus din cum browserul vede corpul].
7. Verificarea apare doar pentru elementele unde HTML definește integrity: script, link cu rel stylesheet, preload sau modulepreload [observat]. Pe alte elemente se emite o notă [dedus].

## Cazuri limită și erori
- Valoare base64 invalidă sau cu lungime greșită pentru algoritm (sha256 = 32 octeți) → «sintaxă invalidă» [dedus].
- Atribut gol → ignorat [observat: șirul gol nu impune verificare].
- Resursă indisponibilă → se raportează mai întâi eroarea de descărcare.
- Mai multe expresii cu algoritmi diferiți: se aleg cele mai puternice, restul nu contează [observat].

## Teste de acceptanță
Valorile de referință pot fi recalculate cu orice implementare SHA-2.
1. Corpul «abc», integrity="sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=" → corespunde.
2. Același corp, aceeași valoare, dar corpul «abd» → nu corespunde.
3. integrity="md5-AAAA" → «niciun algoritm recunoscut».
4. integrity cu sha256 corect și sha512 greșit, corp «abc» → nu corespunde (se aplică sha512, cel mai puternic) [observat pct. 3].
5. integrity="sha256-QUJD" (3 octeți, nu 32) → «sintaxă invalidă».
6. Resursă dintr-o altă origine, fără crossorigin → «fără CORS».
7. integrity pe un element img → notă «atribut neaplicabil».

## Necunoscute
- [necunoscut] Politica pentru opțiunile după «?»: SRI le rezervă și cere ignorarea lor; se păstrează în raport fără efect.

---

## Checklist de filtru (verificare.md)

Spec: 08-integritate-subresurse.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
