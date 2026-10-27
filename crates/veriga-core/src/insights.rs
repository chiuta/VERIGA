//! Analize pe rezultate: domenii, hartă de sănătate, vedere „carte”, redirecționări,
//! diferențe între rulări (nou/rezolvat/deriva conținutului), istoric și tendințe,
//! raport de conformitate, import de 404 din jurnale de acces.

use crate::fixer::{FixLog, FixStatus};
use crate::model::*;
use crate::report::{CSS, esc, tr};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use url::Url;

fn host_of(u: &str) -> String {
    Url::parse(u)
        .ok()
        .and_then(|x| x.host_str().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_else(|| {
            if u.starts_with("file:") {
                "(local)".into()
            } else {
                "(?)".into()
            }
        })
}

// ───────────────────────── domenii ─────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct DomainStat {
    pub host: String,
    pub total: usize,
    pub broken: usize,
    pub review: usize,
    pub redirects: usize,
    pub avg_ms: u64,
    pub health: f64,
}

pub fn domain_stats(s: &Session) -> Vec<DomainStat> {
    let mut m: BTreeMap<String, (usize, usize, usize, usize, u64)> = BTreeMap::new();
    for r in &s.results {
        if matches!(r.status, Status::Orphan | Status::Pending) {
            continue;
        }
        let e = m.entry(host_of(&r.url)).or_default();
        e.0 += 1;
        if r.status.is_broken(&s.config) {
            e.1 += 1;
        }
        if r.status.needs_review(&s.config) {
            e.2 += 1;
        }
        if r.status == Status::Redirected {
            e.3 += 1;
        }
        e.4 += r.elapsed_ms;
    }
    let mut v: Vec<DomainStat> = m
        .into_iter()
        .map(|(host, (t, b, rv, rd, ms))| DomainStat {
            host,
            total: t,
            broken: b,
            review: rv,
            redirects: rd,
            avg_ms: ms / t.max(1) as u64,
            health: if t == 0 {
                100.0
            } else {
                100.0 * (t - b) as f64 / t as f64
            },
        })
        .collect();
    v.sort_by(|a, b| b.broken.cmp(&a.broken).then(b.total.cmp(&a.total)));
    v
}

// ───────────────────────── hartă de sănătate / carte ─────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct PageHealth {
    pub page: String,
    pub title: Option<String>,
    pub links: usize,
    pub broken: usize,
    pub review: usize,
    pub score: f64,
    pub items: Vec<BrokenItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BrokenItem {
    pub url: String,
    pub text: String,
    pub status: Status,
    pub message: String,
    pub review: bool,
}

pub fn health_map(s: &Session) -> Vec<PageHealth> {
    let titles: HashMap<&str, &Option<String>> = s
        .results
        .iter()
        .map(|r| (r.url.as_str(), &r.title))
        .collect();
    let mut m: BTreeMap<String, PageHealth> = BTreeMap::new();
    for r in &s.results {
        if matches!(r.status, Status::Orphan | Status::Pending) {
            continue;
        }
        let bad = r.status.is_broken(&s.config);
        let rev = r.status.needs_review(&s.config);
        for p in &r.parents {
            let e = m.entry(p.url.clone()).or_insert_with(|| PageHealth {
                page: p.url.clone(),
                title: titles.get(p.url.as_str()).and_then(|t| (*t).clone()),
                links: 0,
                broken: 0,
                review: 0,
                score: 100.0,
                items: vec![],
            });
            e.links += 1;
            if bad {
                e.broken += 1;
            }
            if rev {
                e.review += 1;
            }
            if bad || rev {
                e.items.push(BrokenItem {
                    url: r.url.clone(),
                    text: p.text.clone(),
                    status: r.status,
                    message: r.message.clone(),
                    review: rev && !bad,
                });
            }
        }
    }
    let mut v: Vec<PageHealth> = m
        .into_values()
        .map(|mut p| {
            p.score = 100.0 * (p.links - p.broken) as f64 / p.links.max(1) as f64;
            p
        })
        .collect();
    v.sort_by(|a, b| {
        b.broken
            .cmp(&a.broken)
            .then(a.score.partial_cmp(&b.score).unwrap())
    });
    v
}

