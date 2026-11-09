# Funcție: Tratarea tipurilor de relație (atributul rel) la parcurgere

Sursă publică: «HTML Living Standard», §4.6.7 «Link types» și §4.6.6 «Link type "nofollow"» — https://html.spec.whatwg.org/multipage/links.html#linkTypes ; «RFC 8288 Web Linking» — https://www.rfc-editor.org/rfc/rfc8288
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README oferă filtre prin expresii regulate («don't follow»), nu interpretează rel.

## Scop
Utilizatorul poate cere ca legăturile marcate de autor cu nofollow să fie verificate, dar fără a parcurge mai departe pagina țintă; raportul indică și tipurile de relație ale fiecărei legături.

## Intrări / ieșiri
- Intrare: valoarea atributului rel de pe elementele a, area, link și form (dacă ultimul este activat).
- Ieșire: mulțimea de tipuri de relație a legăturii și, cu opțiunea activă, o decizie «verifică și parcurge» / «doar verifică».

## Comportament observabil
1. Valoarea rel este un set de jetoane separate prin spații ASCII, comparate fără distincție majuscule/minuscule, fără ordine și fără duplicate [observat, HTML §2.3.8 «set of space-separated tokens» și §4.6.7].
2. Jetoane recunoscute pentru raport: alternate, canonical, external, icon, next, prev, nofollow, noopener, noreferrer, preload, stylesheet, author, license, help, search [observat că sunt tipuri definite sau înregistrate; lista este aleasă de mine]. Jetoanele necunoscute se păstrează în raport fără efect [dedus].
3. nofollow indică că autorul nu aprobă ținta [observat]; standardul nu cere crawlerelor un comportament. Pentru acest verificator, cu opțiunea activă, ținta se verifică (stare HTTP), dar conținutul ei nu se extrage pentru alte legături [dedus]. Fără opțiune, nu se schimbă nimic față de azi [dedus].
4. Pe elementele a și area, rel are efect numai dacă elementul are href [observat].
5. Pe elementul link, rel este obligatoriu pentru ca legătura să aibă sens; fără rel, link nu creează o relație [observat, HTML §4.2.4].
6. noopener/noreferrer nu afectează verificarea; apar doar în raport [dedus].

## Formate / protocoale
Atribut rel = jetoane unice separate prin spații ASCII (spațiu, tab, LF, FF, CR) [observat].

## Cazuri limită și erori
- Valoare goală sau doar spații → mulțime vidă.
- Jeton cu majuscule mixte, de pildă «NoFollow» → același cu nofollow [observat].
- Același jeton repetat → o singură apariție.
- rel pe form: numai dacă acțiunile formularelor sunt verificate [dedus].
- [necunoscut] Dacă un meta robots cu nofollow la nivel de pagină trebuie respectat; standardul HTML nu îl definește, deci nu face parte din acest spec.

## Teste de acceptanță
1. «rel="nofollow"», opțiune activă → țintă verificată, legăturile din ea neextrase.
2. «rel="NoFollow noopener"» → mulțime {nofollow, noopener}; același efect ca 1.
3. «rel="  "» → mulțime vidă, legătură normală.
4. «rel="nofollow nofollow"» → un singur element în mulțime.
5. Opțiune inactivă, «rel="nofollow"» → țintă verificată și parcursă, ca azi.
6. «a» fără href, cu rel="nofollow" → nu produce legătură.
7. «link rel="stylesheet nofollow"» → legătura către foaia de stil se verifică; jetonul nofollow apare în raport.

## Necunoscute
- [necunoscut] Dacă rel="ugc" și rel="sponsored" trebuie tratate; nu sunt definite în standardul HTML citit, deci rămân simple jetoane necunoscute.

---

## Checklist de filtru (verificare.md)

Spec: 05-relatii-legatura.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
