// Adversarial: un site ostil verificat de aplicație, în context same-origin (cum rulează în pluginul WordPress).
// Nicio sarcină utilă nu are voie să se execute; raportul HTML exportat nu are voie să conțină marcaj activ.
const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');
const assert = require('assert');
const APP = fs.readFileSync(path.join(__dirname, '..', 'veriga.html'));
const P = '"><img src=x onerror=window.__pwn=1><script>window.__pwn=2</script>';
const SITE = `<!doctype html><html><head><title>${P}</title><base href="/sub/"><meta http-equiv="refresh" content="0;url=javascript:window.__pwn=3"></head><body>
<a href="javascript:window.__pwn=4">js</a><a href="/a${P}">${P}</a><a href="/b?x='-alert(1)-'" title="${P}">t</a>
<a href='/c" onmouseover="window.__pwn=5" x="'>q</a><a href="/redir">r</a><a href="/hdr">h</a>
<a href="data:text/html,<script>window.__pwn=6</script>">d</a><a href="/&#x22;&gt;&lt;svg onload=window.__pwn=7&gt;">ent</a>
<a href="/long/${'A'.repeat(60000)}">long</a><a href="//evil.invalid/${P}">ext</a><a href="mailto:a@b.c?subject=${P}">mail</a>
<img src="/pwn?img=${P}"><iframe src="/pwn?iframe"></iframe><script src="/pwn?script"></script></body></html>`;
const srv = http.createServer((req, res) => {
  const u = req.url.split('?')[0];
  const send = (c, t, b, h = {}) => { res.writeHead(c, { 'Content-Type': t, ...h }); res.end(b); };
  if (u === '/__app') return send(200, 'text/html; charset=utf-8', APP);
  if (u === '/') return send(200, 'text/html; charset=utf-8', SITE);
  if (u === '/redir') return send(301, 'text/html', 'r', { Location: '/landing%22%3E%3Csvg%20onload=window.__pwn=8%3E' });
  if (u === '/hdr') return send(200, 'text/html', '<title></title>ok', { Server: 'x', 'X-Foo': 'y' });
  return send(404, 'text/html', `<h1>${P}</h1>`);
});
(async () => {
  await new Promise(r => srv.listen(0, '127.0.0.1', r));
  const base = 'http://127.0.0.1:' + srv.address().port;
  const b = await chromium.launch(); const ctx = await b.newContext({ viewport: { width: 1600, height: 900 } }); const p = await ctx.newPage();
  const dialogs = []; p.on('dialog', d => { dialogs.push(d.message()); d.dismiss(); });
  await p.addInitScript(() => { window.__pwn = 0; });
  await p.goto(base + '/__app');
  await p.click('#tab-online'); await p.fill('#url', base + '/'); await p.click('#b-start');
  await p.waitForFunction(() => !S.running, null, { timeout: 120000 });
  assert.ok(await p.evaluate(() => S.results.length) >= 10, 'a parcurs site-ul ostil');
  await p.evaluate(() => { for (let i = 0; i < S.results.length; i++) showDetail(i); });
  const html = await p.evaluate(() => toHtmlReport());
  const sess = await p.evaluate(() => JSON.parse(JSON.stringify(buildSession())));
  sess.results.forEach(r => { r.url += '"><img src=x onerror=window.__pwn=11>'; r.title = '<img src=x onerror=window.__pwn=12>'; });
  await p.evaluate(s => importSession(s), sess);
  await p.evaluate(() => { for (let i = 0; i < S.results.length; i++) showDetail(i); });
  const dom = await p.evaluate(() => ({ pwn: window.__pwn, imgs: document.querySelectorAll('#tbl img,#detail img,#res img').length, scripts: document.querySelectorAll('#tbl script,#detail script,#res script').length, handlers: [...document.querySelectorAll('#tbl *,#detail *,#res *')].filter(e => [...e.attributes].some(a => /^on/i.test(a.name))).length }));
  assert.deepStrictEqual(dom, { pwn: 0, imgs: 0, scripts: 0, handlers: 0 }, 'nicio execuție/marcaj activ în aplicație: ' + JSON.stringify(dom));
  assert.deepStrictEqual(dialogs, [], 'nicio fereastră de dialog');
  assert.ok(!html.includes('<img src=x onerror') && !html.includes('<script>window.__pwn'), 'raportul HTML exportat nu conține marcaj activ');
  const rp = await ctx.newPage(); rp.on('dialog', d => { dialogs.push('raport:' + d.message()); d.dismiss(); });
  await rp.setContent(html); await rp.waitForTimeout(400);
  assert.strictEqual(await rp.evaluate(() => window.__pwn || 0), 0, 'raportul exportat nu execută cod'); assert.deepStrictEqual(dialogs, []);
  await b.close(); srv.close(); console.log('XSS OK');
})().catch(e => { console.error(e); process.exit(1); });
