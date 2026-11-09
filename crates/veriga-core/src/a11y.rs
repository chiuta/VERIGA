//! Controlul numelui accesibil al legăturilor (`<a href>`), după regulile de calcul ale
//! numelui din Accname 1.2, în varianta simplificată descrisă în specificație.

use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum A11yKind {
    /// Legătură fără nume accesibil (WCAG 2.4.4, 4.1.2).
    NoName,
    /// Nume doar din semne de punctuație sau simboluri.
    Meaningless,
    /// Același nume pentru adrese diferite (informare; contextul poate lămuri).
    Ambiguous,
}

impl A11yKind {
    pub fn label(self, lang: &str) -> &'static str {
        match (self, lang == "en") {
            (Self::NoName, false) => "legătură fără nume",
            (Self::NoName, true) => "link without a name",
            (Self::Meaningless, false) => "nume nesemnificativ",
            (Self::Meaningless, true) => "meaningless name",
            (Self::Ambiguous, false) => "nume ambiguu",
            (Self::Ambiguous, true) => "ambiguous name",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct A11yFinding {
    pub page: String,
    pub kind: A11yKind,
    pub name: String,
    pub targets: Vec<String>,
}

/// Numele calculat al unei legături și adresa ei țintă (rezolvată).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkName {
    pub name: String,
    pub target: String,
}

fn norm_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn attr<'a>(el: &'a ElementRef, name: &str) -> Option<&'a str> {
    el.value().attr(name)
}

fn is_hidden(el: &ElementRef) -> bool {
    attr(el, "hidden").is_some()
        || attr(el, "aria-hidden").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

fn is_inline(name: &str) -> bool {
    matches!(
        name,
        "a" | "span"
            | "b"
            | "i"
            | "em"
            | "strong"
            | "small"
            | "abbr"
            | "code"
            | "sub"
            | "sup"
            | "mark"
            | "u"
            | "s"
            | "cite"
            | "q"
            | "bdi"
            | "bdo"
            | "time"
            | "font"
            | "label"
    )
}

/// Contribuția conținutului unui element (descendenți), fără numele propriu al elementului.
fn content_of(el: &ElementRef, out: &mut String) {
    for child in el.children() {
        match child.value() {
            Node::Text(t) => out.push_str(t),
            Node::Element(_) => {
                let Some(c) = ElementRef::wrap(child) else {
                    continue;
                };
                if is_hidden(&c) || matches!(c.value().name(), "script" | "style" | "template") {
                    continue;
                }
                let name = c.value().name();
                let piece = match name {
                    "img" => match attr(&c, "alt") {
                        Some(a) => a.to_string(), // alt="" = decorativă, fără contribuție
                        None => attr(&c, "title").unwrap_or("").to_string(),
                    },
                    "input"
                        if attr(&c, "type").is_some_and(|t| t.eq_ignore_ascii_case("image")) =>
                    {
                        attr(&c, "alt").unwrap_or("").to_string()
                    }
                    _ => match attr(&c, "aria-label")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        Some(l) => l.to_string(),
                        None => {
                            let mut s = String::new();
                            content_of(&c, &mut s);
                            s
                        }
                    },
                };
                if is_inline(name) {
                    out.push_str(&piece);
                } else {
                    out.push(' ');
                    out.push_str(&piece);
                    out.push(' ');
                }
            }
            _ => {}
        }
    }
}

fn find_by_id<'a>(doc: &'a Html, id: &str) -> Option<ElementRef<'a>> {
    let all = Selector::parse("[id]").ok()?;
    doc.select(&all).find(|e| e.value().attr("id") == Some(id))
}

/// Numele accesibil al unui element `a` (Accname: labelledby, aria-label, conținut, title).
pub fn accessible_name(doc: &Html, a: &ElementRef) -> String {
    // 1. aria-labelledby (referințe unice; id inexistent → se trece mai departe)
    if let Some(ids) = attr(a, "aria-labelledby") {
        let mut used: HashSet<&str> = HashSet::new();
        let mut parts: Vec<String> = vec![];
        for id in ids.split_whitespace() {
            if !used.insert(id) {
                continue;
            }
            if let Some(r) = find_by_id(doc, id) {
                let t = match attr(&r, "aria-label")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    Some(l) => l.to_string(),
                    None => {
                        let mut s = String::new();
                        content_of(&r, &mut s);
                        s
                    }
                };
                parts.push(t);
            }
        }
        let n = norm_ws(&parts.join(" "));
        if !n.is_empty() {
            return n;
        }
    }
    // 2. aria-label nevid
    if let Some(l) = attr(a, "aria-label") {
        let n = norm_ws(l);
        if !n.is_empty() {
            return n;
        }
    }
    // 3. conținut
    let mut s = String::new();
    content_of(a, &mut s);
    let n = norm_ws(&s);
    if !n.is_empty() {
        return n;
    }
    // 4. title, ultimul recurs
    attr(a, "title").map(norm_ws).unwrap_or_default()
}

/// Numele tuturor legăturilor (`a` cu `href`) dintr-o pagină.
pub fn link_names(html: &str, page: &Url) -> Vec<LinkName> {
    let doc = Html::parse_document(html);
    let mut base = page.clone();
    if let Ok(sel) = Selector::parse("base[href]")
        && let Some(b) = doc.select(&sel).next()
        && let Some(h) = b.value().attr("href")
        && let Ok(u) = page.join(h.trim())
    {
        base = u;
    }
    let sel = Selector::parse("a[href]").unwrap();
    doc.select(&sel)
        .map(|a| {
            let href = a.value().attr("href").unwrap_or("").trim();
            let target = base
                .join(href)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| href.to_string());
            LinkName {
                name: accessible_name(&doc, &a),
                target,
            }
        })
        .collect()
}

