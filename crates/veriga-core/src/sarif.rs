//! Export SARIF 2.1.0 al rezultatelor unei verificări.
//!
//! Documentul este determinist (fără marcaje de timp, chei în ordine alfabetică, rezultate
//! sortate), ca două rulări pe aceleași date să dea octeți identici.

use crate::model::{LinkResult, Session, Status};
use serde_json::{Map, Value, json};

pub const SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";

/// Nivelurile permise de standard.
pub const LEVELS: [&str; 4] = ["none", "note", "warning", "error"];

/// Regulile proprii: (id, nivel implicit, descriere scurtă).
const RULES: [(&str, &str, &str); 6] = [
    ("veriga/broken-link", "error", "Broken link"),
    ("veriga/redirect", "warning", "Link redirects elsewhere"),
    (
        "veriga/needs-review",
        "warning",
        "Access denied; needs a manual look",
    ),
    ("veriga/slow", "warning", "Slow response"),
    ("veriga/skipped", "note", "Link not checked"),
    ("veriga/orphan", "note", "File not reachable by any link"),
];

/// O constatare, independentă de sursa ei (permite și locații cu linie cunoscută).
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub rule: &'static str,
    pub level: &'static str,
    pub message: String,
    /// Fișierul sau pagina sursă (uri); `None` dacă nu e cunoscută.
    pub source: Option<String>,
    /// Linia (de la 1) dacă este cunoscută.
    pub line: Option<u32>,
    pub target: String,
    pub http_status: Option<u16>,
    pub status: String,
    pub kind: String,
}

/// Uri valid: adresele parsabile se serializează canonic (non-ASCII devine procent-codat),
/// restul au octeții nesiguri procent-codați.
pub fn uri_of(s: &str) -> String {
    if let Ok(u) = url::Url::parse(s) {
        return u.to_string();
    }
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~:/?#[]@!$&'()*+,;=%".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

fn classify(r: &LinkResult, s: &Session) -> Option<(&'static str, &'static str)> {
    let cfg = &s.config;
    if r.status.is_broken(cfg) {
        return Some(("veriga/broken-link", "error"));
    }
    match r.status {
        Status::Redirected => Some(("veriga/redirect", "warning")),
        Status::Forbidden => Some(("veriga/needs-review", "warning")),
        Status::Skipped | Status::RobotsBlocked => Some(("veriga/skipped", "note")),
        Status::Orphan => Some(("veriga/orphan", "note")),
        Status::Ok if cfg.slow_ms > 0 && r.elapsed_ms > cfg.slow_ms => {
            Some(("veriga/slow", "warning"))
        }
        _ => None,
    }
}

/// Constatările unei sesiuni (legăturile bune nu se emit). O constatare per legătură și pagină sursă.
pub fn findings(s: &Session) -> Vec<Finding> {
    let lang = "en";
    let mut v = vec![];
    for r in &s.results {
        let Some((rule, level)) = classify(r, s) else {
            continue;
        };
        let mut message = format!("{}: {}", r.status.label(lang), r.url);
        if !r.message.is_empty() {
            message.push_str(" — ");
            message.push_str(&r.message);
        }
        let mk = |source: Option<String>| Finding {
            rule,
            level,
            message: message.clone(),
            source,
            line: None,
            target: r.url.clone(),
            http_status: r.http_status,
            status: r.status.label(lang).to_string(),
            kind: r.kind.label(lang).to_string(),
        };
        if r.parents.is_empty() {
            // fără pagină sursă: locația este chiar adresa verificată
            v.push(mk(Some(r.url.clone())));
        } else {
            let mut seen: Vec<&str> = vec![];
            for p in &r.parents {
                if !seen.contains(&p.url.as_str()) {
                    seen.push(&p.url);
                    v.push(mk(Some(p.url.clone())));
                }
            }
        }
    }
    v
}

fn result_value(f: &Finding) -> Value {
    let mut loc = Map::new();
    if let Some(src) = &f.source {
        let mut phys = Map::new();
        phys.insert("artifactLocation".into(), json!({ "uri": uri_of(src) }));
        if let Some(l) = f.line.filter(|l| *l >= 1) {
            phys.insert("region".into(), json!({ "startLine": l }));
        }
        loc.insert("physicalLocation".into(), Value::Object(phys));
    }
    let mut props = Map::new();
    props.insert("targetUrl".into(), json!(f.target));
    props.insert("status".into(), json!(f.status));
    props.insert("kind".into(), json!(f.kind));
    if let Some(c) = f.http_status {
        props.insert("httpStatus".into(), json!(c));
    }
    json!({
        "ruleId": f.rule,
        "level": f.level,
        "message": { "text": f.message },
        "locations": [Value::Object(loc)],
        "properties": Value::Object(props),
    })
}

/// Documentul SARIF pentru un set de constatări.
pub fn build(mut fs: Vec<Finding>) -> Value {
    fs.sort_by(|a, b| {
        (&a.source, a.line, &a.target, a.rule).cmp(&(&b.source, b.line, &b.target, b.rule))
    });
    let rules: Vec<Value> = RULES
        .iter()
        .map(|(id, lvl, d)| {
            json!({
                "id": id,
                "shortDescription": { "text": d },
                "defaultConfiguration": { "level": lvl },
            })
        })
        .collect();
    json!({
        "$schema": SCHEMA_URI,
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "VERIGA",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules,
            }},
            "results": fs.iter().map(result_value).collect::<Vec<_>>(),
        }],
    })
}

