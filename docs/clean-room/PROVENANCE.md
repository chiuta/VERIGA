# Proveniență și clean-room — VERIGA (chiuta/VERIGA)

Document de completat cu fapte reale. Nu declara clean-room pentru ce nu s-a întâmplat așa.

## Ce a existat până acum (completează onest)
- Ținta/țintele: Xenu's Link Sleuth
- Cum s-a documentat comportamentul țintei (documentație publică, utilizare, formate observate, decompilare?): ______
- A fost folosit vreun cod, pseudocod, decompilare sau asset al țintei în sesiunile de dezvoltare? DA / NU / NU ȘTIU: ______
- Sesiunile de dezvoltare au avut vreodată acces la binarul sau codul țintei? ______

## Risc de licență
Freeware proprietar (nu open source). Verifică EULA pentru clauze de reverse engineering. Replicarea funcționalității din comportament observabil este abordarea sigură.

## Dependențe terțe
Rulează `cargo license` (sau `cargo deny check licenses`) și atașează rezultatul. Verifică compatibilitatea cu licența proiectului.

## Pentru lucrul de acum înainte
Folosește fluxul din acest kit (1-analiza → 2-filtru → 3-implementare). Orice funcție nouă inspirată de o țintă trece prin specificație și filtru.

## Recomandare
Pentru lansări comerciale sau donații semnificative, cere o opinie de la un avocat de proprietate intelectuală și fă căutare de mărci (EUIPO), cum ai făcut pentru OSATURA.

## Constatări din audit automat (2026-11-08, doar observații)
- Numele „Xenu” apare doar într-un comentariu (`veriga-core/src/config.rs`: terminologie pentru „threads”) și în `CONTRIBUTING.md` (interdicția de a copia cod/resurse). Nu s-au găsit alte urme ale țintei în sursă.
- 490 de dependențe, 39 de licențe distincte. Singurele expresii cu GPL/LGPL sunt duale și oferă alternativă permisivă: `self_cell` (Apache-2.0 OR GPL-2.0-only), `r-efi` (MIT OR Apache-2.0 OR LGPL-2.1-or-later).
- Nicio dependență fără licență declarată.

**Limite ale auditului:** căutarea după nume nu poate detecta cod copiat fără nume. Rămân de răspuns întrebările din secțiunea „Ce a existat până acum".