/// Redirecționări: (de la, la, coduri, permanent?, pagini).
pub fn redirects(s: &Session) -> Vec<(String, String, Vec<u16>, bool, usize)> {
    s.results
        .iter()
        .filter(|r| r.status == Status::Redirected && !r.chain.is_empty())
        .map(|r| {
            let codes: Vec<u16> = r.chain.iter().map(|h| h.code).collect();
            let perm = codes.iter().all(|c| matches!(c, 301 | 308));
            (
                r.url.clone(),
                r.final_url.clone().unwrap_or_default(),
                codes,
                perm,
                r.parents.len(),
            )
        })
        .collect()
}

/// Impact: link în meniu (aproximat: apare pe ≥ 3 pagini) sau pe ≥ 3 pagini.
pub fn impact_pages(r: &LinkResult) -> usize {
    r.parents
        .iter()
        .map(|p| &p.url)
        .collect::<BTreeSet<_>>()
        .len()
}

// ───────────────────────── diferențe între rulări ─────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct Drift {
    pub url: String,
    pub old_title: Option<String>,
    pub new_title: Option<String>,
    pub old_h1: Option<String>,
    pub new_h1: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Diff {
    pub new_broken: Vec<String>,
    pub resolved: Vec<String>,
    pub drift: Vec<Drift>,
}

fn words(s: &str) -> BTreeSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Similaritate Jaccard pe cuvinte (1.0 = identic).
pub fn similarity(a: &str, b: &str) -> f64 {
    let (x, y) = (words(a), words(b));
    if x.is_empty() && y.is_empty() {
        return 1.0;
    }
    let i = x.intersection(&y).count() as f64;
    let u = x.union(&y).count() as f64;
    i / u
}

pub fn diff(old: &Session, new: &Session) -> Diff {
    let ob: BTreeSet<&str> = old.broken().map(|r| r.url.as_str()).collect();
    let nb: BTreeSet<&str> = new.broken().map(|r| r.url.as_str()).collect();
    let oldmap: HashMap<&str, &LinkResult> =
        old.results.iter().map(|r| (r.url.as_str(), r)).collect();
    let mut d = Diff {
        new_broken: nb.difference(&ob).map(|s| s.to_string()).collect(),
        resolved: ob.difference(&nb).map(|s| s.to_string()).collect(),
        drift: vec![],
    };
    for r in &new.results {
        if r.status.is_broken(&new.config) || r.title.is_none() && r.h1.is_none() {
            continue;
        }
        if let Some(o) = oldmap.get(r.url.as_str()) {
            if o.title.is_none() && o.h1.is_none() {
                continue;
            }
            let a = format!(
                "{} {}",
                o.title.as_deref().unwrap_or(""),
                o.h1.as_deref().unwrap_or("")
            );
            let b = format!(
                "{} {}",
                r.title.as_deref().unwrap_or(""),
                r.h1.as_deref().unwrap_or("")
            );
            // deriva = schimbare semnificativă (parcare de domeniu, pagină înlocuită)
            if similarity(&a, &b) < 0.34 {
                d.drift.push(Drift {
                    url: r.url.clone(),
                    old_title: o.title.clone(),
                    new_title: r.title.clone(),
                    old_h1: o.h1.clone(),
                    new_h1: r.h1.clone(),
                });
            }
        }
    }
    d
}

// ───────────────────────── istoric / tendințe ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub time: String,
    pub start: String,
    pub total: usize,
    pub broken: usize,
    pub review: usize,
    pub redirects: usize,
    pub orphans: usize,
    pub health: f64,
}

