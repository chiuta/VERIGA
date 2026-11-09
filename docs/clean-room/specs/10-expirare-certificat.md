# Funcție: Avertizare la certificate TLS expirate sau aproape de expirare

Sursă publică: «RFC 5280 Internet X.509 PKI Certificate and CRL Profile», §4.1.2.5 «Validity» — https://www.rfc-editor.org/rfc/rfc5280 ; «RFC 9110 HTTP Semantics», §4.3.4 (verificarea identității) — https://www.rfc-editor.org/rfc/rfc9110 ; «RFC 6125» (potrivirea numelui de gazdă) — https://www.rfc-editor.org/rfc/rfc6125
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README oferă doar opțiunea de a accepta certificate invalide, fără raport de expirare.

## Scop
Pe lângă legături rupte, utilizatorul află ce gazde https folosesc certificate care expiră curând, ca să le înnoiască înainte de întreruperi.

## Intrări / ieșiri
- Intrare: certificatul frunză prezentat de o gazdă https în timpul verificării; ora curentă; un prag în zile (implicit 14 [dedus]).
- Ieșire: per pereche gazdă:port o singură constatare: «expirat», «încă nevalid», «expiră în N zile» sau nimic.

## Comportament observabil
1. Valabilitatea = interval închis [notBefore, notAfter], ambele în UTC [observat, RFC 5280 §4.1.2.5].
2. Datele până în 2049 sunt UTCTime (YYMMDDHHMMSSZ, anii 50–99 → 19xx, 00–49 → 20xx), de la 2050 GeneralizedTime (YYYYMMDDHHMMSSZ) [observat].
3. Dacă acum > notAfter → «expirat»; dacă acum < notBefore → «încă nevalid»; altfel zilele rămase = notAfter minus acum, rotunjite în jos; dacă ≤ prag → «expiră în N zile» [dedus].
4. Se evaluează o singură dată pe gazdă:port, indiferent de numărul de legături [dedus]; constatarea listează numărul de legături afectate [dedus].
5. Cu opțiunea de acceptare a certificatelor invalide, verificarea se face oricum, iar constatarea e informativă [dedus].
6. Nepotrivirea numelui de gazdă cu numele alternative ale subiectului (SAN) este o constatare separată «nume nepotrivit» [observat că identitatea se verifică în RFC 9110 §4.3.4 și RFC 6125; separarea e dedusă].
7. Gazdele http: sau cu conexiune eșuată înainte de handshake nu au constatare.

## Cazuri limită și erori
- Certificat fără interval parsabil → constatare «certificat ilizibil».
- Ceasul sistemului greșit → avertisment generic: nu se poate detecta.
- Proxy care termină TLS (de pildă SOCKS cu transport opac) → certificatul vizibil e cel al destinației; cu proxy care interceptează, constatarea privește certificatul proxy-ului [dedus].
- Prag 0 → doar «expirat».

## Teste de acceptanță
Testele folosesc certificate de probă cu date alese.
1. notAfter = ieri → «expirat».
2. notAfter = peste 10 zile, prag 14 → «expiră în 10 zile»; prag 5 → nicio constatare.
3. notBefore = mâine → «încă nevalid».
4. Zece legături pe aceeași gazdă:port cu certificat aproape expirat → o constatare, cu 10 legături afectate.
5. notAfter codat GeneralizedTime «20510101000000Z» → interpretat ca 1 ianuarie 2051.
6. UTCTime «490101000000Z» → 2049; «500101000000Z» → 1950.
7. Certificat valabil 90 zile, prag 14 → nicio constatare.

## Necunoscute
- [necunoscut] Dacă pragul implicit de 14 zile este potrivit utilizatorilor; nu provine dintr-un standard.
- [necunoscut] Verificarea revocării (OCSP/CRL) nu este acoperită.

---

## Checklist de filtru (verificare.md)

Spec: 10-expirare-certificat.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
