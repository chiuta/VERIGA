//! Plic FFF (`.fff.json`), pasul VERIGA din lanțul LegatorOCR → TEZAUR → VERIGA → AcademiSearch.
//! Intră `citation/csl` (de ex. `tezaur bib --fff-out`), iese `citation/csl` cu fiecare referință adnotată în
//! `custom.veriga`: ce s-a verificat (sintaxa DOI, legătura URL/DOI) și cu ce rezultat. Fără rețea nu se inventează
//! verdicte: legăturile rămân „unchecked”, iar o referință fără URL/DOI e marcată „no-link” (nu e greșeală).
//! Specificația plicului: docs/FFF-ENVELOPE.md din hubul chiuta.github.io.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use veriga_core::Session;

pub struct Input {
    pub items: Vec<Value>,
    pub app: String,
    pub title: String,
}

pub fn read(path: &Path) -> Result<Input> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&raw)
}

pub fn parse(raw: &str) -> Result<Input> {
    let v: Value = serde_json::from_str(raw).context("not valid JSON")?;
    let payload = match v.get("fff").and_then(|x| x.as_i64()) {
        Some(2) => v.get("payload"),
        Some(1) => v.get("data"),
        _ => None,
    }
    .context("not an FFF envelope")?;
    let typ = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
    if typ != "citation/csl" {
        bail!("envelope type {typ} is not accepted (citation/csl)");
    }
    let items = payload
        .as_array()
        .context("citation/csl payload must be a list")?
        .clone();
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok(Input {
        items,
        app: s("app"),
        title: s("title"),
    })
}

/// `10.xxxx/sufix` — doar sintaxa (prefix de registru, 4–9 cifre, `/`, sufix fără spații).
pub fn doi_ok(d: &str) -> bool {
    let d = d.trim();
    let Some((reg, suf)) = d.split_once('/') else {
        return false;
    };
    let Some(num) = reg.strip_prefix("10.") else {
        return false;
    };
    (4..=9).contains(&num.len())
        && num.chars().all(|c| c.is_ascii_digit())
        && !suf.is_empty()
        && !suf.chars().any(char::is_whitespace)
}

/// Legătura indică o gazdă locală/privată (literal IP sau nume local)? Pentru astfel de legături dintr-un plic
/// nu se face nicio cerere în mod implicit: plicul vine dintr-un lanț care procesează documente
/// necontrolate, iar un URL privat din el ar fi tratat ca punct de start (adică exceptat de anti-SSRF).
pub fn is_private_link(link: &str) -> bool {
    url::Url::parse(link)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(veriga_core::netguard::host_is_private_syntactic)
        })
        .unwrap_or(false)
}

/// Legătura de verificat pentru o referință: URL http(s) sau https://doi.org/<DOI> (doar dacă DOI-ul are sintaxă validă).
pub fn link_of(item: &Value) -> Option<String> {
    if let Some(u) = item.get("URL").and_then(|x| x.as_str()) {
        let u = u.trim();
        if u.starts_with("http://") || u.starts_with("https://") {
            return Some(u.to_string());
        }
    }
    let d = item.get("DOI").and_then(|x| x.as_str())?;
    doi_ok(d).then(|| format!("https://doi.org/{}", d.trim()))
}

fn norm(u: &str) -> String {
    u.trim().trim_end_matches('/').to_lowercase()
}

/// Adnotează referințele. `session` = rezultatul verificării (None: nu s-a verificat nimic: fără rețea / `--no-external`).
pub fn annotate(items: &[Value], session: Option<&Session>, now: &str) -> Vec<Value> {
    let by: HashMap<String, &veriga_core::model::LinkResult> = session
        .map(|s| s.results.iter().map(|r| (norm(&r.url), r)).collect())
        .unwrap_or_default();
    items
        .iter()
        .map(|it| {
            let mut it = it.clone();
            // un element care nu e obiect (șir, număr, null) rămâne neatins: nu are unde primi `custom`
            if !it.is_object() {
                return it;
            }
            let doi = it.get("DOI").and_then(|x| x.as_str()).map(str::to_string);
            let mut v = serde_json::Map::new();
            if let Some(d) = &doi {
                v.insert(
                    "doi_syntax".into(),
                    json!(if doi_ok(d) { "ok" } else { "bad" }),
                );
            }
            match link_of(&it) {
                None => {
                    v.insert(
                        "status".into(),
                        json!(if doi.is_some() { "bad-doi" } else { "no-link" }),
                    );
                    v.insert("checked".into(), json!(false));
                }
                Some(link) => {
                    v.insert("link".into(), json!(link));
                    match by.get(&norm(&link)) {
                        Some(r) => {
                            use veriga_core::model::Status as S;
                            // lipsa rețelei (DNS, conexiune, timp, TLS) nu e „link rupt”: nu s-a putut verifica
                            let st = match r.status {
                                S::Ok => "ok",
                                S::Redirected => "redirected",
                                S::NoConnection
                                | S::DnsFailure
                                | S::Timeout
                                | S::TlsError
                                | S::RobotsBlocked => "unreachable",
                                x if x.is_error() => "broken",
                                _ => "unchecked",
                            };
                            v.insert("status".into(), json!(st));
                            v.insert(
                                "checked".into(),
                                json!(st != "unchecked" && st != "unreachable"),
                            );
                            if let Some(c) = r.http_status {
                                v.insert("http".into(), json!(c));
                            }
                            if let Some(f) = &r.final_url {
                                v.insert("final_url".into(), json!(f));
                            }
                        }
                        None => {
                            v.insert("status".into(), json!("unchecked"));
                            v.insert("checked".into(), json!(false));
                        }
                    }
                }
            }
            v.insert("date".into(), json!(now));
            if let Some(obj) = it.as_object_mut() {
                let c = obj.entry("custom").or_insert_with(|| json!({}));
                if let Some(co) = c.as_object_mut() {
                    co.insert("veriga".into(), Value::Object(v));
                }
            }
            it
        })
        .collect()
}