pub fn snapshot_of(s: &Session) -> Snapshot {
    let total = s
        .results
        .iter()
        .filter(|r| !matches!(r.status, Status::Orphan | Status::Pending))
        .count();
    let broken = s.broken_count();
    Snapshot {
        time: if s.finished.is_empty() {
            s.started.clone()
        } else {
            s.finished.clone()
        },
        start: s.config.start.join(", "),
        total,
        broken,
        review: s
            .results
            .iter()
            .filter(|r| r.status.needs_review(&s.config))
            .count(),
        redirects: s
            .results
            .iter()
            .filter(|r| r.status == Status::Redirected)
            .count(),
        orphans: s
            .results
            .iter()
            .filter(|r| r.status == Status::Orphan)
            .count(),
        health: if total == 0 {
            100.0
        } else {
            100.0 * (total - broken) as f64 / total as f64
        },
    }
}

pub fn history_append(path: &Path, s: &Session) -> anyhow::Result<Snapshot> {
    use std::io::Write;
    if let Some(p) = path.parent()
        && !p.as_os_str().is_empty()
    {
        std::fs::create_dir_all(p)?;
    }
    let snap = snapshot_of(s);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(&snap)?)?;
    Ok(snap)
}

pub fn history_load(path: &Path) -> Vec<Snapshot> {
    std::fs::read_to_string(path)
        .map(|t| {
            t.lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Grafic SVG simplu (linie) pentru tendința de link-uri rupte.
pub fn trend_svg(h: &[Snapshot], lang: &str) -> String {
    if h.len() < 2 {
        return format!(
            "<p class=\"sub\">{}</p>",
            tr(
                lang,
                "Sunt necesare cel puțin două rulări pentru tendință.",
                "At least two runs are needed for a trend."
            )
        );
    }
    let (w, ht, pad) = (700.0, 180.0, 28.0);
    let max = h.iter().map(|s| s.broken).max().unwrap_or(1).max(1) as f64;
    let pts: Vec<String> = h
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let x = pad + (w - 2.0 * pad) * i as f64 / (h.len() - 1) as f64;
            let y = ht - pad - (ht - 2.0 * pad) * s.broken as f64 / max;
            format!("{x:.1},{y:.1}")
        })
        .collect();
    format!(
        "<svg viewBox=\"0 0 {w} {ht}\" role=\"img\" aria-label=\"{}\" style=\"width:100%;max-width:720px\"><polyline fill=\"none\" stroke=\"var(--acc)\" stroke-width=\"2\" points=\"{}\"/><text x=\"{pad}\" y=\"14\" fill=\"var(--mut)\" font-size=\"11\">max {}</text></svg>",
        tr(lang, "Link-uri rupte în timp", "Broken links over time"),
        pts.join(" "),
        max as usize
    )
}

// ───────────────────────── conformitate ─────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum Period {
    Days(u32),
    All,
}

