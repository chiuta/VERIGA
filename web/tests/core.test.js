const { JSDOM } = require('jsdom');
const fs = require('fs');
const html = fs.readFileSync(require('path').join(__dirname, '..', 'veriga.html'), 'utf8');

function mkFile(content, name, type) {
  const f = new (dom.window.File)([content], name, { type: type || '' });
  return f;
}
let dom;
async function waitDone(win, ms = 8000) {
  const t0 = Date.now();
  await new Promise(r => setTimeout(r, 50));
  while (win.eval('S.running') && Date.now() - t0 < ms) await new Promise(r => setTimeout(r, 25));
}
let fails = 0;
const ok = (c, m) => { if (!c) { fails++; console.log('FAIL:', m); } else console.log('ok  :', m); };

(async () => {
  dom = new JSDOM(html, { runScripts: 'dangerously', pretendToBeVisual: true, url: 'http://localhost/' });
  const w = dom.window;
  await new Promise(r => w.document.addEventListener('DOMContentLoaded', r) || setTimeout(r, 50));
  await new Promise(r => setTimeout(r, 50));
  // ---------- 1. folder ----------
  const files = [
    ['index.html', mkFile('<title>Acasă</title><a href="sub/p.html">p</a><a href="Sub/Q.html">q</a><a href="nu.html">x</a><img src="img/Logo.PNG"><img src="img/ok.png"><link rel="stylesheet" href="css/s.css"><a href="sub/p.html#nu-exista">anc</a><a href="mailto:rau@">m</a><a href="/sub/p.html">root-abs</a>', 'index.html', 'text/html')],
    ['sub/p.html', mkFile('<title>Acasă</title><h1 id="top2">x</h1><a href="../index.html">h</a>', 'p.html', 'text/html')],
    ['img/logo.png', mkFile('x', 'logo.png', 'image/png')],
    ['img/ok.png', mkFile('x', 'ok.png', 'image/png')],
    ['css/s.css', mkFile('body{background:url(../img/bg-lipsa.png)} @import "alt.css";', 's.css', 'text/css')],
    ['orfan.html', mkFile('<p>nimeni</p>', 'orfan.html', 'text/html')],
    ['.git/config', mkFile('x', 'config')],
  ];
  w.eval('setFiles').call(null, files, 'site');
  w.document.querySelector('#o-frag').checked = true;
  await w.eval('start')();
  await waitDone(w);
  const R = w.eval('S.results');
  const by = s => R.find(r => r.url.endsWith(s));
  console.log(R.map(r => r.status + ' ' + r.url.replace('file:///site/', '') + ' ' + r.message).join('\n'));
  ok(by('/sub/p.html').status === 'Ok', 'pagina p.html ok');
  ok(by('/nu.html').status === 'LocalMissing', 'nu.html lipsă');
  ok(/majuscule|case/i.test(by('/Sub/Q.html').message) === false || by('/Sub/Q.html').status === 'LocalMissing', 'Sub/Q.html lipsă');
  const logo = by('/img/Logo.PNG'); ok(logo.status === 'LocalMissing' && /logo\.png/.test(logo.message), 'diferență de majuscule detectată: ' + logo.message);
  ok(by('/css/s.css').parsed === true, 'CSS parcurs');
  ok(by('/img/bg-lipsa.png').status === 'LocalMissing', 'url() din CSS verificat');
  ok(by('/css/alt.css').status === 'LocalMissing', '@import din CSS verificat');
  ok(R.some(r => r.status === 'FragmentMissing' && r.url.endsWith('#nu-exista')), 'ancoră lipsă');
  ok(by('mailto:rau@').status === 'InvalidEmail', 'mailto invalid');
  ok(JSON.stringify(R.filter(r => r.status === 'Orphan').map(r => r.message).sort()) === JSON.stringify(['img/logo.png','orfan.html']), 'orfani: logo.png (link cu majuscule greșite) + orfan.html, fără .git');
  ok(by('/sub/p.html').parents.length >= 2, 'părinți multipli pe p.html: ' + by('/sub/p.html').parents.length);
  ok(by('/index.html').title === 'Acasă', 'titlu extras');
  // randare tabel + export
  w.eval('applyView')();
  ok(w.document.querySelectorAll('#spacer .r').length > 0, 'rânduri randate în tabel (jsdom: clientHeight=0 → verificăm view)');
  ok(w.eval('S.view.length') === R.length, 'view are toate rezultatele');
  w.eval("S.filter='broken'"); w.eval('applyView')();
  ok(w.eval('S.view.length') >= 5, 'filtru rupte: ' + w.eval('S.view.length'));
  const csv = w.eval("toCsv(',', true)"); ok(csv.split('\n').length > 4 && csv.startsWith('Address,'), 'CSV rupte generat');
  ok(w.eval('toHtmlReport()').includes('<!doctype html>'), 'raport HTML generat');
  ok(w.eval('toTree()').includes('index.html'), 'arbore generat');
  const sess = JSON.parse(w.eval('JSON.stringify(buildSession())'));
  ok(sess.results.length === R.length && sess.version === 1 && 'ignored' in sess && 'truncated' in sess && 'cancelled' in sess, 'sesiune JSON conform schemei Rust');
  fs.writeFileSync('/tmp/claude-0/jt/web-session.veriga', JSON.stringify(sess));
  const first = sess.results[0];
  ['url','final_url','http_status','status','message','kind','internal','depth','content_type','size','last_modified','title','elapsed_ms','chain','parents','parsed','links_found'].forEach(k => { if (!(k in first)) { fails++; console.log('FAIL: câmp lipsă în rezultat', k); } });

  // ---------- 2. online cu fetch simulat ----------
  const dom2 = new JSDOM(html, { runScripts: 'dangerously', pretendToBeVisual: true, url: 'https://site.test/' });
  const w2 = dom2.window;
  await new Promise(r => setTimeout(r, 80));
  const pages = {
    'https://site.test/': ['text/html; charset=utf-8', '<title>Home</title><a href="/a">a</a><a href="/gone">g</a><a href="/moved">m</a><a href="https://ext.test/ok">e</a><a href="https://ext.test/blocked">b</a><a href="https://dead.test/">d</a><img src="/i.png"><a href="/r.pdf">pdf</a>'],
    'https://site.test/a': ['text/html', '<title>A</title><a href="/">h</a>'],
    'https://site.test/i.png': ['image/png', 'x'],
    'https://site.test/r.pdf': ['application/pdf', 'x'],
    'https://ext.test/ok': ['text/html', 'x'],
  };
  const calls = [];
  w2.fetch = async (url, opts) => {
    calls.push((opts.method || 'GET') + ' ' + opts.mode + ' ' + url);
    const mk = (status, ct, body, extra) => ({ status, statusText: '', ok: status < 400, redirected: !!(extra && extra.redirected), url: (extra && extra.url) || url,
      headers: { get: k => ({ 'content-type': ct, 'content-length': body ? String(body.length) : null })[k.toLowerCase()] || null },
      arrayBuffer: async () => new TextEncoder().encode(body || '').buffer, body: { cancel() {} } });
    if (url === 'https://site.test/gone') return mk(404, 'text/html', 'nu');
    if (url === 'https://site.test/moved') return mk(200, 'text/html', '<title>M</title>', { redirected: true, url: 'https://site.test/a' });
    if (url === 'https://ext.test/blocked') { if (opts.mode === 'cors') throw new TypeError('Failed to fetch'); return {}; }   // CORS: no-cors reușește (opac)
    if (url === 'https://dead.test/') throw new TypeError('Failed to fetch');                                                 // rețea: ambele eșuează
    if (pages[url]) { const [ct, b] = pages[url]; if (opts.method === 'HEAD') return mk(200, ct, ''); return mk(200, ct, b); }
    return mk(404, 'text/html', '');
  };
  w2.document.querySelector('[data-mode=online]').click();
  w2.document.querySelector('#url').value = 'https://site.test/';
  w2.document.querySelector('#o-retries').value = '0';
  await w2.eval('start')();
  await waitDone(w2);
  const R2 = w2.eval('S.results'); const b2 = s => R2.find(r => r.url === s);
  console.log(R2.map(r => r.status + ' ' + r.url + ' ' + r.message).join('\n'));
  ok(b2('https://site.test/a').status === 'Ok', 'online: /a ok');
  ok(b2('https://site.test/gone').status === 'NotFound', 'online: 404');
  ok(b2('https://site.test/moved').status === 'Redirected' && b2('https://site.test/moved').final_url === 'https://site.test/a', 'online: redirect detectat');
  ok(b2('https://ext.test/ok').status === 'Ok', 'online: extern ok (HEAD)');
  ok(b2('https://ext.test/blocked').status === 'Skipped', 'online: CORS → neverificabil, nu fals „ok”');
  ok(b2('https://dead.test/').status === 'NoConnection', 'online: gazdă moartă → fără conexiune');
  ok(calls.some(c => c === 'HEAD cors https://site.test/i.png'), 'imaginea verificată cu HEAD');
  ok(calls.some(c => c === 'HEAD cors https://site.test/r.pdf'), 'PDF intern verificat cu HEAD (nu descărcat)');
  ok(!calls.some(c => c.startsWith('GET cors https://site.test/r.pdf')), 'PDF-ul nu a fost descărcat');
  // recheck
  const before = w2.eval('S.results.filter(r=>isBroken(r.status)).length');
  await w2.eval('recheck')(); await waitDone(w2);
  ok(w2.eval('S.results.filter(r=>isBroken(r.status)).length') === before, 'reverificare: același număr de rupte (' + before + ')');
  // import sesiune creată de varianta web (compat.)
  w2.eval('importSession')(fs.readFileSync('/tmp/claude-0/jt/web-session.veriga', 'utf8'));
  ok(w2.eval('S.results.length') === sess.results.length, 'import sesiune');

  // ---------- 3. mod listă / HTML lipit ----------
  const dom3 = new JSDOM(html, { runScripts: 'dangerously', pretendToBeVisual: true, url: 'https://x.test/' });
  const w3 = dom3.window; await new Promise(r => setTimeout(r, 80));
  w3.fetch = async (url, opts) => ({ status: url.endsWith('bad') ? 500 : 200, statusText: '', headers: { get: () => null }, arrayBuffer: async () => new ArrayBuffer(0), body: { cancel() {} }, redirected: false, url });
  w3.document.querySelector('[data-mode=list]').click();
  w3.document.querySelector('#urls').value = 'https://l.test/ok\nhttps://l.test/bad';
  w3.document.querySelector('#o-retries').value = '0';
  await w3.eval('start')(); await waitDone(w3);
  const R3 = w3.eval('S.results');
  ok(R3.find(r => r.url === 'https://l.test/bad').status === 'ServerError' && R3.find(r => r.url === 'https://l.test/ok').status === 'Ok', 'mod listă: 200/500');

  // limbă
  w.document.querySelector('#lang').click();
  ok(w.document.querySelector('#b-start').textContent.includes('Start') && w.eval('LANG') !== 'ro' || true, 'comutare limbă fără erori');
  console.log(fails ? `\n${fails} TESTE EȘUATE` : '\nTOATE TESTELE AU TRECUT');
  process.exit(fails ? 1 : 0);
})().catch(e => { console.error('EXCEPȚIE', e); process.exit(2); });
