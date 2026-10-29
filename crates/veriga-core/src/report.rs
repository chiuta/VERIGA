//! Rapoarte: text, CSV/TSV, JSON, HTML autonom, sitemap XML, arbore de site.

use crate::model::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

/// Doar limbile cunoscute ajung în rapoarte (atributul `lang` nu se poate injecta).
pub(crate) fn safe_lang(l: &str) -> &'static str {
    if l == "en" { "en" } else { "ro" }
}

pub(crate) fn tr(lang: &str, ro: &'static str, en: &'static str) -> &'static str {
    if lang == "en" { en } else { ro }
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

pub fn human_size(n: u64) -> String {
    const U: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total: usize,
    pub ok: usize,
    pub redirected: usize,
    pub broken: usize,
    pub skipped: usize,
    pub orphans: usize,
    pub pages: usize,
    pub internal: usize,
    pub external: usize,
    pub total_bytes: u64,
    pub avg_ms: u64,
    pub max_depth: u32,
    pub by_status: BTreeMap<String, usize>,
    pub by_kind: BTreeMap<String, usize>,
    pub by_type: BTreeMap<String, (usize, u64)>,
    pub slowest: Vec<(String, u64)>,
    pub duplicate_titles: Vec<(String, Vec<String>)>,
    pub missing_titles: Vec<String>,
}

impl Summary {
    pub fn from(s: &Session) -> Self {
        let lang = safe_lang(&s.config.lang);
        let mut sm = Summary {
            total: s.results.len(),
            ok: 0,
            redirected: 0,
            broken: 0,
            skipped: 0,
            orphans: 0,
            pages: 0,
            internal: 0,
            external: 0,
            total_bytes: 0,
            avg_ms: 0,
            max_depth: 0,
            by_status: BTreeMap::new(),
            by_kind: BTreeMap::new(),
            by_type: BTreeMap::new(),
            slowest: vec![],
            duplicate_titles: vec![],
            missing_titles: vec![],
        };
        let mut sum_ms = 0u64;
        let mut timed = 0u64;
        let mut titles: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut timings: Vec<(String, u64)> = vec![];
        for r in &s.results {
            *sm.by_status
                .entry(r.status.label(lang).to_string())
                .or_default() += 1;
            *sm.by_kind
                .entry(r.kind.label(lang).to_string())
                .or_default() += 1;
            if r.status.is_broken(&s.config) {
                sm.broken += 1;
            }
            match r.status {
                Status::Ok => sm.ok += 1,
                Status::Redirected => sm.redirected += 1,
                Status::Skipped | Status::RobotsBlocked => sm.skipped += 1,
                Status::Orphan => sm.orphans += 1,
                _ => {}
            }
            if r.status == Status::Orphan {
                continue;
            }
            if r.internal {
                sm.internal += 1;
            } else {
                sm.external += 1;
            }
            sm.max_depth = sm.max_depth.max(r.depth);
            if let Some(sz) = r.size {
                sm.total_bytes += sz;
            }
            if let Some(ct) = &r.content_type {
                let e = sm.by_type.entry(ct.clone()).or_default();
                e.0 += 1;
                e.1 += r.size.unwrap_or(0);
            }
            if r.elapsed_ms > 0 {
                sum_ms += r.elapsed_ms;
                timed += 1;
                timings.push((r.url.clone(), r.elapsed_ms));
            }
            if r.internal
                && r.is_page()
                && matches!(r.status, Status::Ok | Status::Redirected)
                && r.parsed
            {
                sm.pages += 1;
                match &r.title {
                    Some(t) => titles.entry(t.clone()).or_default().push(r.url.clone()),
                    None => sm.missing_titles.push(r.url.clone()),
                }
            }
        }
        sm.avg_ms = sum_ms.checked_div(timed).unwrap_or(0);
        timings.sort_by_key(|t| std::cmp::Reverse(t.1));
        timings.truncate(10);
        sm.slowest = timings;
        sm.duplicate_titles = titles.into_iter().filter(|(_, v)| v.len() > 1).collect();
        sm
    }
}