pub fn compliance_html(s: &Session, log: &FixLog, hist: &[Snapshot], period: Period) -> String {
    let lang = s.config.lang.as_str();
    let cutoff = match period {
        Period::Days(d) => Some(chrono::Local::now() - chrono::Duration::days(d as i64)),
        Period::All => None,
    };
    let in_period = |t: &str| -> bool {
        match (cutoff, chrono::DateTime::parse_from_rfc3339(t)) {
            (Some(c), Ok(t)) => t.with_timezone(&chrono::Local) >= c,
            _ => true,
        }
    };
    let runs: Vec<&Snapshot> = hist.iter().filter(|h| in_period(&h.time)).collect();
    let fixes: Vec<_> = log
        .entries
        .iter()
        .filter(|e| e.status == FixStatus::Applied && in_period(&e.time))
        .collect();
    let undone = log
        .entries
        .iter()
        .filter(|e| e.status == FixStatus::Undone && in_period(&e.time))
        .count();
    let sn = snapshot_of(s);
    let mut o = String::new();
    let _ = write!(
        o,
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'\"><meta name=\"color-scheme\" content=\"light dark\"><title>VERIGA — {}</title><style>{CSS}</style></head><body><main>",
        tr(lang, "raport de conformitate", "compliance report")
    );
    let _ = write!(
        o,
        "<h1>{}</h1><div class=\"sub\">{} · {}</div>",
        tr(
            lang,
            "Raport de conformitate — întreținerea legăturilor",
            "Compliance report — link maintenance"
        ),
        esc(&s.config.start.join(", ")),
        match period {
            Period::Days(d) => format!(
                "{} {d} {}",
                tr(lang, "ultimele", "last"),
                tr(lang, "zile", "days")
            ),
            Period::All => tr(lang, "toată perioada", "all time").to_string(),
        }
    );
    let _ = write!(
        o,
        "<div class=\"cards\"><div class=\"card\"><b>{}</b><span>{}</span></div><div class=\"card okc\"><b>{}</b><span>{}</span></div><div class=\"card bad\"><b>{}</b><span>{}</span></div><div class=\"card warn\"><b>{}</b><span>{}</span></div><div class=\"card\"><b>{:.1}%</b><span>{}</span></div></div>",
        runs.len(),
        tr(lang, "verificări în perioadă", "checks in period"),
        fixes.len(),
        tr(lang, "reparații aplicate", "fixes applied"),
        sn.broken,
        tr(lang, "link-uri rupte acum", "broken now"),
        sn.review,
        tr(lang, "de verificat manual", "to review"),
        sn.health,
        tr(lang, "sănătate", "health")
    );
    let _ = write!(
        o,
        "<p>{}</p>",
        match lang {
            "en" => format!(
                "Fixes undone in period: {undone}. Every change is backed up and can be reverted from the fix log."
            ),
            _ => format!(
                "Reparații anulate în perioadă: {undone}. Fiecare modificare are copie de siguranță și poate fi anulată din jurnal."
            ),
        }
    );
    if !runs.is_empty() {
        let _ = write!(
            o,
            "<h2>{}</h2>{}<table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
            tr(lang, "Verificări", "Checks"),
            trend_svg(hist, lang),
            tr(lang, "Data", "Date"),
            tr(lang, "Total", "Total"),
            tr(lang, "Rupte", "Broken"),
            tr(lang, "Sănătate", "Health")
        );
        for h in runs.iter().rev().take(50) {
            let _ = write!(
                o,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td></tr>",
                esc(&h.time),
                h.total,
                h.broken,
                h.health
            );
        }
        o.push_str("</table>");
    }
    if !fixes.is_empty() {
        let _ = write!(
            o,
            "<h2>{}</h2><table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
            tr(lang, "Reparații", "Fixes"),
            tr(lang, "Data", "Date"),
            tr(lang, "Regulă", "Rule"),
            tr(lang, "Înainte", "Before"),
            tr(lang, "După", "After")
        );
        for e in fixes.iter().rev().take(500) {
            let _ = write!(
                o,
                "<tr><td>{}</td><td>{}</td><td class=\"u\">{}</td><td class=\"u\">{}</td></tr>",
                esc(&e.time),
                e.rule.label(lang),
                esc(&e.old),
                esc(&e.new)
            );
        }
        o.push_str("</table>");
    }
    let open: Vec<&LinkResult> = s.broken().collect();
    if !open.is_empty() {
        let _ = write!(
            o,
            "<h2>{} ({})</h2><table><tr><th>URL</th><th>{}</th><th>{}</th></tr>",
            tr(lang, "Probleme deschise", "Open issues"),
            open.len(),
            tr(lang, "Stare", "Status"),
            tr(lang, "Pagini", "Pages")
        );
        for r in open.iter().take(500) {
            let _ = write!(
                o,
                "<tr><td class=\"u\">{}</td><td>{}</td><td>{}</td></tr>",
                esc(&r.url),
                r.status.label(lang),
                impact_pages(r)
            );
        }
        o.push_str("</table>");
    }
    o.push_str("</main></body></html>");
    o
}

// ───────────────────────── vederi HTML combinate ─────────────────────────

