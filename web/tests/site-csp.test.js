// Site-ul (index.html și paginile pe limbă): hash-urile CSP acoperă stilurile și scripturile REALE, iar browserul nu raportează nicio încălcare.
// Regresie: un «<style>» scris într-un comentariu al codului inclus făcea ca web/stamp-site-csp.py să calculeze hash pentru un text greșit;
// browserul bloca atunci tot stilul paginii (pagină fără aspect, cu derulare orizontală de ~3200 px pe telefon), iar `--check` trecea, fiindcă folosea același regex.
const fs = require('fs'), path = require('path'), crypto = require('crypto'), assert = require('assert');
const { chromium } = require('playwright');
const root = path.join(__dirname, '..', '..');
const PAGES = ['index.html', ...['en', 'fr', 'it', 'es', 'pt', 'de', 'hu'].map(l => l + '/index.html')];
const h = t => "'sha256-" + crypto.createHash('sha256').update(t).digest('base64') + "'";
let bad = 0;
// 1) static: corpul scripturilor este scos înainte de a căuta elementele <style>
for (const f of PAGES) {
  const s = fs.readFileSync(path.join(root, f), 'utf8');
  const csp = /Content-Security-Policy" content="([^"]*)"/.exec(s)[1];
  const styleSrc = (/style-src ([^;]*)/.exec(csp) || [])[1] || '';
  const bare = s.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');
  const styles = [...bare.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)].map(m => m[1]).filter(t => t.trim());
  assert.ok(styles.length >= 2, f + ': nu găsesc blocurile <style>');
  for (const t of styles) if (!styleSrc.includes(h(t))) { bad++; console.log(f + ': un bloc <style> de ' + t.length + ' caractere nu are hash în CSP — rulează python3 web/stamp-site-csp.py ' + f); }
}
// 2) în browser: nicio încălcare CSP de stil/script, toate foile de stil aplicate, fără derulare orizontală pe 360 px
(async () => {
  const b = await chromium.launch();
  for (const f of PAGES) {
    const p = await b.newPage({ viewport: { width: 360, height: 640 } });
    await p.addInitScript(() => { window.__csp = []; document.addEventListener('securitypolicyviolation', e => window.__csp.push(e.violatedDirective + ' ' + e.blockedURI)); });
    await p.goto('file://' + path.join(root, f));
    await p.waitForTimeout(600);
    const r = await p.evaluate(() => ({
      csp: window.__csp.filter(x => /^(style|script)-src/.test(x)),
      unapplied: [...document.querySelectorAll('style')].filter(s => s.textContent.trim() && !s.sheet).length,
      sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth }));
    if (r.csp.length) { bad++; console.log(f + ': încălcări CSP: ' + r.csp.join('; ')); }
    if (r.unapplied) { bad++; console.log(f + ': ' + r.unapplied + ' bloc(uri) <style> neaplicate (blocate de CSP)'); }
    if (r.sw > r.cw) { bad++; console.log(f + ': derulare orizontală pe 360 px (' + r.sw + ' > ' + r.cw + ')'); }
    await p.close();
  }
  await b.close();
  console.log(bad ? 'SITE CSP EȘUAT' : 'SITE CSP OK (' + PAGES.length + ' pagini)');
  process.exit(bad ? 1 : 0);
})().catch(e => { console.error(e); process.exit(1); });