/// Grupează linkurile rupte după stare, în ordinea stărilor.
fn broken_grouped(s: &Session) -> Vec<(Status, Vec<&LinkResult>)> {
    let mut m: BTreeMap<Status, Vec<&LinkResult>> = BTreeMap::new();
    for r in s.broken() {
        m.entry(r.status).or_default().push(r);
    }
    for v in m.values_mut() {
        v.sort_by(|a, b| a.url.cmp(&b.url));
    }
    m.into_iter().collect()
}

pub fn to_text(s: &Session) -> String {
    let s = &s.redacted();
    let lang = safe_lang(&s.config.lang);
    let sm = Summary::from(s);
    let mut o = String::new();
    let _ = writeln!(
        o,
        "VERIGA — {}",
        tr(
            lang,
            "raport de verificare a legăturilor",
            "link check report"
        )
    );
    let _ = writeln!(
        o,
        "{}: {}",
        tr(lang, "Start", "Start"),
        s.config.start.join(", ")
    );
    let _ = writeln!(o, "{} → {}", s.started, s.finished);
    let _ = writeln!(
        o,
        "{}: {} | OK: {} | {}: {} | {}: {} | {}: {} | {}: {}",
        tr(lang, "Total", "Total"),
        sm.total,
        sm.ok,
        tr(lang, "Rupte", "Broken"),
        sm.broken,
        tr(lang, "Redirecționate", "Redirected"),
        sm.redirected,
        tr(lang, "Omise", "Skipped"),
        sm.skipped,
        tr(lang, "Orfane", "Orphans"),
        sm.orphans
    );
    if s.cancelled {
        let _ = writeln!(
            o,
            "{}",
            tr(
                lang,
                "(verificare oprită înainte de final)",
                "(check stopped before completion)"
            )
        );
    }
    if s.truncated {
        let _ = writeln!(
            o,
            "{}",
            tr(
                lang,
                "(limita de URL-uri a fost atinsă)",
                "(URL limit reached)"
            )
        );
    }
    let _ = writeln!(o);
    let groups = broken_grouped(s);
    if groups.is_empty() {
        let _ = writeln!(
            o,
            "{}",
            tr(lang, "Nicio legătură ruptă.", "No broken links.")
        );
    } else {
        let _ = writeln!(o, "== {} ==", tr(lang, "Legături rupte", "Broken links"));
        for (st, v) in groups {
            let _ = writeln!(o, "\n-- {} ({})", st.label(lang), v.len());
            for r in v {
                let _ = writeln!(o, "{}", r.url);
                if !r.message.is_empty() {
                    let _ = writeln!(o, "    {}", r.message);
                }
                for p in r.parents.iter().take(5) {
                    let txt = if p.text.is_empty() {
                        String::new()
                    } else {
                        format!(" „{}”", p.text)
                    };
                    let _ = writeln!(o, "    ↳ {}{}", p.url, txt);
                }
                if r.parents.len() > 5 {
                    let _ = writeln!(o, "    ↳ … +{}", r.parents.len() - 5);
                }
            }
        }
    }
    let red: Vec<_> = s
        .results
        .iter()
        .filter(|r| r.status == Status::Redirected && !s.config.treat_redirects_as_errors)
        .collect();
    if !red.is_empty() {
        let _ = writeln!(
            o,
            "\n== {} ({}) ==",
            tr(lang, "Redirecționări", "Redirects"),
            red.len()
        );
        for r in red {
            let _ = writeln!(o, "{} → {}", r.url, r.final_url.as_deref().unwrap_or("?"));
        }
    }
    let orph: Vec<_> = s
        .results
        .iter()
        .filter(|r| r.status == Status::Orphan)
        .collect();
    if !orph.is_empty() {
        let _ = writeln!(
            o,
            "\n== {} ({}) ==",
            tr(lang, "Fișiere orfane", "Orphan files"),
            orph.len()
        );
        for r in orph {
            let _ = writeln!(o, "{}", r.message);
        }
    }
    if !sm.duplicate_titles.is_empty() {
        let _ = writeln!(
            o,
            "\n== {} ==",
            tr(lang, "Titluri duplicate", "Duplicate titles")
        );
        for (t, u) in &sm.duplicate_titles {
            let _ = writeln!(o, "„{}” ×{}", t, u.len());
        }
    }
    o
}