/// Plicul de ieșire (`fff:2`, `citation/csl`).
pub fn envelope(input: &Input, items: Vec<Value>, now: &str) -> String {
    let n = items.len();
    let broken = items
        .iter()
        .filter(|i| i["custom"]["veriga"]["status"] == "broken")
        .count();
    let checked = items
        .iter()
        .filter(|i| i["custom"]["veriga"]["checked"] == true)
        .count();
    let env = json!({
        "fff": 2,
        "type": "citation/csl",
        "title": if input.title.is_empty() { "VERIGA".to_string() } else { format!("VERIGA · {}", input.title) },
        "payload": items,
        "source": {"label": format!("VERIGA · {n} referințe, {checked} legături verificate, {broken} rupte"), "via": input.app},
        "created": now,
        "app": "VERIGA",
    });
    serde_json::to_string_pretty(&env).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doi_syntax() {
        assert!(doi_ok("10.1000/xyz123"));
        assert!(
            !doi_ok("11.1000/x")
                && !doi_ok("10.1/x")
                && !doi_ok("10.1000/")
                && !doi_ok("10.1000/a b")
        );
    }

    #[test]
    fn scan_without_links_is_marked_no_link_and_kept() {
        let inp = parse(r#"{"fff":2,"type":"citation/csl","title":"TEZAUR","payload":[{"id":"k","type":"document","title":"Raport"}],"created":"2026-11-03T00:00:00Z","app":"TEZAUR"}"#).unwrap();
        assert!(link_of(&inp.items[0]).is_none());
        let a = annotate(&inp.items, None, "2026-11-03T00:00:00.000Z");
        assert_eq!(a[0]["title"], "Raport");
        assert_eq!(a[0]["custom"]["veriga"]["status"], "no-link");
        assert_eq!(a[0]["custom"]["veriga"]["checked"], false);
        assert!(envelope(&inp, a, "2026-11-03T00:00:00.000Z").contains("\"app\": \"VERIGA\""));
    }

    #[test]
    fn doi_and_url_become_links_and_bad_doi_is_flagged() {
        let inp = parse(r#"{"fff":1,"type":"citation/csl","data":[{"title":"a","DOI":"10.1000/x"},{"title":"b","URL":"https://e.org/p"},{"title":"c","DOI":"zzz"}],"src":"t"}"#).unwrap();
        assert_eq!(
            link_of(&inp.items[0]).as_deref(),
            Some("https://doi.org/10.1000/x")
        );
        assert_eq!(link_of(&inp.items[1]).as_deref(), Some("https://e.org/p"));
        let a = annotate(&inp.items, None, "t");
        assert_eq!(a[0]["custom"]["veriga"]["status"], "unchecked");
        assert_eq!(a[2]["custom"]["veriga"]["status"], "bad-doi");
        assert!(
            parse(r#"{"fff":2,"type":"table/csv","payload":[],"created":"x","app":"y"}"#).is_err()
        );
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    #[test]
    fn non_object_items_do_not_panic_and_are_kept_unchanged() {
        let inp = parse(
            r#"{"fff":2,"type":"citation/csl","payload":["text",5,null,[1],{"URL":"https://e.org/p"},{"URL":"https://e.org/q","custom":"x"}],"created":"t","app":"a"}"#,
        )
        .unwrap();
        let out = annotate(&inp.items, None, "t");
        assert_eq!(out.len(), 6);
        assert_eq!(out[0], "text");
        assert_eq!(out[1], 5);
        assert!(out[2].is_null());
        assert_eq!(out[3], json!([1]));
        assert_eq!(out[4]["custom"]["veriga"]["status"], "unchecked");
        // `custom` care nu e obiect rămâne cum e (nu se suprascrie datele altcuiva)
        assert_eq!(out[5]["custom"], "x");
        let env = envelope(&inp, out, "t");
        assert!(env.contains("\"app\": \"VERIGA\""));
    }

    #[test]
    fn private_and_local_envelope_links_are_recognised() {
        for u in [
            "http://127.0.0.1:8080/x",
            "http://localhost/x",
            "http://[::1]/x",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://2130706433/",
            "http://0x7f000001/",
        ] {
            assert!(is_private_link(u), "{u}");
        }
        for u in [
            "https://example.org/p",
            "https://doi.org/10.1000/x",
            "http://8.8.8.8/",
        ] {
            assert!(!is_private_link(u), "{u}");
        }
        assert!(!is_private_link("nu e url"));
    }
}
