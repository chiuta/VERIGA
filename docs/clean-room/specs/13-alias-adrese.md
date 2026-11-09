# Funcție: Detectarea adreselor echivalente scrise diferit

Sursă publică: «RFC 3986 URI: Generic Syntax», §6 «Normalization and Comparison» — https://www.rfc-editor.org/rfc/rfc3986#section-6 ; «RFC 9110», §4.2.3 «http and https normalization and comparison» — https://www.rfc-editor.org/rfc/rfc9110#section-4.2.3
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README folosește adrese normalizate pentru parcurgere (fragmentul se ignoră) fără a raporta adrese echivalente scrise diferit; acest spec cere raportarea.

## Scop
Raportează grupuri de adrese scrise diferit dar echivalente (de pildă majuscule în gazdă, port implicit explicit, procent-codare inutilă, segmente cu puncte), ca să fie unificate în conținut și să nu fie verificate de două ori.

## Intrări / ieșiri
- Intrare: mulțimea adreselor absolute întâlnite, în forma scrisă de autor.
- Ieșire: grupuri cu 2 sau mai multe forme scrise distincte, cu forma normalizată comună și locurile unde apar.

## Comportament observabil
1. Normalizarea sintactică: schema și gazda cu litere mici [observat, RFC 3986 §6.2.2.1]; hexazecimalele din procent-codări cu majuscule [observat]; decodarea caracterelor neprotejate (litere, cifre, «-», «.», «_», «~») codate procentual [observat, §6.2.2.2]; eliminarea segmentelor «.» și «..» [observat, §6.2.2.3].
2. Normalizarea pe schemă: portul implicit (80 pentru http, 443 pentru https) se elimină [observat, §6.2.3; RFC 9110 §4.2.3]; calea goală devine «/» pentru http și https [observat]; gazda goală și «localhost» nu se unifică [dedus].
3. Fragmentul se exclude din comparația de echivalență a resursei [observat, §6.2.3 evită fragmentul la dereferențiere; RFC 9110 §4.2.3].
4. Nu se unifică: litere mari/mici în cale sau interogare, ordinea parametrilor interogării, prezența sau absența barei finale, «www.» față de fără [observat că aceste forme nu sunt echivalente sintactic; dedus că nu se tratează ca alias].
5. Fiecare grup alias se raportează ca informare, nu ca eroare; numărul de cereri se reduce la una pe grup [dedus].
6. Comparația gazdelor internaționalizate se face după conversia în forma ASCII (Punycode) [dedus; standardul URL WHATWG/IDNA, fără a fi în sursele citite → necunoscut].

## Cazuri limită și erori
- Adresă neparsabilă → se omite din grupare, rămâne la raportul de eroare existent.
- «%2F» în cale nu se decodează (delimitator rezervat) [observat, §2.2, §6.2.2.2].
- Umplere cu «..» dincolo de rădăcină → rămâne la rădăcină [observat, §5.2.4].

## Teste de acceptanță
1. «HTTP://Ex.ORG/a» și «http://ex.org/a» → un grup alias.
2. «http://ex.org:80/a» și «http://ex.org/a» → un grup; «https://ex.org:443/» și «https://ex.org» → un grup.
3. «http://ex.org/%7Euser» și «http://ex.org/~user» → un grup; «http://ex.org/%7euser» tot același grup.
4. «http://ex.org/a/b/../c» și «http://ex.org/a/c» → un grup.
5. «http://ex.org/A» și «http://ex.org/a» → grupuri separate.
6. «http://ex.org/a%2Fb» și «http://ex.org/a/b» → grupuri separate.
7. «http://ex.org/a#x» și «http://ex.org/a#y» → aceeași resursă; nu e raportat ca alias de formă, ci unificat în tăcere.

## Necunoscute
- [necunoscut] Tratamentul gazdelor IDN: necesită sursa standardului URL WHATWG, neinclusă în acest spec.

---

## Checklist de filtru (verificare.md)

Spec: 13-alias-adrese.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