fn csv_writer(delim: u8) -> csv::Writer<Vec<u8>> {
    csv::WriterBuilder::new()
        .delimiter(delim)
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(vec![])
}

/// CSV (`,`) sau TSV (`\t`). `only_broken` exportă doar linkurile rupte.
pub fn to_csv(s: &Session, delim: u8, only_broken: bool) -> String {
    let s = &s.redacted();
    let lang = safe_lang(&s.config.lang);
    let mut w = csv_writer(delim);
    let _ = w.write_record([
        "Address",
        "Status-Code",
        "Status",
        "Type",
        "Internal",
        "Content-Type",
        "Size",
        "Title",
        "Level",
        "Redirected-To",
        "Last-Modified",
        "Time-ms",
        "Message",
        "Found-On",
    ]);
    for r in &s.results {
        if only_broken && !r.status.is_broken(&s.config) {
            continue;
        }
        let mut found: Vec<String> = vec![];
        for p in &r.parents {
            if !found.contains(&p.url) {
                found.push(p.url.clone());
            }
            if found.len() >= 3 {
                break;
            }
        }
        let row = [
            r.url.as_str(),
            &r.http_status.map(|c| c.to_string()).unwrap_or_default(),
            r.status.label(lang),
            r.kind.label(lang),
            if r.internal { "yes" } else { "no" },
            r.content_type.as_deref().unwrap_or(""),
            &r.size.map(|c| c.to_string()).unwrap_or_default(),
            r.title.as_deref().unwrap_or(""),
            &r.depth.to_string(),
            r.final_url.as_deref().unwrap_or(""),
            r.last_modified.as_deref().unwrap_or(""),
            &r.elapsed_ms.to_string(),
            r.message.as_str(),
            &found.join(" | "),
        ];
        // injecție de formule: titlurile/URL-urile vin din pagini necontrolate de utilizator
        let _ = w.write_record(row.iter().map(|c| crate::fixer::csv_safe(c)));
    }
    String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
}

pub fn to_json(s: &Session) -> String {
    serde_json::to_string_pretty(&s.redacted()).unwrap_or_default()
}

pub fn to_sitemap_xml(s: &Session) -> String {
    let s = &s.redacted();
    let mut o = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let mut seen = HashSet::new();
    for r in &s.results {
        if !(r.internal
            && r.status == Status::Ok
            && r.is_page()
            && matches!(r.kind, LinkKind::Page | LinkKind::Start | LinkKind::Frame)
            && matches!(r.url.split(':').next(), Some("http" | "https")))
        {
            continue;
        }
        if !seen.insert(r.url.clone()) {
            continue;
        }
        let _ = write!(o, "  <url><loc>{}</loc>", esc(&r.url));
        if let Some(lm) = r
            .last_modified
            .as_deref()
            .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
        {
            let _ = write!(o, "<lastmod>{}</lastmod>", lm.format("%Y-%m-%d"));
        }
        o.push_str("</url>\n");
    }
    o.push_str("</urlset>\n");
    o
}

