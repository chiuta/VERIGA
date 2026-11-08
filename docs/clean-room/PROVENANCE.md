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

## Constatări din audit adversarial (2026-11-08) — fapte verificate în repo

**Corectură:** o versiune anterioară a acestei secțiuni spunea că „Xenu" apare doar într-un comentariu și în `CONTRIBUTING.md`. Era greșit; auditul inițial a căutat doar în cod. Mențiunile reale sunt mai jos.

### Istoric
- Tot proiectul a intrat într-un singur commit: `07f1b35` (2026-10-27, 42 de fișiere, 18.299 de linii). Nu există istoric incremental care să dovedească evoluția independentă a codului.
- Commit-urile poartă `Co-Authored-By: Claude`: codul a fost scris în mare parte cu un model AI. Titularul drepturilor asupra părții generate și eventualele influențe din datele de antrenare ale modelului nu pot fi stabilite din repo.
- Licența a fost schimbată din `MIT OR Apache-2.0` în MIT în `9093b22` (2026-10-28), la o zi după 1.0.0. Copiile obținute înainte sub Apache-2.0 rămân valabile sub acel termen.
- Kitul clean-room din `docs/clean-room/` a fost adăugat după fapt (2026-11-08). Folderele `evidence/`, `candidat/`, `spec/` și `src/` sunt goale: **nu a existat separare analiză/implementare și nicio specificație filtrată**.

### Mențiuni ale țintei (15 fișiere)
`CONTRIBUTING.md`, `config.rs:39` („Xenu: threads"), `index.html` și cele 7 variante lingvistice (câte 2 apariții; ex. „replică clean-room a funcțiilor Xenu Link Sleuth"), `Dockerfile`, `packaging/docker/Dockerfile.release`, `packaging/deb/build.sh`, `packaging/yunohost/manifest.toml`, `packaging/yunohost/doc/DESCRIPTION.md`.

### Constatări
- **Termenul „clean-room" nu e susținut în sens strict.** Formularea apărabilă pe fapte: *implementare independentă, fără acces la codul sau binarul Xenu*. Nu pretinde mai mult decât poți dovedi.
- **„Replică a funcțiilor Xenu"** (site, metadate de pachet) arată că setul de funcții al țintei a fost urmărit și folosește numele unui produs terț în metadate comerciale. Recomandare: descrie funcția („link checker"), elimină „Xenu-like" din pachete, menționează Xenu cel mult factual și cu precizarea că nu ești afiliat.
- `threads: 30` (`config.rs:108`) pare să coincidă cu valoarea implicită a Xenu din cunoștințele auditorului; neverificat din sursă oficială. Valoare generică, risc scăzut.
- Coloanele CSV/TSV din `report.rs` (Address, Status-Code, Status, Type, …) nu au putut fi comparate cu un export Xenu real. De verificat manual doar ca output; dacă ordinea e identică, schimb-o.
- Nu s-au găsit antete de copyright străine, comentarii „adapted from/ported from", URL-uri către cod terț sau texte/asset-uri Xenu.
- Fonturile din `assets/fonts/` sunt terțe sub OFL-1.1; `portableapps/.../readme.txt:19` le descrie greșit ca „proprii". De corectat.
- Pachetele (.deb, portable, Docker) nu includ note de licență pentru dependențe (Apache-2.0, BSD, ISC, MPL, OFL cer păstrarea notelor). Recomandare: generează `THIRD-PARTY-LICENSES` (`cargo about`) și include-l.
- Dependențe: 490 de pachete externe, toate cu licență declarată; expresiile cu GPL/LGPL (`self_cell`, `r-efi`) sunt duale cu alternativă permisivă; câteva crate-uri MPL-2.0 (copyleft la nivel de fișier).
- Funcțiile din Trade-Free Link Guardian: README spune că lista a fost reconstruită din descrierea publică și „conversații anterioare", nu din codul pluginului. Documentează titularul și licența pluginului original.

### Completat din istoricul conversațiilor Claude (2026-11-08)
Sursa: conversația claude.ai 385f3bdf-… (dezvoltare VERIGA) și 
ec6ab1f3-… (plugin Trade-Free Link Guardian). Nu au fost citite integral.
- **Acces la Xenu:** în porțiunile citite, sesiunea a pornit doar de la cererea titularului și de la o verificare a toolchain-ului. Nu apare descărcare, rulare sau decompilare a Xenu ori citirea documentației lui. Absența nu este dovedită pentru ture necitite. Titularul confirmă: ______
- **EULA Xenu:** nu apare în istoricul citit că ar fi fost citit. Neconfirmat; de verificat de titular: ______
- **Trade-Free Link Guardian:** plugin construit pentru titular (Alexio) de Claude în conversații din septembrie 2026; proprietar de drepturi: titularul, în măsura în care nu există contribuții terțe. Din declarațiile titularului: pluginul a fost comandat ca „link checker local, trade-free, care repară linkurile", rulează pe site-ul CNRED și se urmărește publicarea pe WordPress.org (care cere licență compatibilă GPL). Atenție: dacă s-a portat COD (nu doar funcții) din plugin în VERIGA (MIT), licențele trebuie să fie compatibile, iar rularea pe site-ul instituției poate ridica o chestiune de drepturi față de angajator. Licența plugin-ului și titularul de drept: ______

**Limite:** nu s-a avut acces la Xenu; căutarea după nume nu detectează copierea fără nume. Nu este opinie juridică.
