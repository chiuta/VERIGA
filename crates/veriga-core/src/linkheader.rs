//! Citirea câmpurilor HTTP `Link` (RFC 8288): legături declarate în antetul răspunsului.

use crate::extract::parse_rel;
use url::Url;

/// O legătură din antet, cu ținta rezolvată față de adresa cererii.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderLink {
    pub target: Url,
    /// Tipurile de relație (din prima apariție a lui `rel`); `["unknown"]` dacă lipsește.
    pub rel: Vec<String>,
    /// Toți parametrii, în ordinea apariției (nume în minuscule).
    pub params: Vec<(String, String)>,
    /// Contextul `anchor`, rezolvat, dacă există.
    pub anchor: Option<Url>,
    /// `title*` (prioritar) sau `title`.
    pub title: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub links: Vec<HeaderLink>,
    pub warnings: Vec<String>,
}

fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Legătura din antet, ca legătură de verificat (originea apare în textul ei).
pub fn to_raw_link(h: HeaderLink) -> crate::extract::RawLink {
    use crate::model::LinkKind;
    let has = |t: &str| h.rel.iter().any(|r| r == t);
    let as_kind = h
        .params
        .iter()
        .find(|(n, _)| n == "as")
        .map(|(_, v)| v.to_ascii_lowercase())
        .unwrap_or_default();
    let kind = if has("stylesheet") {
        LinkKind::Stylesheet
    } else if has("icon") {
        LinkKind::Icon
    } else if has("preload") || has("prefetch") {
        match as_kind.as_str() {
            "style" => LinkKind::Stylesheet,
            "script" => LinkKind::Script,
            "image" => LinkKind::Image,
            _ => LinkKind::Link,
        }
    } else {
        LinkKind::Link
    };
    let label = h.title.clone().unwrap_or_else(|| h.rel.join(" "));
    crate::extract::RawLink {
        written: h.target.to_string(),
        url: h.target,
        kind,
        text: crate::urlutil::truncate_chars(&format!("Link header: {label}"), 120),
        rel: h.rel,
    }
}

/// Poziția primei virgule din afara ghilimelelor, începând de la `from` (sau sfârșitul).
fn next_comma(s: &[char], from: usize) -> usize {
    let (mut i, mut inq) = (from, false);
    while i < s.len() {
        match s[i] {
            '\\' if inq => i += 1,
            '"' => inq = !inq,
            ',' if !inq => return i,
            _ => {}
        }
        i += 1;
    }
    s.len()
}

/// Decodează valoarea extinsă a lui `title*` (RFC 8187): `charset'limbă'procent-codat`.
fn decode_ext(v: &str) -> Option<String> {
    let mut it = v.splitn(3, '\'');
    let cs = it.next()?;
    let _lang = it.next()?;
    let enc = it.next()?;
    let bytes: Vec<u8> = percent_encoding::percent_decode_str(enc).collect();
    match cs.to_ascii_lowercase().as_str() {
        "utf-8" => String::from_utf8(bytes).ok(),
        "iso-8859-1" => Some(bytes.iter().map(|&b| b as char).collect()),
        _ => None,
    }
}