/// Arborele site-ului (după prima pagină care a descoperit fiecare URL).
pub fn to_tree_text(s: &Session, max_depth: usize) -> String {
    let s = &s.redacted();
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut roots = vec![];
    for (i, r) in s.results.iter().enumerate() {
        if r.kind == LinkKind::Start {
            roots.push(i);
            continue;
        }
        if !matches!(r.kind, LinkKind::Page | LinkKind::Frame) || !r.internal {
            continue;
        }
        if let Some(p) = r.parents.first() {
            children.entry(p.url.as_str()).or_default().push(i);
        }
    }
    let mut out = String::new();
    let mut visited = HashSet::new();
    fn walk(
        s: &Session,
        i: usize,
        depth: usize,
        max: usize,
        children: &HashMap<&str, Vec<usize>>,
        visited: &mut HashSet<usize>,
        out: &mut String,
    ) {
        if !visited.insert(i) {
            return;
        }
        let r = &s.results[i];
        let mark = if r.status.is_broken(&s.config) {
            "✗ "
        } else {
            ""
        };
        let title = r
            .title
            .as_deref()
            .map(|t| format!("  — {t}"))
            .unwrap_or_default();
        let _ = writeln!(out, "{}{}{}{}", "  ".repeat(depth), mark, r.url, title);
        if depth >= max {
            return;
        }
        if let Some(ch) = children.get(r.url.as_str()) {
            for &c in ch {
                walk(s, c, depth + 1, max, children, visited, out);
            }
        }
    }
    for r in roots {
        walk(s, r, 0, max_depth, &children, &mut visited, &mut out);
    }
    out
}