/// Pagină HTML cu: tablou de bord, domenii, hartă de sănătate, carte, redirecționări, diferențe.
pub fn guardian_html(s: &Session, prev: Option<&Session>, hist: &[Snapshot]) -> String {
    let lang = s.config.lang.as_str();
    let sn = snapshot_of(s);
    let mut o = String::new();
    let _ = write!(
        o,
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'\"><meta name=\"color-scheme\" content=\"light dark\"><title>VERIGA — Guardian</title><style>{CSS}.bar{{height:8px;border-radius:4px;background:var(--bd);overflow:hidden}}.bar i{{display:block;height:100%;background:var(--ok)}}</style></head><body><main>"
    );
    let _ = write!(
        o,
        "<h1>VERIGA Guardian</h1><div class=\"sub\">{}</div>",
        esc(&s.config.start.join(", "))
    );
    let _ = write!(
        o,
        "<div class=\"cards\"><div class=\"card\"><b>{}</b><span>{}</span></div><div class=\"card bad\"><b>{}</b><span>{}</span></div><div class=\"card warn\"><b>{}</b><span>{}</span></div><div class=\"card\"><b>{}</b><span>{}</span></div><div class=\"card okc\"><b>{:.1}%</b><span>{}</span></div></div>",
        sn.total,
        tr(lang, "verificate", "checked"),
        sn.broken,
        tr(lang, "rupte", "broken"),
        sn.review,
        tr(lang, "de verificat (403)", "to review (403)"),
        sn.redirects,
        tr(lang, "redirecționate", "redirected"),
        sn.health,
        tr(lang, "sănătate", "health")
    );
    // diferențe
    if let Some(p) = prev {
        let d = diff(p, s);
        let _ = write!(
            o,
            "<h2>{}</h2><p>{}: <b>{}</b> · {}: <b>{}</b> · {}: <b>{}</b></p>",
            tr(lang, "Față de rularea anterioară", "Since the previous run"),
            tr(lang, "noi rupte", "newly broken"),
            d.new_broken.len(),
            tr(lang, "rezolvate", "resolved"),
            d.resolved.len(),
            tr(lang, "derivă de conținut", "content drift"),
            d.drift.len()
        );
        if !d.drift.is_empty() {
            let _ = write!(
                o,
                "<table><tr><th>URL</th><th>{}</th><th>{}</th></tr>",
                tr(lang, "Înainte", "Before"),
                tr(lang, "Acum", "Now")
            );
            for x in &d.drift {
                let _ = write!(
                    o,
                    "<tr><td class=\"u\">{}</td><td>{}<span class=\"par\">{}</span></td><td>{}<span class=\"par\">{}</span></td></tr>",
                    esc(&x.url),
                    esc(x.old_title.as_deref().unwrap_or("")),
                    esc(x.old_h1.as_deref().unwrap_or("")),
                    esc(x.new_title.as_deref().unwrap_or("")),
                    esc(x.new_h1.as_deref().unwrap_or(""))
                );
            }
            o.push_str("</table>");
        }
    }
    let _ = write!(
        o,
        "<h2>{}</h2>{}",
        tr(lang, "Tendințe", "Trends"),
        trend_svg(hist, lang)
    );
    // domenii
    let _ = write!(
        o,
        "<h2>{}</h2><table><tr><th>{}</th><th>{}</th><th>{}</th><th>403</th><th>{}</th><th>ms</th><th>{}</th></tr>",
        tr(lang, "După domeniu", "By domain"),
        tr(lang, "Gazdă", "Host"),
        tr(lang, "Total", "Total"),
        tr(lang, "Rupte", "Broken"),
        tr(lang, "Redir.", "Redir."),
        tr(lang, "Sănătate", "Health")
    );
    for d in domain_stats(s).iter().take(300) {
        let _ = write!(
            o,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><div class=\"bar\"><i style=\"width:{:.0}%\"></i></div>{:.0}%</td></tr>",
            esc(&d.host),
            d.total,
            d.broken,
            d.review,
            d.redirects,
            d.avg_ms,
            d.health,
            d.health
        );
    }
    o.push_str("</table>");
    // hartă + carte
    let hm = health_map(s);
    let _ = write!(
        o,
        "<h2>{}</h2><table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        tr(
            lang,
            "Harta de sănătate (pe pagini)",
            "Health map (per page)"
        ),
        tr(lang, "Pagină", "Page"),
        tr(lang, "Linkuri", "Links"),
        tr(lang, "Rupte", "Broken"),
        tr(lang, "Scor", "Score")
    );
    for p in hm.iter().take(500) {
        let _ = write!(
            o,
            "<tr><td class=\"u\">{}<span class=\"par\">{}</span></td><td>{}</td><td>{}</td><td><div class=\"bar\"><i style=\"width:{:.0}%\"></i></div>{:.0}%</td></tr>",
            esc(&p.page),
            esc(p.title.as_deref().unwrap_or("")),
            p.links,
            p.broken,
            p.score,
            p.score
        );
    }
    o.push_str("</table>");
    let _ = write!(
        o,
        "<h2>{}</h2>",
        tr(
            lang,
            "Vedere „carte” (probleme pe pagină)",
            "Book view (issues per page)"
        )
    );
    for p in hm.iter().filter(|p| !p.items.is_empty()).take(300) {
        let _ = write!(
            o,
            "<details><summary>{} — {}</summary><table><tr><th>{}</th><th>URL</th><th>{}</th></tr>",
            esc(p.title.as_deref().unwrap_or(&p.page)),
            p.items.len(),
            tr(lang, "Text", "Text"),
            tr(lang, "Stare", "Status")
        );
        for i in &p.items {
            let _ = write!(
                o,
                "<tr><td>{}</td><td class=\"u\">{}</td><td><span class=\"pill {}\">{}</span><span class=\"par\">{}</span></td></tr>",
                esc(&i.text),
                esc(&i.url),
                if i.review { "p-warn" } else { "p-bad" },
                i.status.label(lang),
                esc(&i.message)
            );
        }
        o.push_str("</table></details>");
    }
    // redirecționări
    let rd = redirects(s);
    if !rd.is_empty() {
        let _ = write!(
            o,
            "<h2>{} ({})</h2><table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
            tr(lang, "Redirecționări", "Redirects"),
            rd.len(),
            tr(lang, "De la", "From"),
            tr(lang, "La", "To"),
            tr(lang, "Tip", "Type"),
            tr(lang, "Pagini", "Pages")
        );
        for (a, b, c, perm, n) in rd.iter().take(500) {
            let _ = write!(
                o,
                "<tr><td class=\"u\">{}</td><td class=\"u\">{}</td><td>{} {}</td><td>{n}</td></tr>",
                esc(a),
                esc(b),
                c.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("→"),
                if *perm {
                    tr(lang, "permanent", "permanent")
                } else {
                    tr(lang, "temporar", "temporary")
                }
            );
        }
        o.push_str("</table>");
    }
    o.push_str("</main></body></html>");
    o
}

