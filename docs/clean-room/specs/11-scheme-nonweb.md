# Funcție: Validarea sintaxei pentru schemele tel, geo și data

Sursă publică: «RFC 3966 The tel URI for Telephone Numbers» — https://www.rfc-editor.org/rfc/rfc3966 ; «RFC 5870 A Uniform Resource Identifier for Geographic Locations ('geo' URI)» — https://www.rfc-editor.org/rfc/rfc5870 ; «RFC 2397 The "data" URL scheme» — https://www.rfc-editor.org/rfc/rfc2397 ; «Fetch Standard», data: URL processor — https://fetch.spec.whatwg.org/#data-urls ; «RFC 4648» — https://www.rfc-editor.org/rfc/rfc4648
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README verifică sintaxa mailto: și ignoră alte scheme non-web; aceste scheme rămân neverificate.

## Scop
Legăturile tel:, geo: și data: nu pot fi verificate prin rețea; se verifică doar corectitudinea sintactică, ca erorile de scriere să fie găsite.

## Intrări / ieșiri
- Intrare: o adresă cu una dintre schemele tel, geo, data.
- Ieșire: «validă» sau «invalidă» cu motivul.

## Comportament observabil
**tel** [RFC 3966]
1. Număr global: «+» urmat de cifre și separatori vizuali (cratimă, punct, paranteze) cu cel puțin o cifră [observat].
2. Număr local: cifre (și hexa, «*», «#»), cu obligatoriu parametrul phone-context; lipsa lui face adresa invalidă [observat].
3. Parametri după «;»: ext (doar cifre), isub, phone-context [observat]. Spațiile sunt interzise [observat].

**geo** [RFC 5870]
4. Forma: latitudine, longitudine, altitudine opțională, separate prin virgulă; apoi parametri crs și u [observat].
5. Latitudine în [-90, 90], longitudine în [-180, 180]; la poli (latitudine ±90) longitudinea e ignorabilă [observat]. Numere zecimale cu semn opțional, fără notație științifică [observat].
6. Dacă crs lipsește, se presupune wgs84; crs diferit de wgs84 este acceptat ca informare [observat].

**data** [RFC 2397; Fetch]
7. Forma: «data:» [tip-media] [«;base64»] «,» date; lipsa virgulei → invalidă [observat].
8. Fără tip, valoarea implicită este text/plain cu charset US-ASCII [observat].
9. Cu «;base64», datele se decodează după RFC 4648 §4; Fetch tolerează lipsa umpluturii dacă lungimea nu lasă un rest de 1 modulo 4 [observat în Fetch]; spațiile ASCII se ignoră [observat în Fetch].
10. Fără «;base64», procent-codările trebuie să fie bine formate (%HH) [dedus].

## Cazuri limită și erori
- Schemă cu majuscule (TEL:) → același tratament, schemele sunt insensibile la majuscule [observat, RFC 3986 §3.1].
- Adresă data: foarte mare → plafon de dimensiune pentru analiză; peste plafon, nota «nu s-a verificat» [dedus].
- [necunoscut] Dacă un tip media invalid după regulile RFC 2045 trebuie să fie eroare: Fetch îl înlocuiește cu text/plain, deci este avertisment, nu eroare [observat în Fetch].

## Teste de acceptanță
1. «tel:+1-201-555-0123» → validă.
2. «tel:5551234» → invalidă (lipsă phone-context); «tel:5551234;phone-context=+40» → validă.
3. «tel:+40 721 000 000» (cu spații) → invalidă.
4. «geo:48.2010,16.3695,183» → validă; «geo:91,0» → invalidă; «geo:13.4125,103.8667;u=20» → validă.
5. «data:,A%20brief%20note» → validă; «data:text/plain» (fără virgulă) → invalidă.
6. «data:text/plain;base64,SGVsbG8=» → validă; «data:text/plain;base64,S» → invalidă (rest 1 modulo 4).
7. «data:text/plain;base64,SGVsbG8» (fără umplutură) → validă (tolerată de Fetch).

## Necunoscute
- [necunoscut] Alte scheme comune (sms:, facetime:, whatsapp:) nu sunt acoperite de aceste surse.

---

## Checklist de filtru (verificare.md)

Spec: 11-scheme-nonweb.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