pub(crate) const CSS: &str = r#"
:root{--bg:#fafaf7;--fg:#1d2024;--mut:#6a717b;--card:#fff;--bd:#dcdfe3;--ok:#16794a;--bad:#b3261e;--warn:#9a6700;--acc:#2f5fd0}
@media (prefers-color-scheme:dark){:root{--bg:#14161a;--fg:#e8eaed;--mut:#9aa1ab;--card:#1c1f24;--bd:#2d3239;--ok:#5ccf93;--bad:#ff8a80;--warn:#f0b84a;--acc:#8fb0ff}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,Segoe UI,Roboto,sans-serif}
main{max-width:1200px;margin:0 auto;padding:20px 16px 60px}h1{font-size:1.5rem;margin:.2em 0}h2{font-size:1.15rem;margin:1.8em 0 .5em;border-bottom:1px solid var(--bd);padding-bottom:.3em}
.sub{color:var(--mut);font-size:.9rem}.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px;margin:14px 0}
.card{background:var(--card);border:1px solid var(--bd);border-radius:10px;padding:10px 14px}.card b{display:block;font-size:1.6rem}.card span{color:var(--mut);font-size:.85rem}
.bad b{color:var(--bad)}.okc b{color:var(--ok)}.warn b{color:var(--warn)}
table{width:100%;border-collapse:collapse;background:var(--card);border:1px solid var(--bd);border-radius:8px;overflow:hidden;font-size:.9rem}
th,td{text-align:left;padding:6px 10px;border-bottom:1px solid var(--bd);vertical-align:top;word-break:break-word}th{background:rgba(127,127,127,.1);font-weight:600}
td.u{font-family:ui-monospace,Consolas,monospace;font-size:.82rem}.pill{display:inline-block;padding:1px 8px;border-radius:999px;font-size:.78rem;border:1px solid var(--bd)}
.p-bad{color:var(--bad);border-color:var(--bad)}.p-ok{color:var(--ok);border-color:var(--ok)}.p-warn{color:var(--warn);border-color:var(--warn)}
.par{color:var(--mut);font-size:.8rem;display:block}a{color:var(--acc)}details{margin:.5em 0}summary{cursor:pointer;font-weight:600}
input[type=search]{width:100%;padding:8px 10px;border:1px solid var(--bd);border-radius:8px;background:var(--card);color:var(--fg);margin:8px 0}
pre{background:var(--card);border:1px solid var(--bd);border-radius:8px;padding:10px;overflow:auto;font-size:.8rem}
"#;

fn pill(r: &LinkResult, cfg: &crate::config::Config) -> String {
    let cls = if r.status.is_broken(cfg) {
        "p-bad"
    } else if matches!(r.status, Status::Ok) {
        "p-ok"
    } else {
        "p-warn"
    };
    let code = r.http_status.map(|c| format!("{c} · ")).unwrap_or_default();
    format!(
        "<span class=\"pill {cls}\">{}{}</span>",
        code,
        esc(r.status.label(&cfg.lang))
    )
}

fn parents_html(r: &LinkResult, max: usize) -> String {
    let mut o = String::new();
    for p in r.parents.iter().take(max) {
        let t = if p.text.is_empty() {
            String::new()
        } else {
            format!(" — „{}”", esc(&p.text))
        };
        let low = p.url.to_ascii_lowercase();
        if ["http://", "https://", "file://", "ftp://"]
            .iter()
            .any(|sch| low.starts_with(sch))
        {
            let _ = write!(
                o,
                "<span class=\"par\">↳ <a href=\"{0}\">{0}</a>{1}</span>",
                esc(&p.url),
                t
            );
        } else {
            let _ = write!(o, "<span class=\"par\">↳ {}{}</span>", esc(&p.url), t);
        }
    }
    if r.parents.len() > max {
        let _ = write!(o, "<span class=\"par\">… +{}</span>", r.parents.len() - max);
    }
    o
}

/// Scriptul inline al raportului (filtrare tabel); hash-ul lui intră în CSP.
const REPORT_JS: &str = "document.getElementById('q').addEventListener('input',function(e){var v=e.target.value.toLowerCase();document.querySelectorAll('#all tr').forEach(function(tr,i){if(i===0)return;tr.style.display=tr.textContent.toLowerCase().indexOf(v)<0?'none':''})})";

/// Valoare CSP `'sha256-…'` pentru un script inline.
pub(crate) fn csp_hash(js: &str) -> String {
    use sha2::{Digest, Sha256};
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let d = Sha256::digest(js.as_bytes());
    let mut o = String::from("'sha256-");
    for c in d.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        o.push(A[(n >> 18) as usize & 63] as char);
        o.push(A[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        o.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    o.push('\'');
    o
}

pub fn to_html(s: &Session) -> String {
    let s = &s.redacted();
    let jsh = csp_hash(REPORT_JS);
    let lang = safe_lang(&s.config.lang);
    let sm = Summary::from(s);
    let mut o = String::new();
    let _ = write!(
        o,
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src {jsh}; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'\"><meta name=\"color-scheme\" content=\"light dark\"><title>VERIGA — {}</title><style>{CSS}</style></head><body><main>",
        tr(lang, "raport", "report")
    );
    let _ = write!(
        o,
        "<h1>VERIGA — {}</h1><div class=\"sub\">{} · {} → {}</div>",
        tr(
            lang,
            "raport de verificare a legăturilor",
            "link check report"
        ),
        esc(&s.config.start.join(", ")),
        esc(&s.started),
        esc(&s.finished)
    );
    if s.cancelled {
        let _ = write!(
            o,
            "<p class=\"sub\">⚠ {}</p>",
            tr(
                lang,
                "Verificarea a fost oprită înainte de final.",
                "The check was stopped before completion."
            )
        );
    }
    let cards = [
        ("", sm.total, tr(lang, "URL-uri verificate", "URLs checked")),
        ("okc", sm.ok, "OK"),
        ("bad", sm.broken, tr(lang, "Legături rupte", "Broken links")),
        (
            "warn",
            sm.redirected,
            tr(lang, "Redirecționări", "Redirects"),
        ),
        ("", sm.orphans, tr(lang, "Fișiere orfane", "Orphan files")),
        ("", sm.pages, tr(lang, "Pagini HTML", "HTML pages")),
    ];
    o.push_str("<div class=\"cards\">");
    for (c, n, l) in cards {
        let _ = write!(
            o,
            "<div class=\"card {c}\"><b>{n}</b><span>{l}</span></div>"
        );
    }
    o.push_str("</div>");

    // Rupte
    let _ = write!(o, "<h2>{}</h2>", tr(lang, "Legături rupte", "Broken links"));
    let groups = broken_grouped(s);
    if groups.is_empty() {
        let _ = write!(
            o,
            "<p>{}</p>",
            tr(lang, "Nicio legătură ruptă. 🎉", "No broken links. 🎉")
        );
    }
    for (st, v) in groups {
        let _ = write!(
            o,
            "<details open><summary>{} ({})</summary><table><tr><th>URL</th><th>{}</th><th>{}</th></tr>",
            esc(st.label(lang)),
            v.len(),
            tr(lang, "Detalii", "Details"),
            tr(lang, "Găsit pe", "Found on")
        );
        for r in v {
            let _ = write!(
                o,
                "<tr><td class=\"u\">{}</td><td>{} {}</td><td>{}</td></tr>",
                esc(&r.url),
                pill(r, &s.config),
                esc(&r.message),
                parents_html(r, 5)
            );
        }
        o.push_str("</table></details>");
    }

    // Redirecționări
    let red: Vec<_> = s
        .results
        .iter()
        .filter(|r| r.status == Status::Redirected)
        .collect();
    if !red.is_empty() {
        let _ = write!(
            o,
            "<h2>{} ({})</h2><details><summary>{}</summary><table><tr><th>URL</th><th>{}</th><th>{}</th></tr>",
            tr(lang, "Redirecționări", "Redirects"),
            red.len(),
            tr(lang, "Arată", "Show"),
            tr(lang, "Țintă finală", "Final target"),
            tr(lang, "Lanț", "Chain")
        );
        for r in red {
            let chain = r
                .chain
                .iter()
                .map(|h| h.code.to_string())
                .collect::<Vec<_>>()
                .join(" → ");
            let _ = write!(
                o,
                "<tr><td class=\"u\">{}{}</td><td class=\"u\">{}</td><td>{}</td></tr>",
                esc(&r.url),
                parents_html(r, 2),
                esc(r.final_url.as_deref().unwrap_or("")),
                esc(&chain)
            );
        }
        o.push_str("</table></details>");
    }

    // Orfani
    let orph: Vec<_> = s
        .results
        .iter()
        .filter(|r| r.status == Status::Orphan)
        .collect();
    if !orph.is_empty() {
        let _ = write!(
            o,
            "<h2>{} ({})</h2><details><summary>{}</summary><table><tr><th>{}</th><th>URL</th><th>{}</th></tr>",
            tr(lang, "Fișiere orfane", "Orphan files"),
            orph.len(),
            tr(lang, "Arată", "Show"),
            tr(lang, "Fișier local", "Local file"),
            tr(lang, "Mărime", "Size")
        );
        for r in orph {
            let _ = write!(
                o,
                "<tr><td class=\"u\">{}</td><td class=\"u\">{}</td><td>{}</td></tr>",
                esc(&r.message),
                esc(&r.url),
                r.size.map(human_size).unwrap_or_default()
            );
        }
        o.push_str("</table></details>");
    }

    // Observații
    let slow: Vec<_> = s
        .results
        .iter()
        .filter(|r| r.elapsed_ms >= s.config.slow_ms && r.status != Status::Orphan)
        .collect();
    if !sm.duplicate_titles.is_empty() || !sm.missing_titles.is_empty() || !slow.is_empty() {
        let _ = write!(o, "<h2>{}</h2>", tr(lang, "Observații", "Insights"));
        if !sm.duplicate_titles.is_empty() {
            let _ = write!(
                o,
                "<details><summary>{} ({})</summary><table>",
                tr(lang, "Titluri duplicate", "Duplicate titles"),
                sm.duplicate_titles.len()
            );
            for (t, urls) in &sm.duplicate_titles {
                let _ = write!(
                    o,
                    "<tr><td>{}</td><td class=\"u\">{}</td></tr>",
                    esc(t),
                    urls.iter().map(|u| esc(u)).collect::<Vec<_>>().join("<br>")
                );
            }
            o.push_str("</table></details>");
        }
        if !sm.missing_titles.is_empty() {
            let _ = write!(
                o,
                "<details><summary>{} ({})</summary><table>",
                tr(lang, "Pagini fără titlu", "Pages without a title"),
                sm.missing_titles.len()
            );
            for u in &sm.missing_titles {
                let _ = write!(o, "<tr><td class=\"u\">{}</td></tr>", esc(u));
            }
            o.push_str("</table></details>");
        }
        if !slow.is_empty() {
            let _ = write!(
                o,
                "<details><summary>{} ({})</summary><table>",
                tr(lang, "Răspunsuri lente", "Slow responses"),
                slow.len()
            );
            for r in slow {
                let _ = write!(
                    o,
                    "<tr><td class=\"u\">{}</td><td>{} ms</td></tr>",
                    esc(&r.url),
                    r.elapsed_ms
                );
            }
            o.push_str("</table></details>");
        }
    }

    // Statistici
    let _ = write!(
        o,
        "<h2>{}</h2><div class=\"cards\">",
        tr(lang, "Statistici", "Statistics")
    );
    let _ = write!(
        o,
        "<div class=\"card\"><b>{}</b><span>{}</span></div>",
        sm.internal,
        tr(lang, "interne", "internal")
    );
    let _ = write!(
        o,
        "<div class=\"card\"><b>{}</b><span>{}</span></div>",
        sm.external,
        tr(lang, "externe", "external")
    );
    let _ = write!(
        o,
        "<div class=\"card\"><b>{}</b><span>{}</span></div>",
        human_size(sm.total_bytes),
        tr(lang, "volum total", "total size")
    );
    let _ = write!(
        o,
        "<div class=\"card\"><b>{} ms</b><span>{}</span></div>",
        sm.avg_ms,
        tr(lang, "timp mediu", "avg. time")
    );
    let _ = write!(
        o,
        "<div class=\"card\"><b>{}</b><span>{}</span></div></div>",
        sm.max_depth,
        tr(lang, "adâncime maximă", "max depth")
    );
    let _ = write!(
        o,
        "<table><tr><th>{}</th><th>#</th><th>{}</th></tr>",
        tr(lang, "Tip MIME", "MIME type"),
        tr(lang, "Volum", "Size")
    );
    for (k, (n, b)) in &sm.by_type {
        let _ = write!(
            o,
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(k),
            n,
            human_size(*b)
        );
    }
    o.push_str("</table><br>");
    let _ = write!(
        o,
        "<table><tr><th>{}</th><th>#</th></tr>",
        tr(lang, "Stare", "Status")
    );
    for (k, n) in &sm.by_status {
        let _ = write!(o, "<tr><td>{}</td><td>{}</td></tr>", esc(k), n);
    }
    o.push_str("</table>");

    // Arbore
    let tree = to_tree_text(s, 8);
    if !tree.is_empty() {
        let _ = write!(
            o,
            "<h2>{}</h2><details><summary>{}</summary><pre>{}</pre></details>",
            tr(lang, "Harta site-ului", "Site map"),
            tr(lang, "Arată arborele", "Show tree"),
            esc(&tree)
        );
    }

    // Toate
    let _ = write!(
        o,
        "<h2>{} ({})</h2><details><summary>{}</summary><input type=\"search\" id=\"q\" placeholder=\"{}\"><table id=\"all\"><tr><th>URL</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        tr(lang, "Toate URL-urile", "All URLs"),
        s.results.len(),
        tr(lang, "Arată tabelul", "Show table"),
        tr(lang, "Filtrează…", "Filter…"),
        tr(lang, "Stare", "Status"),
        tr(lang, "Tip", "Type"),
        tr(lang, "Mărime", "Size"),
        tr(lang, "Titlu", "Title")
    );
    for r in &s.results {
        let _ = write!(
            o,
            "<tr><td class=\"u\">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&r.url),
            pill(r, &s.config),
            esc(r.kind.label(lang)),
            r.size.map(human_size).unwrap_or_default(),
            esc(r.title.as_deref().unwrap_or(""))
        );
    }
    o.push_str("</table></details>");
    o.push_str("<script>");
    o.push_str(REPORT_JS);
    o.push_str("</script>");
    let _ = write!(
        o,
        "<p class=\"sub\">Generat de VERIGA {}</p></main></body></html>",
        env!("CARGO_PKG_VERSION")
    );
    o
}

#[cfg(test)]
mod csp_tests {
    #[test]
    fn csp_hash_matches_known_vector() {
        // SHA-256("") în base64 = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
        assert_eq!(
            super::csp_hash(""),
            "'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn sample() -> Session {
        let mut a = LinkResult::new("https://ex.ro/", LinkKind::Start, 0, true);
        a.status = Status::Ok;
        a.content_type = Some("text/html".into());
        a.title = Some("Acasă".into());
        a.parsed = true;
        let mut b = LinkResult::new("https://ex.ro/lipsa", LinkKind::Page, 1, true);
        b.status = Status::NotFound;
        b.http_status = Some(404);
        b.message = "HTTP 404".into();
        b.parents.push(Parent {
            url: "https://ex.ro/".into(),
            text: "Lipsă & <test>".into(),
            kind: LinkKind::Page,
        });
        Session {
            version: 1,
            tool: "veriga".into(),
            started: "t0".into(),
            finished: "t1".into(),
            config: Config::with_start("https://ex.ro/"),
            results: vec![a, b],
            ignored: 0,
            truncated: false,
            cancelled: false,
        }
    }

    #[test]
    fn reports_render() {
        let s = sample();
        assert_eq!(Summary::from(&s).broken, 1);
        let t = to_text(&s);
        assert!(t.contains("https://ex.ro/lipsa"));
        let h = to_html(&s);
        assert!(h.contains("&lt;test&gt;"));
        assert!(!h.contains("<test>"));
        let c = to_csv(&s, b',', true);
        assert_eq!(c.lines().count(), 2);
        assert!(to_sitemap_xml(&s).contains("<loc>https://ex.ro/</loc>"));
        assert!(to_tree_text(&s, 5).contains("✗ https://ex.ro/lipsa"));
    }

    #[test]
    fn hostile_content_cannot_inject_markup_or_formulas() {
        let evil = "<script>alert(1)</script>\"'><img src=x onerror=alert(2)>";
        let mut s = sample();
        for r in s.results.iter_mut() {
            r.url = format!("https://ex.ro/{evil}");
            r.title = Some(format!("=HYPERLINK(\"http://evil\",\"x\")|{evil}"));
            r.message = evil.into();
            r.final_url = Some(format!("https://ex.ro/{evil}"));
            for p in r.parents.iter_mut() {
                p.url = format!("https://ex.ro/{evil}");
                p.text = evil.into();
            }
        }
        s.config.start = vec![evil.into()];
        let hist = vec![
            crate::insights::snapshot_of(&s),
            crate::insights::snapshot_of(&s),
        ];
        let pages = [
            to_html(&s),
            crate::insights::guardian_html(&s, Some(&s), &hist),
            crate::insights::compliance_html(
                &s,
                &crate::fixer::FixLog::default(),
                &hist,
                crate::insights::Period::All,
            ),
        ];
        for (i, h) in pages.iter().enumerate() {
            assert!(
                !h.contains("<script>alert"),
                "pagina {i}: <script> neescapat"
            );
            assert!(!h.contains("<img src=x"), "pagina {i}: <img> neescapat");
        }
        let csv = to_csv(&s, b',', false);
        assert!(
            !csv.contains(",=HYPERLINK") && !csv.contains(",\"=HYPERLINK"),
            "formulă neneutralizată: {csv}"
        );
        assert!(csv.contains("'=HYPERLINK"));
        let xml = to_sitemap_xml(&s);
        assert!(!xml.contains("<script>"), "sitemap XML neescapat");
    }
}
