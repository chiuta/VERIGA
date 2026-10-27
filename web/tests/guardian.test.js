const { JSDOM, VirtualConsole } = require('jsdom');
const fs = require('fs');
const html = fs.readFileSync(require('path').join(__dirname, '..', 'veriga.html') + '', 'utf8');
let fails = 0; const ok = (c, m) => { if (!c) { fails++; console.log('FAIL:', m); } else console.log('ok  :', m); };
(async () => {
  const vc = new VirtualConsole(); vc.on('jsdomError', e => { fails++; console.log('JSDOM ERROR', e.message); });
  const dom = new JSDOM(html, { runScripts: 'dangerously', pretendToBeVisual: true, url: 'http://localhost/', virtualConsole: vc });
  const w = dom.window; await new Promise(r => setTimeout(r, 100));
  const ev = s => w.eval(s);
  // reguli
  ok(ev(`fixSchemeTypo('htp://a.ro/x')`) === 'http://a.ro/x', 'schemă htp');
  ok(ev(`fixSchemeTypo('https//a.ro/p')`) === 'https://a.ro/p', 'schemă https//');
  ok(ev(`fixSchemeTypo('hhttps://a.ro')`) === 'https://a.ro', 'schemă hhttps');
  ok(ev(`fixSchemeTypo('pagina.html')`) === null, 'fără fals pozitiv');
  ok(ev(`unmangle('http://https://a.ro/x')`) === 'https://a.ro/x', 'schemă dublă');
  ok(ev(`unmangle('https://a.ro/x%20')`) === 'https://a.ro/x', '%20 final');
  ok(ev(`unmangle('https://a.ro/f_(1)')`) === null, 'paranteze echilibrate');
  ok(ev(`stripTracking('https://a.ro/p?id=3&utm_source=x&fbclid=1#s')`) === 'https://a.ro/p?id=3#s', 'tracking');
  ok(ev(`addMissingScheme('www.a.ro/x')`) === 'https://www.a.ro/x', 'www fără schemă');
  // surse + aplicare + anulare
  const src = '<a href="htp://a.ro/x">1</a> <a href="http://a.ro/p?utm_source=a&amp;id=1">2</a> <a href="http://gone.ro/z">z</a> <a href="http://gone.ro/z">z2</a>';
  ev(`GD.files.set('a.html', {text: ${JSON.stringify(src)}, backup:null, dirty:false}); GD.verify=false`);
  ev(`GD.files.set('b.md', {text: 'Vezi [bun](http://gone.ro/z) și <https://c.ro/>.\\n\\n[r]: https://d.ro/q', backup:null, dirty:false})`);
  ev(`GD.files.set('e.xml', {text: '<rss><wp:wxr_version>1.2</wp:wxr_version><item><title>Despre</title><content:encoded><![CDATA[<a href="http://x.ro/a?utm_medium=1">a</a>]]></content:encoded></item><item><title>M</title><wp:postmeta><wp:meta_key><![CDATA[_menu_item_url]]></wp:meta_key><wp:meta_value><![CDATA[htp://x.ro/m]]></wp:meta_value></wp:postmeta></item></rss>', backup:null, dirty:false})`);
  const occ = ev(`gScanAll().map(o => o.attr + ':' + o.url)`);
  ok(occ.includes('menu:htp://x.ro/m') && occ.includes('md:http://gone.ro/z') && occ.includes('mdref:https://d.ro/q') && occ.includes('auto:https://c.ro/'), 'scanner HTML/MD/WXR: ' + occ.length);
  const props = await ev(`gPropose()`);
  ok(props.length === 4 && props.every(p => !p.verified), 'propuneri neverificate fără verificare live: ' + props.length);
  ok(props.some(p => p.nav && p.rule === 'scheme_typo'), 'meniu detectat (impact)');
  let r = ev(`gApply(GD.props = ${JSON.stringify(props)}, 'r0')`);
  ok(r.applied === 0 && r.skipped.length === 4, 'neverificat → nu se aplică');
  props.forEach(p => p.verified = true);
  r = ev(`gApply(${JSON.stringify(props)}, 'r1')`);
  ok(r.applied === 4, 'aplicate 4');
  const nowA = ev(`GD.files.get('a.html').text`);
  ok(nowA.includes('href="http://a.ro/x"') && nowA.includes('href="http://a.ro/p?id=1"'), 'text modificat corect');
  ok(!ev(`GD.files.get('e.xml').text`).includes('utm_medium') && ev(`GD.files.get('e.xml').text`).includes('http://x.ro/m'), 'WXR modificat');
  const u = ev(`gUndo(null, 'r1')`);
  ok(u.undone === 4 && u.conflicts === 0, 'anulare completă');
  ok(ev(`GD.files.get('a.html').text`) === src, 'a.html identic după anulare');
  // dezlegare + anulare cu context deplasat
  const un = ev(`gManual('http://gone.ro/z', '')`);
  ok(un.length === 3, 'unlink propuneri: ' + un.length);
  ok(ev(`gApply(${JSON.stringify(un)}, 'r2')`).applied === 3, 'unlink aplicat');
  ok(ev(`GD.files.get('a.html').text`).includes('> z z2') || ev(`GD.files.get('a.html').text`).includes(' z z2'), 'linkuri dezlegate');
  ok(ev(`gUndo(null,'r2')`).conflicts === 0 && ev(`GD.files.get('a.html').text`) === src, 'unlink anulat identic');
  // 403
  ev(`S.cfg = {reviewForbidden:true}`);
  ok(ev(`isBroken('Forbidden')`) === false && ev(`isReview('Forbidden')`) === true, '403 = de verificat');
  ev(`S.cfg = {reviewForbidden:false}`);
  ok(ev(`isBroken('Forbidden')`) === true, '403 rupt dacă opțiunea e oprită');
  ev(`S.cfg = {reviewForbidden:true}`);
  // vederi
  ev(`S.results = [
    {url:'https://a.ro/', status:'Ok', title:'Acasă', h1:'Acasă', parents:[], elapsed_ms:10},
    {url:'https://a.ro/x', status:'NotFound', parents:[{url:'https://a.ro/', text:'x'}], elapsed_ms:20, message:''},
    {url:'https://b.ro/y', status:'Forbidden', parents:[{url:'https://a.ro/', text:'y'}], elapsed_ms:30, message:''},
    {url:'http://a.ro/old', status:'Redirected', final_url:'https://a.ro/new', chain:[{url:'http://a.ro/old', code:301}], parents:[{url:'https://a.ro/', text:'o'}], elapsed_ms:5}]; S.startUrls=[new URL('https://a.ro/')]`);
  const d = ev(`domainStats().map(x => x.host + ':' + x.broken + ':' + x.review)`);
  ok(d.includes('a.ro:1:0') && d.includes('b.ro:0:1'), 'domenii: ' + d);
  ok(ev(`healthMap()[0].broken`) === 1 && ev(`healthMap()[0].review`) === 1, 'harta de sănătate');
  ok(ev(`redirectsList()[0].perm`) === true, 'redirecționare permanentă');
  const html1 = ev(`guardianHtml()`); ok(html1.includes('VERIGA Guardian') && html1.includes('a.ro'), 'raport guardian');
  ev(`GD.log=[{status:'Applied', time:new Date().toISOString(), rule:'tracking', old:'=cmd', new:'x', run:'r'}]`);
  ok(ev(`complianceHtml(30)`).includes('compliance report') || ev(`complianceHtml(30)`).includes('conformitate'), 'raport conformitate');
  ok(ev(`csvSafe('=HYPERLINK(1)')`) === "'=HYPERLINK(1)", 'protecție formule CSV');
  // drift
  const dd = ev(`sessDiff({results:[{url:'https://a.ro/', status:'Ok', title:'Cofetăria Dulcea prăjituri artizanale', h1:'Prăjituri artizanale'}]}, [{url:'https://a.ro/', status:'Ok', title:'Domeniu de vânzare', h1:'Acest domeniu este de vânzare'}]).drift.length`);
  ok(dd === 1, 'derivă conținut');
  // 404
  const log = '1.1.1.1 - - [10/Oct/2026:13:55:36 +0000] "GET /vechi/pagina HTTP/1.1" 404 153 "https://x.ro/a" "Mozilla/5.0"\n5.6.7.8 - - [10/Oct/2026:13:57:00 +0000] "GET /alt HTTP/1.1" 404 1 "-" "Googlebot/2.1"';
  const h = ev(`parse404(${JSON.stringify(log)}, true, ['/nou/pagina'])`);
  ok(h.length === 1 && h[0].suggestion === '/nou/pagina', '404 + sugestie');

  // ───── audit adversarial ─────
  ok(ev(`fixSchemeTypo('http/pagina.html')`) === null && ev(`fixSchemeTypo('https/ghid/index.html')`) === null && ev(`fixSchemeTypo('htp/doc.pdf')`) === null && ev(`fixSchemeTypo('http:foo.html')`) === null, 'căi relative NU sunt scheme greșite');
  ok(ev(`fixSchemeTypo('ttp://a.ro/x')`) === 'http://a.ro/x', 'ttp:// reparat');
  ev(`GD.files.clear(); GD.verify=false`);
  const code = '<!-- <a href="htp://x.ro/c">c</a> --><pre><a href="htp://x.ro/p">p</a></pre><script>var a=\'<a href="htp://x.ro/s">\';</script><a href="htp://x.ro/real">r</a>';
  const md = '```\n[x](htp://a.ro/f)\n```\nIar `[y](htp://a.ro/i)` și [z](htp://a.ro/real)';
  ev(`GD.files.set('c.html', {text: ${JSON.stringify(code)}, backup:null, dirty:false}); GD.files.set('c.md', {text: ${JSON.stringify(md)}, backup:null, dirty:false})`);
  const urls = ev(`gScanAll().map(o => o.url)`);
  ok(urls.length === 2 && urls.every(u => u.endsWith('/real')), 'cod/comentarii/fenced ignorate: ' + urls);
  ev(`GD.files.clear(); GD.files.set('m.html', {text: '<map><area href="http://x.ro/g" shape="rect"></map><abbr title="t">A</abbr> text <a href="http://x.ro/h">h</a>', backup:null, dirty:false})`);
  ok(ev(`gManual('http://x.ro/g','').length`) === 0, 'unlink nu atinge <area>');
  ok(ev(`gManual('http://x.ro/h','').length`) === 1, 'unlink funcționează pe <a>');
  // jurnal importat ostil
  const evil = [{ id: '"><img src=x onerror=alert(1)>', run: '<b>r</b>', rule: '<script>', status: '<img src=x onerror=1>', old: '<script>alert(1)</script>', new: 'n', old_raw: 'INJECTAT', new_raw: 'x', before: '', pos: 0, file: 'm.html' }];
  ev(`GD.log = gSanitizeLog(${JSON.stringify(evil)}); GD.tab='log'; gdRender()`);
  const logHtml = w.document.querySelector('#gd-body').innerHTML;
  ok(!/<img src=x/.test(logHtml) && !/<script>alert/.test(logHtml) && w.document.querySelectorAll('#gd-body img').length === 0, 'jurnal ostil: fără HTML injectat');
  ok(ev(`GD.log[0].status`) === 'Conflict' && ev(`GD.log[0].rule`) === 'manual', 'jurnal ostil: câmpuri aduse la liste albe');
  ev(`GD.log[0].status='Applied'; GD.files.set('m.html', {text:'ceva x altceva', backup:null, dirty:false}); GD.trust=false`);
  ok(ev(`gUndo(null,null).undone`) === 0 && ev(`GD.files.get('m.html').text`) === 'ceva x altceva', 'jurnal importat neconfirmat nu scrie în fișiere');
  // sesiune ostilă
  ev(`importSession(${JSON.stringify(JSON.stringify({ results: [{ url: '<img src=x onerror=alert(1)>', status: '<script>', kind: '<b>', http_status: '<img src=x>', depth: '<svg onload=1>', elapsed_ms: '<i>', size: '<u>', title: '=cmd|calc', chain: [{ url: 'u', code: '<x>' }], parents: [{ url: '<p>', text: '<q>' }] }] }))})`);
  const r0 = ev(`S.results[0]`);
  ok(r0.status === 'Skipped' && r0.http_status === null && r0.depth === 0 && r0.chain[0].code === 0, 'sesiune ostilă: tipuri sigure');
  ev(`applyView(); renderRows(); showDetail(0)`);
  ok(w.document.querySelectorAll('#spacer img, #detail img, #spacer svg, #detail svg').length === 0, 'sesiune ostilă: nimic injectat în DOM');
  ok(ev(`toCsv(',', false)`).includes("'=cmd|calc"), 'CSV principal neutralizează formule');
  // verificare live: acceptare strictă
  ok(ev(`gAccept('http://a.ro/x','https://a.ro/x',{ok:true,code:200,final:'https://www.a.ro/x/'})[0]`) === true, 'același resursă acceptată');
  ok(ev(`gAccept('http://a.ro/x','https://a.ro/x',{ok:true,code:200,final:'https://a.ro/y'})[0]`) === false, 'redirecționare în altă parte refuzată');
  ok(ev(`gAccept('http://a.ro/v','https://a.ro/n',{ok:true,code:200,final:'https://a.ro/'})[0]`) === false, 'aterizare pe pagina principală refuzată');
  ok(ev(`gAccept('http://a.ro/v','https://a.ro/n',{ok:true,code:200,final:'https://a.ro/login'})[0]`) === false, 'login refuzat');
  ok(ev(`gAccept('a','b',{ok:false,code:0,cors:true})[0]`) === false && ev(`gAccept('a','https://a.ro/',{ok:true,code:200,final:null})[0]`) === false, 'CORS / URL final necunoscut ⇒ neverificat');

  // injecție prin URL nou
  ev(`GD.files.clear(); GD.files.set('i.html', {text: "<a href='http://x.ro/v'>v</a>", backup:null, dirty:false}); GD.files.set('i.md', {text: '[a](http://x.ro/v)', backup:null, dirty:false})`);
  for (const bad of ["http://x.ro/n' onmouseover='alert(1)", 'http://x.ro/"><script>', 'http://x.ro/a b', 'http://x.ro/`x`']) ok(ev(`gManual('http://x.ro/v', ${JSON.stringify(bad)}).length`) === 0, 'manual refuză: ' + bad);
  ok(ev(`gManual('http://x.ro/v', 'http://x.ro/ok?a=1&b=2').length`) === 2, 'manual acceptă URL curat');
  const hostile = { id: 'p1', file: 'i.html', page: 'i', start: 9, end: 22, old_raw: 'http://x.ro/v', new_raw: "http://x.ro/n' onmouseover='alert(1)", old: 'http://x.ro/v', new: 'x', rule: 'redirect', verified: true, nav: false, pages: 1, note: '' };
  const rr = ev(`gApply([${JSON.stringify(hostile)}], 'rx')`);
  ok(rr.applied === 0 && ev(`GD.files.get('i.html').text`) === "<a href='http://x.ro/v'>v</a>", 'aplicarea refuză Location ostil');
  // UI
  ev(`GD.tab='fix'; gdRender()`); ok(w.document.querySelector('#gd-body').innerHTML.includes('gd-scan'), 'UI reparații');
  for (const t of ['log', 'dom', 'health', 'redir', 'trend', 'l404', 'comp']) { ev(`GD.tab='${t}'; gdRender()`); ok(w.document.querySelector('#gd-body').innerHTML.length > 20, 'UI tab ' + t); }
  ev(`LANG='en'; gdRender()`);
  console.log(fails ? fails + ' EȘECURI' : 'TOATE TESTELE GUARDIAN AU TRECUT'); process.exit(fails ? 1 : 0);
})();
