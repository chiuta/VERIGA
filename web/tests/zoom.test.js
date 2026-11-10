// Zoom pe bara de jos, zoom automat și antet de tabel fix (Chromium)
const { chromium } = require('playwright');
const path = require('path');
const assert = require('assert');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1920, height: 1080 } });
  await p.goto('file://' + path.join(__dirname, '..', 'veriga.html'));
  await p.evaluate(() => { for (let i = 0; i < 500; i++) { const r = newResult('https://x.test/' + i, 'Link', 1, false); r.status = 'NotFound'; r.http_status = 404; S.results.push(r); } document.querySelector('#res').classList.remove('hide'); applyView(); });
  const pct = () => p.evaluate(() => document.querySelector('#zpct').textContent);
  assert.strictEqual(await pct(), '160%', 'zoom automat la 1920x1080');
  const bar = await p.evaluate(() => { const r = document.querySelector('#zbar').getBoundingClientRect(); return [r.bottom, innerHeight]; });
  assert.strictEqual(bar[0], bar[1], 'bara de jos lipită de marginea ecranului');
  await p.click('#z-in'); assert.strictEqual(await pct(), '170%');
  await p.click('#z-out'); await p.click('#z-out'); assert.strictEqual(await pct(), '150%');
  assert.strictEqual(await p.evaluate(() => localStorage.getItem('veriga.zoom')), '1.5');
  assert.strictEqual(await p.getAttribute('#z-auto', 'aria-pressed'), 'false');
  await p.reload(); assert.strictEqual(await pct(), '150%', 'zoom manual păstrat după reîncărcare');
  await p.click('#z-auto'); assert.strictEqual(await pct(), '160%');
  await p.setViewportSize({ width: 1366, height: 768 }); await p.waitForTimeout(300);
  assert.strictEqual(await pct(), '115%', 'zoom automat urmărește fereastra');
  await p.evaluate(() => { for (let i = 0; i < 500; i++) { const r = newResult('https://y.test/' + i, 'Link', 1, false); r.status = 'NotFound'; S.results.push(r); } applyView(); document.querySelector('#tbl').scrollTop = 4000; });
  const hd = await p.evaluate(() => document.querySelector('#thead').getBoundingClientRect().top - document.querySelector('#tbl').getBoundingClientRect().top);
  assert.ok(Math.abs(hd) <= 2, 'antetul rămâne fix la derulare: ' + hd);
  for (let i = 0; i < 20; i++) await p.click('#z-out', { force: true });
  assert.strictEqual(await pct(), '60%'); assert.ok(await p.isDisabled('#z-out'));
  await b.close(); console.log('ZOOM OK');
})().catch(e => { console.error(e); process.exit(1); });
