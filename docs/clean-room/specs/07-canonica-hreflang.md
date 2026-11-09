# Funcție: Verificarea legăturilor canonice și a etichetelor de limbă alternativă

Sursă publică: «RFC 6596 The Canonical Link Relation» — https://www.rfc-editor.org/rfc/rfc6596 ; «RFC 5646 Tags for Identifying Languages (BCP 47)» — https://www.rfc-editor.org/rfc/rfc5646 ; «HTML Living Standard», atributul hreflang și rel=alternate — https://html.spec.whatwg.org/multipage/links.html
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: standarde publice; README nu verifică semantica rel=canonical sau hreflang.

## Scop
Detectează pagini cu legătură canonică spre o țintă care nu răspunde bine și etichete hreflang scrise greșit.

## Intrări / ieșiri
- Intrare: elementele link cu rel canonical sau alternate dintr-o pagină și rezultatele verificării țintelor.
- Ieșire: constatări cu motiv: canonică ruptă, canonică redirecționată, canonice multiple, etichetă de limbă invalidă.

## Comportament observabil
1. Canonica = link rel="canonical" href=... ; ținta se rezolvă față de baza documentului [observat, RFC 6596 §3; HTML pentru bază].
2. Ținta canonică trebuie să răspundă cu succes (2xx) [dedus din scopul relației: «URI preferat»]; 3xx, 4xx, 5xx → constatare. 3xx se raportează separat de rupt [dedus].
3. Mai multe canonice diferite într-o pagină → constatare; RFC 6596 definește o singură adresă canonică per resursă [observat că relația indică adresa preferată; interdicția exactă este deducție].
4. Canonica indicând adresa paginii însăși este permisă și nu produce constatare [dedus].
5. hreflang pe link rel="alternate" și pe a: valoarea trebuie să fie etichetă de limbă bine formată conform gramaticii RFC 5646 §2.1 [observat în HTML: «a valid BCP 47 language tag»]. Verificarea e sintactică, fără registru IANA [dedus].
6. Gramatică sumară: limbă 2–3 litere (+ extensii) sau 4 litere rezervate sau 5–8 litere; apoi opțional scriptul (4 litere), regiunea (2 litere sau 3 cifre), variante (5–8 alfanumerice sau 4 începând cu cifră), extensii (o literă sau cifră, nu x, apoi 2–8 alfanumerice), utilizare privată (x- urmat de 1–8 alfanumerice) [observat, RFC 5646 §2.1]. Separator doar cratima [observat].
7. Comparația nu distinge majuscule [observat, RFC 5646 §2.1.1].
8. Valoarea «x-default» nu este definită de RFC 5646 sau HTML; nu se raportează ca invalidă, dar este marcată «convenție externă» [dedus, necunoscut ca standard].

## Cazuri limită și erori
- hreflang gol → invalid.
- Etichetă cu underscore, de pildă en_US → invalidă.
- Etichete gramaticale dar neînregistrate (de pildă «english», 7 litere) → sintactic valide, fără constatare de sintaxă [observat că gramatica admite 5–8 litere].
- Reciprocitatea (pagina de limbă alternativă trimite înapoi) nu este cerută de standardele citite și nu se verifică [necunoscut/dedus].

## Teste de acceptanță
1. canonical către o pagină 200 → fără constatare.
2. canonical către o adresă 404 → constatare «canonică ruptă».
3. canonical către o adresă cu 301 → constatare «canonică redirecționată», distinctă de rupt.
4. hreflang="zh-Hant-TW" → valid; "de-DE-1996" → valid.
5. hreflang="en_US" și hreflang="e" și hreflang="" → toate invalide.
6. hreflang="english" → valid sintactic.
7. Două canonice cu adrese diferite → constatare «canonice multiple».

## Necunoscute
- [necunoscut] Cum tratează motoarele de căutare canonica de pe pagini cu parametri; în afara standardelor.

---

## Checklist de filtru (verificare.md)

Spec: 07-canonica-hreflang.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
