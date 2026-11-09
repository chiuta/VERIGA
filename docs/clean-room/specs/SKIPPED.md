# Candidate omise (lotul 1)

| Candidat | Motiv |
|---|---|
| Detectare „soft 404” | Nu există standard public; euristică fără sursă normativă. |
| Respectarea X-Robots-Tag / meta robots (noindex) | X-Robots-Tag nu este într-un standard public (RFC 9309 nu îl definește); meta robots nu e definit de HTML Living Standard. |
| Redarea JavaScript | Deja declarat ca limită în README; nu e funcție de specificat dintr-un standard de legături. |
| Extragere din PDF | Surse de specificare neaccesibile liber în lotul acesta (ISO 32000 e cu plată); se poate reevalua cu specificația Adobe publică. |
| Extensii de sitemap (imagine, video, știri) | Definite de un furnizor de motor de căutare, nu de sitemaps.org; în afara regulilor de surse. |
| Reciprocitatea hreflang și x-default | Convenții ale unui motor de căutare, nu RFC 5646 / HTML. Rămân note în spec 07. |
| Verificarea revocării certificatelor (OCSP/CRL) | Sursă publică există (RFC 6960, 5280), dar cere acces de rețea separat; amânat, mărime L. |
| HSTS (RFC 6797) ca raport | Valoare mică pentru un verificator de legături; amânat. |
| rel="ugc" / rel="sponsored" | Nu sunt în HTML Living Standard; tratate ca jetoane necunoscute în spec 05. |
| Linkuri din CommonMark/Markdown la parcurgere | Modulul de reparații citește deja Markdown; extinderea parcurgerii cere decizie de produs. |
| Format de raport XUnit/TAP | Alternative la 04 fără surse normative proprii. |
