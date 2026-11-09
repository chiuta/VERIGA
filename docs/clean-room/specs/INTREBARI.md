# Întrebări și rezerve din implementare (lot 1)

- Spec 03, test 4 (validare cu schema JSON oficială SARIF): neautomatizat — fără rețea și fără o dependență de validare JSON Schema; testele verifică structural părțile obligatorii (version, runs, tool.driver.name, level, message.text, startLine >= 1). De rulat manual cu schema oficială.
- Spec 03, pct. 6 (startLine): rezultatele parcurgerii nu poartă numărul liniei, deci `region` lipsește în practică; generatorul îl scrie când o constatare are linie (testat la nivel de constatare).
- Spec 08: verdictul se calculează per pereche (pagină, resursă), dar rezumatul există numai dacă prima legătură către resursă (sau una văzută cât timp sarcina era în coadă) cerea verificarea `integrity`; altfel verdictul este «neverificat». Specificația nu spune cum se tratează aceeași resursă cu metadate diferite pe pagini diferite.