// ───────────────────────── 404 din jurnale de acces ─────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct Hit404 {
    pub path: String,
    pub count: usize,
    pub last: String,
    pub referers: Vec<(String, usize)>,
    pub suggestion: Option<String>,
}

fn log_re() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        regex::Regex::new(r#"\[([^\]]+)\]\s+"(?:GET|HEAD)\s+(\S+)\s+HTTP/[\d.]+"\s+(\d{3})\s+\S+(?:\s+"([^"]*)"\s+"([^"]*)")?"#).unwrap()
    })
}

fn is_bot(ua: &str) -> bool {
    let l = ua.to_ascii_lowercase();
    [
        "bot",
        "spider",
        "crawl",
        "slurp",
        "curl/",
        "wget",
        "python-requests",
        "scanner",
    ]
    .iter()
    .any(|b| l.contains(b))
}

fn lev(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push(
                (prev[j] + (ca != cb) as usize)
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Cel mai apropiat path cunoscut: potrivire pe ultimul segment (≤ 25% distanță),
/// apoi pe calea întreagă (≤ 35%).
pub fn closest(path: &str, known: &[String]) -> Option<String> {
    let p = path
        .split('?')
        .next()
        .unwrap_or(path)
        .trim_end_matches('/')
        .to_ascii_lowercase();
    let last = |x: &str| x.rsplit('/').next().unwrap_or("").to_string();
    if p.len() > MAX_404_PATH {
        return None;
    }
    let pl = last(&p);
    let (pc, plc) = (p.chars().count(), pl.chars().count());
    let mut best: Option<((usize, usize), &String)> = None;
    for k in known {
        if k.len() > MAX_404_PATH {
            continue;
        }
        let kk = k.trim_end_matches('/').to_ascii_lowercase();
        if kk == p {
            continue;
        }
        // prefiltru ieftin: distanța Levenshtein ≥ diferența de lungime
        let kl = last(&kk);
        let full_lb = pc.abs_diff(kk.chars().count());
        let seg_lb = plc.abs_diff(kl.chars().count());
        if full_lb * 100 > p.len().max(kk.len()) * 35
            && (pl.is_empty() || seg_lb * 100 > pl.len().max(kl.len()) * 25)
        {
            continue;
        }
        let full = lev(&p, &kk);
        let seg = lev(&pl, &last(&kk));
        let seg_ok = !pl.is_empty() && seg * 100 <= pl.len().max(last(&kk).len()) * 25;
        let full_ok = full * 100 <= p.len().max(kk.len()) * 35;
        if seg_ok || full_ok {
            let score = (if seg_ok { seg } else { 99 }, full);
            if best.map(|(b, _)| score < b).unwrap_or(true) {
                best = Some((score, k));
            }
        }
    }
    best.map(|(_, k)| k.clone())
}

/// Căile mai lungi decât atât nu se compară (protecție CPU).
const MAX_404_PATH: usize = 512;
/// Numărul maxim de căi distincte reținute din jurnale.
const MAX_404_DISTINCT: usize = 20_000;

/// Parsează jurnale Apache/nginx (format common/combined) și numără 404-urile.
pub fn parse_404(text: &str, ignore_bots: bool, known_paths: &[String]) -> Vec<Hit404> {
    let mut m: HashMap<String, (usize, String, HashMap<String, usize>)> = HashMap::new();
    for line in text.lines() {
        let Some(c) = log_re().captures(line) else {
            continue;
        };
        if &c[3] != "404" {
            continue;
        }
        if ignore_bots && c.get(5).map(|u| is_bot(u.as_str())).unwrap_or(false) {
            continue;
        }
        let path = c[2].to_string();
        if path.ends_with("/favicon.ico") || path.len() > MAX_404_PATH {
            continue;
        }
        if m.len() >= MAX_404_DISTINCT && !m.contains_key(&path) {
            continue;
        }
        let e = m
            .entry(path)
            .or_insert_with(|| (0, String::new(), HashMap::new()));
        e.0 += 1;
        e.1 = c[1].to_string();
        if let Some(r) = c.get(4) {
            let r = r.as_str();
            if r != "-" && !r.is_empty() && (e.2.len() < 100 || e.2.contains_key(r)) {
                *e.2.entry(r.to_string()).or_default() += 1;
            }
        }
    }
    let mut v: Vec<Hit404> = m
        .into_iter()
        .map(|(path, (count, last, refs))| {
            let mut referers: Vec<_> = refs.into_iter().collect();
            referers.sort_by_key(|r| std::cmp::Reverse(r.1));
            referers.truncate(5);
            Hit404 {
                suggestion: closest(&path, known_paths),
                path,
                count,
                last,
                referers,
            }
        })
        .collect();
    v.sort_by_key(|h| std::cmp::Reverse(h.count));
    v
}

pub fn hits_csv(h: &[Hit404]) -> String {
    use crate::fixer::csv_safe;
    let mut w = csv::Writer::from_writer(vec![]);
    let _ = w.write_record(["path", "count", "last", "top_referer", "suggested_redirect"]);
    for x in h {
        let _ = w.write_record([
            csv_safe(&x.path),
            x.count.to_string(),
            csv_safe(&x.last),
            csv_safe(x.referers.first().map(|r| r.0.as_str()).unwrap_or("")),
            csv_safe(x.suggestion.as_deref().unwrap_or("")),
        ]);
    }
    String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
}

/// Reguli `Redirect 301` pentru Apache din 404-urile cu sugestie.
pub fn htaccess_rules(h: &[Hit404]) -> String {
    let mut o = String::from("# generat de VERIGA — verificați înainte de a folosi\n");
    for x in h.iter().filter(|x| x.suggestion.is_some()) {
        let p = x.path.split('?').next().unwrap_or(&x.path);
        if p.chars().all(|c| c.is_ascii_graphic()) && !p.contains(' ') {
            let _ = writeln!(o, "Redirect 301 {} {}", p, x.suggestion.as_deref().unwrap());
        }
    }
    o
}

pub fn default_paths(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        dir.join("veriga-history.jsonl"),
        dir.join("veriga-fixlog.json"),
        dir.join(".veriga-backup"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_404_log() {
        let log = r#"1.2.3.4 - - [10/Oct/2026:13:55:36 +0000] "GET /vechi/pagina HTTP/1.1" 404 153 "https://x.ro/a" "Mozilla/5.0"
1.2.3.4 - - [10/Oct/2026:13:56:00 +0000] "GET /vechi/pagina HTTP/1.1" 404 153 "https://x.ro/b" "Mozilla/5.0"
5.6.7.8 - - [10/Oct/2026:13:57:00 +0000] "GET /alt HTTP/1.1" 404 153 "-" "Googlebot/2.1"
5.6.7.8 - - [10/Oct/2026:13:57:00 +0000] "GET /ok HTTP/1.1" 200 153 "-" "Mozilla/5.0""#;
        let known = vec!["/nou/pagina".to_string(), "/despre".to_string()];
        let h = parse_404(log, true, &known);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].count, 2);
        assert_eq!(h[0].referers.len(), 2);
        assert_eq!(h[0].suggestion.as_deref(), Some("/nou/pagina"));
        let all = parse_404(log, false, &known);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn drift_detection() {
        let mk = |t: &str, h: &str| {
            let mut r = LinkResult::new("https://x.ro/", LinkKind::Page, 0, false);
            r.status = Status::Ok;
            r.title = Some(t.into());
            r.h1 = Some(h.into());
            Session {
                version: 1,
                tool: "t".into(),
                started: String::new(),
                finished: String::new(),
                config: crate::Config::default(),
                results: vec![r],
                ignored: 0,
                truncated: false,
                cancelled: false,
            }
        };
        let a = mk(
            "Cofetăria Dulcea — prăjituri artizanale",
            "Prăjituri artizanale",
        );
        let b = mk("Domeniu de vânzare", "Acest domeniu este de vânzare");
        let c = mk(
            "Cofetăria Dulcea — prăjituri artizanale noi",
            "Prăjituri artizanale",
        );
        assert_eq!(diff(&a, &b).drift.len(), 1);
        assert_eq!(diff(&a, &c).drift.len(), 0);
    }

    #[test]
    fn forbidden_is_review_not_broken() {
        let mut r = LinkResult::new("https://x.ro/", LinkKind::Page, 0, false);
        r.status = Status::Forbidden;
        let cfg = crate::Config::default();
        assert!(!r.status.is_broken(&cfg));
        assert!(r.status.needs_review(&cfg));
    }

    #[test]
    fn log404_limits_path_length_and_skips_far_candidates() {
        let long = format!("/{}", "a".repeat(2000));
        let log = format!(
            "1.2.3.4 - - [10/Oct/2026:13:55:36 +0000] \"GET {long} HTTP/1.1\" 404 1 \"-\" \"Mozilla\"\n"
        );
        assert!(parse_404(&log, true, &["/a".into()]).is_empty());
        assert_eq!(closest(&long, &["/a".into()]), None);
        // prefiltrul nu pierde potrivirile apropiate
        assert_eq!(
            closest("/contactt", &["/contact".into(), "/x".repeat(300)]),
            Some("/contact".into())
        );
    }
}
