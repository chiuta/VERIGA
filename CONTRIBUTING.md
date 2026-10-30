# Contributing / Contribuții

**EN.** Thanks for helping. VERIGA is a *clean-room* link checker: do not copy code, assets or text from Xenu Link Sleuth or other proprietary tools; describe behaviour from public documentation and observation only.

1. Build and test: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Web tests: `cd web/tests && npm ci && npm test`.
2. Site: edit `index.html` (Romanian is the source language; English inline with `data-en`; other languages in the `STR` table), then run `python3 web/build-i18n.py` (generates `en/ fr/ it/ es/ pt/ de/ hu/` and `sitemap.xml` and re-stamps the CSP). The web edition is `web/veriga.html` (`python3 web/stamp-csp.py web/veriga.html`, then `python3 web/build-pwa.py`).
3. User-visible messages are bilingual: use `veriga_core::i18n::tr` / `trf!`; add English help text for any new CLI option (a test fails if it is missing).
4. Add a test for every fix; note user-visible changes in `CHANGELOG.md`.
5. Security issues: see `SECURITY.md` (privately, not in a public issue).
6. Commits: short imperative subject; sign off your work as your own.

**RO.** Mulțumim. VERIGA este *clean-room*: nu copia cod, resurse sau texte din Xenu Link Sleuth ori din alte unelte proprietare. Pași: testele și lint-ul de mai sus; pentru site editezi `index.html` (română = sursa, engleza inline, restul în `STR`) și rulezi `python3 web/build-i18n.py`; mesajele vizibile sunt bilingve (`tr` / `trf!`); fiecare reparație are test și intrare în `CHANGELOG.md`; vulnerabilitățile se raportează privat (`SECURITY.md`).
