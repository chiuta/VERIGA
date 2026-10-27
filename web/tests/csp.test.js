// Verifică că meta CSP are hash-urile curente ale <script>/<style> (rulează stamp-csp.py dacă pică).
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const s = fs.readFileSync(path.join(__dirname, '..', 'veriga.html'), 'utf8');
const h = t => "'sha256-" + crypto.createHash('sha256').update(t).digest('base64') + "'";
const [head, body] = s.split('<body>');
const sty = /<style>([\s\S]*?)<\/style>/.exec(head)[1];
const scripts = [...body.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
const csp = /Content-Security-Policy" content="([^"]*)"/.exec(s)[1];
let bad = 0;
for (const x of [h(sty), ...scripts.map(h)]) if (!csp.includes(x)) { bad++; console.log('lipsește hash', x); }
if (/script-src[^;]*unsafe-inline/.test(csp)) { bad++; console.log("script-src nu trebuie să conțină 'unsafe-inline'"); }
console.log(bad ? 'CSP NEACTUALIZAT — rulează: python3 web/stamp-csp.py web/veriga.html' : 'CSP ok');
process.exit(bad ? 1 : 0);
