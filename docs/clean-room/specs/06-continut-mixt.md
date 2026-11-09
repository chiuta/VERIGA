# Funcție: Detectarea conținutului mixt (resurse http:// în pagini https://)

Sursă publică: «Mixed Content», W3C Candidate Recommendation — https://www.w3.org/TR/mixed-content/ ; «Secure Contexts», W3C — https://www.w3.org/TR/secure-contexts/ (noțiunea «potentially trustworthy origin») ; «HTML Living Standard» — https://html.spec.whatwg.org/ ; «Fetch Standard» — https://fetch.spec.whatwg.org/
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README nu menționează conținutul mixt (regula de amestec http/https din reparații se referă la adrese, altceva).

## Scop
Semnalează paginile servite prin https care încarcă resurse prin http, pe care browserele le blochează sau le avertizează.

## Intrări / ieșiri
- Intrare: o pagină cu adresa ei, schema ei și lista de legături extrase, cu rolul fiecăreia (navigare sau subresursă).
- Ieșire: pentru fiecare subresursă cu schema http: de pe o pagină https:, un rezultat de tip «conținut mixt» cu categoria «blocabil» sau «opțional blocabil».

## Comportament observabil
1. Se aplică numai dacă pagina însăși e https: [observat].
2. Navigările de nivel superior (a href, area href) nu sunt conținut mixt și nu se raportează [observat: specificația tratează cererile de subresurse]. Un iframe este subresursă, nu navigare de nivel superior [observat].
3. «Opțional blocabil»: imagini, audio și video (elementele img, audio, video) [observat în nivelul 1: «optionally-blockable content»]. Orice altă subresursă (script, link stylesheet/preload, iframe, embed, object, fonturi din CSS, form action la trimitere) este «blocabilă» [observat].
4. O adresă http cu gazdă «potențial de încredere» (localhost, 127.0.0.0/8, [::1]) nu se raportează [observat, Secure Contexts].
5. Subresursele din CSS (url(), @import) moștenesc categoria elementului sau foii de stil care le încarcă: imaginile de fundal sunt opțional blocabile, @import și fonturile sunt blocabile [dedus].
6. Adresele relative sau cu schemă implicită moștenesc schema paginii și nu sunt mixte [observat].
7. Dacă pagina trimite upgrade-insecure-requests (antet CSP) sau browserul ar înnoi cererea, se adaugă o notă «ar fi înnoită», nu se anulează [dedus].

## Formate / protocoale
Schema din RFC 3986, fără distincție majuscule; comparație pe schemă și gazdă, fără port [dedus].

## Cazuri limită și erori
- Pagină http: → nicio verificare mixtă.
- Resursă http care redirecționează la https: tot raportată (cererea inițială e nesigură) [dedus].
- Resursă https care redirecționează la http: raportată cu categoria elementului [dedus].
- srcset: fiecare candidat se evaluează separat [dedus].
- [necunoscut] Dacă browserele moderne tratează audio/video ca blocabile; textul citit păstrează categoria opțional blocabilă, versiunile mai noi pot diferi.

## Teste de acceptanță
1. https://ex.org/p cu img src="http://cdn.ex/a.png" → un rezultat, opțional blocabil.
2. https://ex.org/p cu script src="http://cdn.ex/a.js" → un rezultat, blocabil.
3. https://ex.org/p cu a href="http://alt.ex/" → niciun rezultat.
4. https://ex.org/p cu img src="http://localhost:8080/a.png" → niciun rezultat.
5. http://ex.org/p cu script src="http://cdn.ex/a.js" → niciun rezultat.
6. https://ex.org/p cu CSS care are @import url("http://f.ex/s.css") → un rezultat, blocabil.
7. img cu srcset de două candidate, unul http și unul https → un rezultat.

## Necunoscute
- [necunoscut] Starea finală a specificației (Candidate Recommendation, nu Recommendation); comportamentul poate fi mutat în Fetch.

---

## Checklist de filtru (verificare.md)

Spec: 06-continut-mixt.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