/// Constatările unei pagini. `ambiguity` activează și notele de ambiguitate.
pub fn findings(page: &str, links: &[LinkName], ambiguity: bool) -> Vec<A11yFinding> {
    let mut out = vec![];
    let mut seen: BTreeSet<(A11yKind, String, String)> = BTreeSet::new();
    for l in links {
        let kind = if l.name.is_empty() {
            A11yKind::NoName
        } else if !l.name.chars().any(char::is_alphanumeric) {
            A11yKind::Meaningless
        } else {
            continue;
        };
        if seen.insert((kind, l.name.clone(), l.target.clone())) {
            out.push(A11yFinding {
                page: page.to_string(),
                kind,
                name: l.name.clone(),
                targets: vec![l.target.clone()],
            });
        }
    }
    if ambiguity {
        let mut by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut shown: BTreeMap<String, String> = BTreeMap::new();
        for l in links
            .iter()
            .filter(|l| l.name.chars().any(char::is_alphanumeric))
        {
            let k = l.name.to_lowercase();
            shown.entry(k.clone()).or_insert_with(|| l.name.clone());
            by_name.entry(k).or_default().insert(l.target.clone());
        }
        for (k, targets) in by_name {
            if targets.len() > 1 {
                out.push(A11yFinding {
                    page: page.to_string(),
                    kind: A11yKind::Ambiguous,
                    name: shown.remove(&k).unwrap_or(k),
                    targets: targets.into_iter().collect(),
                });
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(html: &str) -> Vec<String> {
        let p = Url::parse("http://x.org/").unwrap();
        link_names(html, &p).into_iter().map(|l| l.name).collect()
    }

    #[test]
    fn t1_text() {
        assert_eq!(names(r#"<a href="/p">Prețuri</a>"#), vec!["Prețuri"]);
    }

    #[test]
    fn t2_img_empty_alt_is_nameless() {
        assert_eq!(
            names(r#"<a href="/p"><img src="i.png" alt=""></a>"#),
            vec![""]
        );
        let p = Url::parse("http://x.org/").unwrap();
        let f = findings(
            "p",
            &link_names(r#"<a href="/p"><img alt=""></a>"#, &p),
            false,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, A11yKind::NoName);
    }

    #[test]
    fn t3_img_alt() {
        assert_eq!(
            names(r#"<a href="/p"><img src="i.png" alt="Acasă"></a>"#),
            vec!["Acasă"]
        );
    }

    #[test]
    fn img_without_alt_uses_title() {
        assert_eq!(
            names(r#"<a href="/p"><img src="i" title="Titlu"></a>"#),
            vec!["Titlu"]
        );
    }

    #[test]
    fn t4_aria_label_wins_over_text() {
        assert_eq!(
            names(r#"<a href="/p" aria-label="Contact">→</a>"#),
            vec!["Contact"]
        );
    }

    #[test]
    fn t5_hidden_content_then_title() {
        assert_eq!(
            names(r#"<a href="/p" title="Ajutor"><span aria-hidden="true">?</span></a>"#),
            vec!["Ajutor"]
        );
        assert_eq!(
            names(r#"<a href="/p"><span hidden>x</span>Vizibil</a>"#),
            vec!["Vizibil"]
        );
    }

    #[test]
    fn t6_punctuation_only_is_meaningless() {
        let p = Url::parse("http://x.org/").unwrap();
        let l = link_names(r#"<a href="/p">  ...  </a>"#, &p);
        assert_eq!(l[0].name, "...");
        let f = findings("p", &l, false);
        assert_eq!(f[0].kind, A11yKind::Meaningless);
    }

    #[test]
    fn t7_ambiguity() {
        let p = Url::parse("http://x.org/").unwrap();
        let l = link_names(r#"<a href="/x">Detalii</a><a href="/y">detalii</a>"#, &p);
        let f = findings("p", &l, true);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, A11yKind::Ambiguous);
        assert_eq!(f[0].targets.len(), 2);
        let same = link_names(r#"<a href="/x">Detalii</a><a href="/x">Detalii</a>"#, &p);
        assert!(findings("p", &same, true).is_empty());
        // fără opțiunea de ambiguitate nu se emite nimic
        assert!(findings("p", &l, false).is_empty());
    }

    #[test]
    fn labelledby_refs_and_fallbacks() {
        assert_eq!(
            names(
                r#"<span id="a">Ana</span><span id="b">Maria</span><a href="/p" aria-labelledby="a b a">x</a>"#
            ),
            vec!["Ana Maria"]
        );
        // id inexistent → pasul următor
        assert_eq!(
            names(r#"<a href="/p" aria-labelledby="nu" aria-label="Eticheta">x</a>"#),
            vec!["Eticheta"]
        );
        // referință circulară către propriul element: nu se blochează
        assert_eq!(
            names(r#"<a id="s" href="/p" aria-labelledby="s">Text</a>"#),
            vec!["Text"]
        );
    }

    #[test]
    fn whitespace_normalised_and_anchor_without_href_skipped() {
        assert_eq!(
            names("<a href=\"/p\">  Un \n\t text   lung </a><a name=\"z\">fara href</a>"),
            vec!["Un text lung"]
        );
        assert_eq!(names(r#"<a href="/p"><b>Pre</b>țuri</a>"#), vec!["Prețuri"]);
    }
}
