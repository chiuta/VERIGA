# Funcție: Verificarea legăturilor din antetul HTTP «Link»

Sursă publică: «RFC 8288 Web Linking» — https://www.rfc-editor.org/rfc/rfc8288 ; «RFC 9110 HTTP Semantics» §5.3, §5.6 — https://www.rfc-editor.org/rfc/rfc9110 ; «RFC 3986 URI» §5 — https://www.rfc-editor.org/rfc/rfc3986
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: standard public; README nu menționează antetul Link (extragerea este din HTML/CSS).

## Scop
Legăturile declarate în antetele de răspuns (preîncărcare, pagina următoare, canonică etc.) sunt verificate ca orice altă legătură.

## Intrări / ieșiri
- Intrare: un răspuns HTTP al unei pagini deja vizitate (câmpuri Link) și adresa cererii.
- Ieșire: zero sau mai multe legături găsite, fiecare cu: adresă țintă absolută, tipuri de relație, parametri (title, type, hreflang, media, anchor) și originea «din antet Link», plus avertismente de sintaxă.

## Comportament observabil
1. Mai multe câmpuri Link se combină ca o singură listă separată prin virgule, în ordinea primirii [observat, RFC 9110 §5.3].
2. Fiecare valoare de legătură: adresă între «<» și «>», apoi zero sau mai mulți parametri precedați de «;» [observat, RFC 8288 §3].
3. Adresa se rezolvă față de adresa cererii după RFC 3986 §5 [observat, RFC 8288 §3.1]; dacă există parametrul anchor, contextul este adresa lui rezolvată, dar ținta se verifică la fel [dedus].
4. Parametrul rel conține una sau mai multe tipuri de relație separate prin spații, fără distincție majuscule/minuscule [observat]. Dacă rel apare de mai multe ori, doar prima apariție contează [observat].
5. Valorile parametrilor pot fi token sau șir între ghilimele; în șiruri, virgula și punct-virgula nu separă valori [observat].
6. Parametrul title* (cu codare extinsă) se citește separat de title; dacă ambele există, title* are prioritate [observat, RFC 8288 §3.4.1].
7. Legătura fără rel este păstrată și verificată, cu relația «necunoscută» [dedus].
8. Rezultatul verificării țintei se raportează ca pentru legăturile din HTML, cu pagina sursă = adresa cererii [dedus].

## Formate / protocoale
Câmp: virgule între legături; legătură = «<» referință-URI «>» *( «;» parametru ). Parametru = nume [«=» (token / șir-citat)] [observat].

## Cazuri limită și erori
- Lipsa «>» sau «<» → valoarea se ignoră, restul câmpului se citește mai departe de la următoarea virgulă din afara ghilimelelor; avertisment [dedus].
- Referință goală «<>» → se rezolvă la adresa cererii; nu se verifică a doua oară [dedus].
- Țintă cu schemă neacceptată → tratată ca orice schemă neacceptată din HTML [dedus].
- Număr mare de legături într-un antet → plafon configurabil [dedus].
- [necunoscut] Comportamentul la parametri duplicați alții decât rel: standardul nu îl precizează pentru toți; se păstrează toți.

## Teste de acceptanță
1. «<https://ex.org/a.css>; rel=preload; as=style» → o legătură, rel = preload.
2. «</n>; rel="next", </p>; rel="prev"» pe pagina https://ex.org/d/pag → https://ex.org/n și https://ex.org/p, două legături.
3. «<https://ex.org/x>; rel="alternate stylesheet"; title="a, b; c"» → o singură legătură, două tipuri de relație, title = «a, b; c».
4. Două câmpuri Link separate, câte o legătură → două legături în ordinea primirii.
5. «<https://ex.org/y; rel=next» (fără «>») → 0 legături, 1 avertisment.
6. «<https://ex.org/z>; rel=a; rel=b» → relația este doar «a».
7. Referință relativă «<../s.png>; rel=preload» la https://ex.org/a/b/c → https://ex.org/a/s.png.

## Necunoscute
- [necunoscut] Dacă antetele Link ale redirecționărilor intermediare trebuie verificate; implicit nu.

---

## Checklist de filtru (verificare.md)

Spec: 02-antet-link.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
