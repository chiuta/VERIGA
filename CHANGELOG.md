# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/ro/1.1.0/) · versionare [SemVer](https://semver.org/lang/ro/).

## [Nepublicat / Unreleased]

### Securitate
- Anti-SSRF implicit (`--allow-private`), antete sensibile limitate la originea internă, filtru de injecție pentru URL-uri inserate, CSV fără injecție de formule, CSP cu hash în varianta web și în rapoartele HTML.
### Remedieri GUI
- Pictogramele din butoane apăreau ca pătrățele după portarea la egui 0.37 (fonturile de emoji sunt acum activate, iar săgețile → ↶ ↳ au font de rezervă).
- Clicul pe un rând din tabel nu selecta rândul (etichetele selectabile interceptau clicul): selecția și panoul de detalii funcționează din nou.
### Pagina de prezentare
- Capturi de ecran ale aplicației native (RO/EN), bară de meniu compactă și lipicioasă pe telefon, 0 încălcări axe-core.
### Standarde
- Rust edition 2024, MSRV 1.88 (GUI 1.95); `clippy -D warnings` curat.
- robots.txt conform RFC 9309 (grupuri îmbinate, normalizare procentuală, plafon 500 KiB, 5xx ⇒ interdicție totală).
- Dependențe actualizate: reqwest 0.13, scraper 0.29, thiserror 2, egui/eframe 0.37, rfd 0.17.
- Web: WCAG 2.2 AA (axe-core: 0 încălcări), tab-uri WAI-ARIA cu tastatură, `prefers-reduced-motion`, `forced-colors`.
- Lanț de aprovizionare: `cargo-audit` + `cargo-deny` (`deny.toml`), `LICENSE` (MIT), `SECURITY.md`, Dependabot.