/// Raportul SARIF (JSON UTF-8) al unei sesiuni.
pub fn to_sarif(s: &Session) -> String {
    let s = &s.redacted();
    serde_json::to_string_pretty(&build(findings(s))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LinkKind, Parent};

    fn base_finding() -> Finding {
        Finding {
            rule: "veriga/broken-link",
            level: "error",
            message: "m".into(),
            source: Some("file:///site/index.html".into()),
            line: Some(12),
            target: "http://x.org/a".into(),
            http_status: Some(404),
            status: "Not found (404)".into(),
            kind: "Page".into(),
        }
    }

    /// Verificare structurală a părților obligatorii din standard (fără schema oficială).
    fn assert_structurally_valid(v: &Value) {
        assert_eq!(v["version"], "2.1.0");
        let runs = v["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 1);
        assert!(runs[0]["tool"]["driver"]["name"].is_string());
        for r in runs[0]["results"].as_array().unwrap() {
            assert!(LEVELS.contains(&r["level"].as_str().unwrap()));
            assert!(r["message"]["text"].is_string());
            assert!(r["ruleId"].is_string());
            for l in r["locations"].as_array().unwrap() {
                if let Some(reg) = l["physicalLocation"].get("region") {
                    assert!(reg["startLine"].as_u64().unwrap() >= 1);
                }
                assert!(l["physicalLocation"]["artifactLocation"]["uri"].is_string());
            }
        }
        for rule in runs[0]["tool"]["driver"]["rules"].as_array().unwrap() {
            assert!(rule["id"].is_string());
            assert!(rule["shortDescription"]["text"].is_string());
        }
    }

    #[test]
    fn t1_empty_run() {
        let v = build(vec![]);
        assert_structurally_valid(&v);
        assert_eq!(v["runs"][0]["results"], json!([]));
    }

    #[test]
    fn t2_line_known() {
        let v = build(vec![base_finding()]);
        let r = &v["runs"][0]["results"][0];
        assert_eq!(r["level"], "error");
        assert_eq!(
            r["locations"][0]["physicalLocation"]["region"]["startLine"],
            12
        );
        assert_structurally_valid(&v);
    }

    #[test]
    fn t3_no_line_no_region() {
        let mut f = base_finding();
        f.line = None;
        let v = build(vec![f.clone()]);
        assert!(
            v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
                .get("region")
                .is_none()
        );
        f.line = Some(0); // 0 nu e o linie validă: regiunea lipsește
        let v = build(vec![f]);
        assert!(
            v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
                .get("region")
                .is_none()
        );
    }

    #[test]
    fn t5_message_roundtrip() {
        let mut f = base_finding();
        f.message = "ghilimele \" backslash \\ linie\nnouă\t\u{1}".into();
        let text = serde_json::to_string_pretty(&build(vec![f.clone()])).unwrap();
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back["runs"][0]["results"][0]["message"]["text"], f.message);
    }

    #[test]
    fn t6_levels_in_allowed_set_for_all_rules() {
        for (_, lvl, _) in RULES {
            assert!(LEVELS.contains(&lvl));
        }
    }

    fn session() -> Session {
        let mut s = Session {
            version: 1,
            tool: "veriga".into(),
            started: "a".into(),
            finished: "b".into(),
            config: crate::Config::default(),
            results: vec![],
            ignored: 0,
            truncated: false,
            cancelled: false,
            aliases: vec![],
            a11y: vec![],
            warnings: vec![],
            seo: vec![],
        };
        let mk = |u: &str, st: Status, parent: &str| {
            let mut r = LinkResult::new(u, LinkKind::Page, 1, true);
            r.status = st;
            r.parents.push(Parent {
                url: parent.into(),
                text: String::new(),
                kind: LinkKind::Page,
            });
            r
        };
        s.results
            .push(mk("http://x.org/ok", Status::Ok, "http://x.org/"));
        s.results
            .push(mk("http://x.org/z", Status::NotFound, "http://x.org/b ă"));
        s.results
            .push(mk("http://x.org/m", Status::Redirected, "http://x.org/a"));
        s.results
            .push(mk("http://x.org/f", Status::Forbidden, "http://x.org/a"));
        s.results
            .push(mk("http://x.org/s", Status::Skipped, "http://x.org/a"));
        s
    }

    #[test]
    fn levels_mapping_and_good_links_omitted() {
        let v = build(findings(&session()));
        assert_structurally_valid(&v);
        let rs = v["runs"][0]["results"].as_array().unwrap();
        assert_eq!(rs.len(), 4);
        let lvl = |rule: &str| {
            rs.iter().find(|r| r["ruleId"] == rule).unwrap()["level"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(lvl("veriga/broken-link"), "error");
        assert_eq!(lvl("veriga/redirect"), "warning");
        assert_eq!(lvl("veriga/needs-review"), "warning");
        assert_eq!(lvl("veriga/skipped"), "note");
        // non-ASCII și spații în sursă: uri procent-codat
        let uri = rs
            .iter()
            .find(|r| r["ruleId"] == "veriga/broken-link")
            .unwrap()["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(uri.is_ascii() && !uri.contains(' '), "{uri}");
    }

    #[test]
    fn t7_deterministic_bytes_and_sorted() {
        let a = to_sarif(&session());
        let b = to_sarif(&session());
        assert_eq!(a, b);
        let v: Value = serde_json::from_str(&a).unwrap();
        let uris: Vec<String> = v["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        let mut sorted = uris.clone();
        sorted.sort();
        assert_eq!(uris, sorted);
    }

    #[test]
    fn no_parent_uses_own_address() {
        let mut s = session();
        s.results.clear();
        let mut r = LinkResult::new("http://x.org/", LinkKind::Start, 0, true);
        r.status = Status::NoConnection;
        s.results.push(r);
        let v = build(findings(&s));
        assert_eq!(
            v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "http://x.org/"
        );
    }
}
