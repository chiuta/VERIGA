# Funcție: Citirea hărților de site comprimate (gzip)

Sursă publică: «sitemaps.org Protocol» — https://www.sitemaps.org/protocol.html ; «RFC 1952 GZIP file format specification 4.3» — https://www.rfc-editor.org/rfc/rfc1952 ; «RFC 9110 HTTP Semantics» (Content-Encoding) — https://www.rfc-editor.org/rfc/rfc9110
Legendă: [observat] = enunțat explicit în sursa publică citată; [dedus] = decizie sau inferență proprie pentru un verificator de legături; [necunoscut] = neprecizat de sursă.
Origine candidat: README.md, «Known limits» (fișierele .xml.gz nu sunt citite).

## Scop
Ca sitemap-urile și indexurile de sitemap livrate comprimat să poată hrăni verificarea cu adrese de pornire, la fel ca cele necomprimate.

## Intrări / ieșiri
- Intrare: octeții unui răspuns (sau fișier local) indicat ca sitemap sau ca intrare într-un index de sitemap. Adresa poate sfârși în .xml.gz, dar nu obligatoriu.
- Ieșire: aceeași listă de adrese pe care ar da-o documentul XML decomprimat (elementele de adresă dintr-un set de adrese sau dintr-un index), ori o eroare descriptivă.

## Comportament observabil
1. Protocolul permite comprimarea gzip a fișierelor de sitemap [observat]. Un index poate trimite către sitemap-uri comprimate [observat].
2. Un conținut se consideră gzip dacă începe cu octeții 0x1F 0x8B [observat, RFC 1952 §2.3.1]; extensia adresei nu decide [dedus].
3. Conținutul decomprimat se tratează apoi ca XML UTF-8 [observat, sitemaps.org cere UTF-8].
4. Dacă serverul a aplicat deja Content-Encoding gzip la nivel de transport, decodarea la nivel HTTP are loc o singură dată; octeții rezultați pot fi la rândul lor un fișier gzip, tratat ca la pct. 2 [dedus].
5. Limita protocolului: cel mult 50.000 de adrese și 50 MB (decomprimat) per fișier [observat]. Peste limita de dimensiune decomprimarea se oprește și fișierul este refuzat (protecție contra bombelor de decomprimare) [dedus]. Peste 50.000 de adrese: se citesc primele 50.000 și se emite avertisment [dedus].
6. Un fișier gzip poate avea mai multe membri concatenați; se decomprimă toți [observat, RFC 1952 §2.2].

## Formate / protocoale
Antet gzip: 10 octeți fixi (ID1=0x1F, ID2=0x8B, CM=8, FLG, MTIME, XFL, OS), câmpuri opționale după FLG, date deflate, apoi CRC32 și ISIZE (mod 2^32), ambele little-endian [observat].

## Cazuri limită și erori
- CRC32 sau ISIZE nepotrivite → fișier corupt, eroare, nicio adresă din el [observat că CRC există; consecința este dedusă].
- Gzip trunchiat → eroare; adresele deja extrase nu se păstrează [dedus].
- Octeți care nu sunt gzip dar adresa se termină în .gz → se încearcă XML simplu; dacă nici acesta nu e valid, eroare [dedus].
- Index recursiv (un index care se referă la el însuși sau la alt index) → adâncime maximă și detectare de cicluri [necunoscut ce limită impune protocolul; sitemaps.org nu permite indexuri imbricate, dedus].

## Teste de acceptanță
1. Set de adrese XML cu 2 adrese, comprimat gzip, servit la o adresă .xml.gz → 2 adrese.
2. Același conținut servit la o adresă fără extensie .gz, cu octeții 1F 8B la început → 2 adrese.
3. Index cu un sitemap indicat ca .xml.gz care conține 3 adrese → 3 adrese în total.
4. Conținut gzip cu ultimul octet CRC32 modificat → eroare de integritate, 0 adrese.
5. Fișier care se decomprimă la mai mult de 52.428.800 de octeți → refuzat, fără a încărca tot în memorie.
6. Două membri gzip concatenați, fiecare cu un set de adrese complet → comportament stabilit de pct. 6 (concatenarea decomprimată nu e XML valid, deci eroare clară) [dedus].
7. Răspuns HTTP cu Content-Encoding gzip pe un .xml → o singură decodare, rezultat identic cu necomprimatul.

## Necunoscute
- [necunoscut] Dacă sitemaps.org permite gzip imbricat în gzip: nu spune; tratat ca nepermis dincolo de pct. 4.
- [necunoscut] Politica exactă pentru peste 50.000 de adrese în practică.

---

## Checklist de filtru (verificare.md)

Spec: 01-sitemap-comprimat.md   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă
- [x] Niciun text de UI, asset sau branding din țintă
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut]
- [x] Licența/EULA sursei verificată și notată (în `docs/clean-room/specs/JURNAL.md`, nu în `1-analiza/jurnal.md`)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu am verificat brevete; în caz de dubiu: consultă un avocat de PI)
- [ ] Aprobată de mine înainte de predare
