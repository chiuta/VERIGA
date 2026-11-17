// Poziția grupurilor de captură fără steagul d: fallback-ul din web/veriga.html (blocul fff:idx) față de m.indices nativ.
// Se extrag din sursă blocul de funcții și obiectul RX (cu steagul d), apoi se compară pe zeci de mii de intrări aleatoare (semânță fixă).
// Rulare: node web/tests/indices.test.js
const fs = require('fs'), path = require('path');
const html = fs.readFileSync(path.join(__dirname, '..', 'veriga.html'), 'utf8');
let ok = 0, bad = 0;
const check = (n, c, x) => { if (c) { ok++; console.log('  ok   ' + n); } else { bad++; console.log('  FAIL ' + n + (x !== undefined ? '  → ' + JSON.stringify(x) : '')); } };

const block = /\/\* fff:idx:start \*\/([\s\S]*?)\/\* fff:idx:end \*\//.exec(html);
check('blocul fff:idx există în veriga.html', !!block);
const H = new Function(block[1] + '\nreturn {gGrp, gAttrPos, gMdPos, gRefPos, gAutoPos, gMenuPos, gLocPos};')();
const a = html.indexOf('const RX = {'), b = html.indexOf('\n};', a);
// după coborâre, expresiile sunt $fffRx(sursă, steaguri) (ajutor local peste FFF.rx): în Node (motor modern) = new RegExp
const RX = new Function('const FFF = {rx: (s, f) => new RegExp(s, f)}, $fffRx = FFF.rx; return ' + html.slice(a + 'const RX = '.length, b + 3).replace(/;\s*$/, ''))();
check('expresiile cu d se extrag din sursă', ['attr', 'md', 'mdref', 'auto', 'menu', 'loc'].every((k) => RX[k] instanceof RegExp && RX[k].hasIndices), Object.keys(RX));

let seed = 20261117;
const rnd = () => { seed |= 0; seed = (seed + 0x6D2B79F5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const pick = (x) => x[Math.floor(rnd() * x.length)];
const gen = (vocab, lo, hi) => { let s = ''; const n = lo + Math.floor(rnd() * (hi - lo + 1)); for (let i = 0; i < n; i++) s += pick(vocab); return s; };
const V = {
  attr: ['href', 'src', 'action', 'poster', 'data-src', 'HREF', '=', '= ', ' =', ' ', '  ', '\t', '\n', '"', "'", 'a', 'b c', 'x-y', ':', '&amp;', '/', '?q=1', '#f', 'é', '"u r l"', "'u'", '""', "''"],
  md: ['[', ']', '(', ')', '!', '<', '>', ' ', '  ', '\t', '"t"', "'t'", '"t w"', 'a', 'b c', 'http://x.y/z_(w)', '\n', '[a]', '[a [b] c]', '](', '](  ', ' "x y")', '<u r l>', '<>', '()', '[]('],
  mdref: ['[a]:', '[a b]:', '[a:b]:', ' ', '  ', '   ', '    ', '\t', ':', '<u>', '<u r>', '<>', 'http://x', 'u', '[', ']', '\n', '[x]: y\n', ' "t"', " 't'", ' (t)'],
  auto: ['<', '>', 'http://', 'https://', 'ftp://', 'x.y', '/z', ' ', '\n', 'a b', 'mailto:', '<http://a>', '<https://b.c/d>'],
  menu: ['_menu_item_url]]></wp:meta_key>', '_menu_item_url]]></wp:meta_key>\n<wp:meta_value><![CDATA[', '<wp:meta_value><![CDATA[', ' ', '\n', '\t', '<![CDATA[', ']]>', ']]></wp:meta_value>', 'http://x/y', 'a b', '<![CDATA[]]>', ']]></wp:meta_value>', '_menu_item_url'],
  loc: ['<loc>', '<LOC>', '</loc>', ' ', '\n', '\t', '<![CDATA[', ']]>', 'http://x/y', 'a b', '<', '>', '&amp;', '<loc></loc>', '<loc> </loc>', '<loc><![CDATA[ http://q ]]></loc>'],
};
const POS = {attr: H.gAttrPos, md: H.gMdPos, mdref: H.gRefPos, auto: H.gAutoPos, menu: H.gMenuPos, loc: H.gLocPos};
const GROUP = {attr: (m) => (m[2] != null ? 2 : 3), md: () => 2, mdref: () => 1, auto: () => 1, menu: () => 1, loc: () => 1};
for (const name of Object.keys(V)) {
  const re = RX[name];
  let strings = 0, matches = 0, empty = 0; const diffs = [];
  for (let i = 0; i < 80000; i++) {
    const s = gen(V[name], 1, 14); strings++;
    re.lastIndex = 0;
    let m, guard = 0;
    while ((m = re.exec(s)) && guard++ < 50) {
      const k = GROUP[name](m), nat = m.indices[k];
      delete m.indices;                                  // motor fără d
      const got = H.gGrp(m, k, POS[name]);
      matches++; if (m[k] === '') empty++;
      if (got[0] !== nat[0] || got[1] !== nat[1]) { if (diffs.length < 3) diffs.push({s, k, nat, got, m0: m[0]}); }
      if (m[0] === '') re.lastIndex++;
    }
  }
  check(`${name}: ${matches} potriviri în ${strings} șiruri (${empty} goale) — pozițiile coincid cu m.indices`, diffs.length === 0 && matches > 300, diffs);
}
console.log(`\n${ok} ok, ${bad} eșuate`);
process.exit(bad ? 1 : 0);
