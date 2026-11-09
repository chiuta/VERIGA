# Funcție: Export de raport în format XML de tip JUnit

Sursă publică: Documentația Apache Ant, sarcina «JUnitReport» (descrie structura testsuites/testsuite/testcase) — https://ant.apache.org/manual/Tasks/junitreport.html ; «Extensible Markup Language (XML) 1.0 (Fifth Edition)», W3C Recommendation — https://www.w3.org/TR/xml/
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: formatul cerut de sistemele de integrare continuă; README nu îl are.
Notă: nu există o specificație normativă unică pentru acest format; ce nu e în documentația citită este marcat [necunoscut] sau [dedus].

## Scop
Rularea într-o linie CI să afișeze fiecare verificare ca test reușit/eșuat.

## Intrări / ieșiri
- Intrare: rezultatele unei rulări.
- Ieșire: document XML 1.0, UTF-8, cu rădăcina «testsuites» sau un singur «testsuite».

## Comportament observabil
1. Rădăcină «testsuites» cu un «testsuite» per pagină sursă (sau unul global) [dedus; documentația Ant arată testsuites cu testsuite-uri].
2. «testsuite» are atributele name, tests, failures, errors, skipped (opțional), time, în secunde cu zecimale [observat pentru name/tests/failures/errors/time; skipped dedus].
3. Fiecare legătură verificată este un «testcase» cu «name» = adresa țintă și «classname» = pagina sursă [dedus].
4. Legătură ruptă (4xx/5xx, DNS, TLS) → copil «failure» cu atributele message și type; text: detalii [observat că failure există].
5. Eroare de infrastructură (timeout, rețea indisponibilă) → copil «error» [dedus, distincția failure/error există în documentație].
6. Legătură ignorată de reguli → copil «skipped» [dedus]; legătură bună → fără copil.
7. Invariant: tests = nr. testcase; failures și errors = nr. de copii respectivi [dedus].

## Formate / protocoale
Declarația XML cu codare UTF-8; caracterele ilegale în XML 1.0 (coduri de control în afară de tab, LF, CR) se elimină sau se înlocuiesc; &, <, >, ghilimele în atribute se scapă [observat, W3C XML 1.0 §2.2, §2.4].

## Cazuri limită și erori
- Adrese cu caractere ilegale XML → înlocuite cu U+FFFD [dedus].
- Zero legături → un testsuite cu tests="0".
- Timp necunoscut → time="0".
- [necunoscut] Dacă numele sau classname trebuie să fie unice; nu e cerut, dar duplicatele sunt permise.

## Teste de acceptanță
1. 3 legături: 2 bune, 1 ruptă 404 → tests=3, failures=1, errors=0, un singur copil failure.
2. Legătură cu timeout → un copil error, failures neschimbat.
3. Adresă care conține «&» și «"» → documentul se parsează cu un parser XML 1.0 și numele revine identic.
4. Un octet de control 0x01 într-un mesaj → documentul rămâne XML 1.0 bine format.
5. Zero legături → document valid, tests="0".
6. Suma testcase din toate testsuite-urile = numărul de legături verificate.
7. Două rulări pe aceleași date → fișiere identice.

## Necunoscute
- [necunoscut] Schema exactă acceptată de fiecare sistem CI (variază); se țintește intersecția atributelor de mai sus.

---

## Checklist de filtru (verificare.md)

Spec: 04-raport-junit.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
