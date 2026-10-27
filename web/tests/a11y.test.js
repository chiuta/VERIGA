// Audit de accesibilitate WCAG 2.2 AA (axe-core) în Chromium, temă clară și întunecată.
// Rulează:  npm test   (din web/tests; necesită `npx playwright install chromium`)
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const axeSrc = fs.readFileSync(require.resolve('axe-core/axe.min.js'), 'utf8');
(async () => {
  const b = await chromium.launch(); let bad = 0;
  for (const scheme of ['light', 'dark']) {
    const ctx = await b.newContext({ bypassCSP: true, colorScheme: scheme, viewport: { width: 1280, height: 900 } });
    const p = await ctx.newPage();
    await p.goto('file://' + path.join(__dirname, '..', 'veriga.html'));
    await p.addScriptTag({ content: axeSrc });
    await p.evaluate(() => { const d = document.querySelector('#gd-det'); if (d) d.open = true; });
    const r = await p.evaluate(() => axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice'] } }));
    console.log(scheme, 'violations:', r.violations.length);
    for (const v of r.violations) { bad++; console.log(' -', v.id, v.impact, v.help, v.nodes.slice(0, 3).map(n => n.target.join(' '))); }
  }
  await b.close(); process.exit(bad ? 1 : 0);
})();