/// Parsează valorile câmpurilor `Link` (în ordinea primirii), ca o singură listă.
pub fn parse(values: &[String], request: &Url, cap: usize) -> Parsed {
    let joined = values.join(",");
    let s: Vec<char> = joined.chars().collect();
    let mut out = Parsed::default();
    let mut i = 0;
    while i < s.len() {
        while i < s.len() && (is_ows(s[i]) || s[i] == ',') {
            i += 1;
        }
        if i >= s.len() {
            break;
        }
        if s[i] != '<' {
            out.warnings.push("valoare Link fără «<»; ignorată".into());
            i = next_comma(&s, i);
            continue;
        }
        let Some(rel_end) = s[i + 1..].iter().position(|&c| c == '>') else {
            out.warnings.push("valoare Link fără «>»; ignorată".into());
            i = next_comma(&s, i);
            continue;
        };
        let end = i + 1 + rel_end;
        let reference: String = s[i + 1..end].iter().collect();
        i = end + 1;
        // parametri
        let mut params: Vec<(String, String)> = vec![];
        loop {
            while i < s.len() && is_ows(s[i]) {
                i += 1;
            }
            if i >= s.len() || s[i] == ',' {
                break;
            }
            if s[i] != ';' {
                // zgomot între parametri: se sare la următorul `;` sau `,`
                while i < s.len() && s[i] != ';' && s[i] != ',' {
                    i += 1;
                }
                continue;
            }
            i += 1;
            while i < s.len() && is_ows(s[i]) {
                i += 1;
            }
            let ns = i;
            while i < s.len() && !matches!(s[i], '=' | ';' | ',') && !is_ows(s[i]) {
                i += 1;
            }
            let name: String = s[ns..i].iter().collect::<String>().to_ascii_lowercase();
            while i < s.len() && is_ows(s[i]) {
                i += 1;
            }
            let mut val = String::new();
            if i < s.len() && s[i] == '=' {
                i += 1;
                while i < s.len() && is_ows(s[i]) {
                    i += 1;
                }
                if i < s.len() && s[i] == '"' {
                    i += 1;
                    let mut closed = false;
                    while i < s.len() {
                        match s[i] {
                            '\\' if i + 1 < s.len() => {
                                val.push(s[i + 1]);
                                i += 2;
                            }
                            '"' => {
                                closed = true;
                                i += 1;
                                break;
                            }
                            c => {
                                val.push(c);
                                i += 1;
                            }
                        }
                    }
                    if !closed {
                        out.warnings
                            .push("șir între ghilimele neînchis în antetul Link".into());
                    }
                } else {
                    let vs = i;
                    while i < s.len() && s[i] != ';' && s[i] != ',' {
                        i += 1;
                    }
                    val = s[vs..i].iter().collect::<String>().trim().to_string();
                }
            }
            if !name.is_empty() {
                params.push((name, val));
            }
        }
        if out.links.len() >= cap {
            out.warnings.push(format!(
                "peste {cap} legături în antetul Link; restul ignorate"
            ));
            break;
        }
        let Ok(target) = request.join(reference.trim()) else {
            out.warnings
                .push(format!("referință Link nevalidă: {reference}"));
            continue;
        };
        let rel = params
            .iter()
            .find(|(n, _)| n == "rel")
            .map(|(_, v)| parse_rel(v))
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| vec!["unknown".into()]);
        let title = params
            .iter()
            .find(|(n, _)| n == "title*")
            .and_then(|(_, v)| decode_ext(v))
            .or_else(|| {
                params
                    .iter()
                    .find(|(n, _)| n == "title")
                    .map(|(_, v)| v.clone())
            });
        let anchor = params
            .iter()
            .find(|(n, _)| n == "anchor")
            .and_then(|(_, v)| request.join(v).ok());
        out.links.push(HeaderLink {
            target,
            rel,
            params,
            anchor,
            title,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(vals: &[&str], req: &str) -> Parsed {
        let v: Vec<String> = vals.iter().map(|s| s.to_string()).collect();
        parse(&v, &Url::parse(req).unwrap(), 100)
    }

    #[test]
    fn t1_preload() {
        let r = p(
            &["<https://ex.org/a.css>; rel=preload; as=style"],
            "https://ex.org/",
        );
        assert_eq!(r.links.len(), 1);
        assert_eq!(r.links[0].rel, vec!["preload"]);
        assert!(r.links[0].params.contains(&("as".into(), "style".into())));
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn t2_next_prev_relative() {
        let r = p(
            &["</n>; rel=\"next\", </p>; rel=\"prev\""],
            "https://ex.org/d/pag",
        );
        let t: Vec<_> = r.links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(t, vec!["https://ex.org/n", "https://ex.org/p"]);
    }

    #[test]
    fn t3_quoted_title_with_separators() {
        let r = p(
            &["<https://ex.org/x>; rel=\"alternate stylesheet\"; title=\"a, b; c\""],
            "https://ex.org/",
        );
        assert_eq!(r.links.len(), 1);
        assert_eq!(r.links[0].rel, vec!["alternate", "stylesheet"]);
        assert_eq!(r.links[0].title.as_deref(), Some("a, b; c"));
    }

    #[test]
    fn t4_two_fields_in_order() {
        let r = p(
            &["<https://ex.org/1>; rel=a", "<https://ex.org/2>; rel=b"],
            "https://ex.org/",
        );
        assert_eq!(r.links.len(), 2);
        assert_eq!(r.links[0].target.path(), "/1");
        assert_eq!(r.links[1].target.path(), "/2");
    }

    #[test]
    fn t5_missing_gt_is_warning() {
        let r = p(&["<https://ex.org/y; rel=next"], "https://ex.org/");
        assert_eq!((r.links.len(), r.warnings.len()), (0, 1));
    }

    #[test]
    fn t5b_recovers_at_next_comma() {
        let r = p(
            &["<https://ex.org/y; rel=next, <https://ex.org/ok>; rel=prev"],
            "https://ex.org/",
        );
        // prima valoare își găsește un «>» la a doua; rezultatul rămâne determinist și fără panică
        assert!(r.links.len() <= 2);
        let r = p(&["junk, <https://ex.org/ok>; rel=prev"], "https://ex.org/");
        assert_eq!(r.links.len(), 1);
        assert_eq!(r.warnings.len(), 1);
    }

    #[test]
    fn t6_first_rel_wins() {
        let r = p(&["<https://ex.org/z>; rel=a; rel=b"], "https://ex.org/");
        assert_eq!(r.links[0].rel, vec!["a"]);
    }

    #[test]
    fn t7_relative_reference() {
        let r = p(&["<../s.png>; rel=preload"], "https://ex.org/a/b/c");
        assert_eq!(r.links[0].target.as_str(), "https://ex.org/a/s.png");
    }

    #[test]
    fn no_rel_is_unknown_and_kept() {
        let r = p(&["</x>"], "https://ex.org/");
        assert_eq!(r.links[0].rel, vec!["unknown"]);
    }

    #[test]
    fn empty_reference_is_request_url() {
        let r = p(&["<>; rel=self"], "https://ex.org/q");
        assert_eq!(r.links[0].target.as_str(), "https://ex.org/q");
    }

    #[test]
    fn title_star_has_priority() {
        let r = p(
            &["</x>; rel=a; title=\"plain\"; title*=UTF-8''%C8%98coal%C4%83"],
            "https://ex.org/",
        );
        assert_eq!(r.links[0].title.as_deref(), Some("Școală"));
    }

    #[test]
    fn anchor_is_resolved() {
        let r = p(&["</x>; rel=a; anchor=\"/ctx\""], "https://ex.org/d/");
        assert_eq!(
            r.links[0].anchor.as_ref().unwrap().as_str(),
            "https://ex.org/ctx"
        );
    }

    #[test]
    fn cap_limits_links_with_warning() {
        let v: Vec<String> = (0..5).map(|i| format!("</{i}>; rel=a")).collect();
        let r = parse(&v, &Url::parse("https://ex.org/").unwrap(), 3);
        assert_eq!(r.links.len(), 3);
        assert_eq!(r.warnings.len(), 1);
    }

    #[test]
    fn duplicate_other_params_are_all_kept() {
        let r = p(
            &["</x>; rel=a; hreflang=ro; hreflang=en"],
            "https://ex.org/",
        );
        assert_eq!(
            r.links[0]
                .params
                .iter()
                .filter(|(n, _)| n == "hreflang")
                .count(),
            2
        );
    }
}
