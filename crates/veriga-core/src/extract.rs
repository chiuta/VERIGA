//! Extragerea linkurilor din HTML și CSS.

use crate::model::LinkKind;
use crate::urlutil::truncate_chars;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashSet;
use std::sync::OnceLock;
use url::Url;

/// Rolul unei legături ca subresursă încărcată de pagină (pentru conținutul mixt).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sub {
    /// Nu este subresursă (navigare, referință informativă).
    #[default]
    None,
    /// Imagini, audio, video: «opțional blocabilă».
    Optional,
    /// Orice altă subresursă: «blocabilă».
    Blockable,
}

fn is_font_path(p: &str) -> bool {
    let p = p.to_ascii_lowercase();
    [".woff2", ".woff", ".ttf", ".otf", ".eot"]
        .iter()
        .any(|e| p.ends_with(e))
}

/// Rolul implicit după tipul legăturii.
pub fn default_sub(kind: LinkKind, u: &Url) -> Sub {
    match kind {
        LinkKind::Image if is_font_path(u.path()) => Sub::Blockable,
        LinkKind::Image | LinkKind::Media => Sub::Optional,
        LinkKind::Stylesheet
        | LinkKind::Script
        | LinkKind::Frame
        | LinkKind::Object
        | LinkKind::Icon
        | LinkKind::Form => Sub::Blockable,
        _ => Sub::None,
    }
}

#[derive(Clone, Debug)]
pub struct RawLink {
    pub url: Url,
    pub kind: LinkKind,
    pub text: String,
    /// Adresa absolută așa cum a fost scrisă de autor (pentru detectarea alias-urilor).
    pub written: String,
    /// Tipurile de relație din atributul `rel` (minuscule, unice, în ordinea apariției).
    pub rel: Vec<String>,
    pub sub: Sub,
    /// Valoarea atributului `integrity` (script, link stylesheet/preload/modulepreload).
    pub integrity: Option<String>,
    /// Atributul `crossorigin`, dacă există (valoarea în minuscule).
    pub crossorigin: Option<String>,
}

#[derive(Debug, Default)]
pub struct Extracted {
    pub title: Option<String>,
    pub h1: Option<String>,
    pub links: Vec<RawLink>,
    /// (text brut, tip, descriere) pentru valori care nu pot fi interpretate ca URL.
    pub bad: Vec<(String, LinkKind, String)>,
    pub anchors: HashSet<String>,
    pub meta_robots: Option<String>,
    /// Adresele `link rel=canonical` (rezolvate față de baza documentului).
    pub canonicals: Vec<Url>,
    /// Valorile `hreflang` de pe `link rel=alternate` și de pe `a`.
    pub hreflangs: Vec<String>,
    /// Numele elementelor cu `integrity` la care atributul nu se aplică.
    pub sri_notes: Vec<String>,
    /// Pagina a depășit bugetul de parsare ([`MAX_PARSE_EVENTS`]): s-a analizat doar începutul ei.
    pub parse_truncated: bool,
}

/// Bugetul de «evenimente de parsare» (octeți NUL și semne `<`) pe pagină. Parserul HTML păstrează în
/// memorie fiecare eroare și fiecare nod: o pagină de 7 MB doar din NUL sau din `<` ocupa 560–710 MB
/// (amplificare ≈100×), iar cu multe fire în paralel un site ostil putea epuiza memoria. 200.000 de
/// evenimente înseamnă aproximativ 100.000 de elemente, cu mult peste o pagină reală.
pub const MAX_PARSE_EVENTS: usize = 200_000;

/// Taie `html` la depășirea bugetului de parsare; întoarce textul de parsat și dacă a fost tăiat.
pub fn cap_parse_events(html: &str) -> (&str, bool) {
    let mut n = 0usize;
    for (i, b) in html.bytes().enumerate() {
        if b == b'<' || b == 0 {
            n += 1;
            if n > MAX_PARSE_EVENTS {
                // `<` și NUL sunt ASCII: `i` este mereu la limită de caracter
                return (&html[..i], true);
            }
        }
    }
    (html, false)
}

fn css_url_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?i)url\(\s*(?:"([^"]*)"|'([^']*)'|([^)\s'"]+))\s*\)"#).unwrap())
}

fn css_import_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?i)@import\s+(?:"([^"]+)"|'([^']+)')"#).unwrap())
}

fn refresh_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?i)url\s*=\s*['"]?([^'";]+)"#).unwrap())
}

/// `integrity` și `crossorigin` ale unui element (dacă `integrity` nu e gol).
fn sri_of(v: &scraper::node::Element) -> Option<(String, Option<String>)> {
    let i = v.attr("integrity")?.trim();
    if i.is_empty() {
        return None;
    }
    Some((
        i.to_string(),
        v.attr("crossorigin").map(|c| c.trim().to_ascii_lowercase()),
    ))
}

fn has_scheme(s: &str) -> bool {
    let Some((sch, _)) = s.split_once(':') else {
        return false;
    };
    let mut c = sch.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || matches!(x, '+' | '-' | '.'))
}

/// Mulțimea jetoanelor din atributul `rel`: separate prin spații ASCII, fără distincție
/// majuscule/minuscule, fără duplicate.
pub fn parse_rel(v: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in v.split(['\u{20}', '\t', '\n', '\u{0C}', '\r']) {
        if t.is_empty() {
            continue;
        }
        let t = t.to_ascii_lowercase();
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

struct Sink<'a> {
    base: &'a Url,
    out: &'a mut Extracted,
    /// `rel` al elementului în curs de prelucrare; consumat de prima legătură adăugată.
    rel: Vec<String>,
    /// Păstrează `tel:` și `data:` (validate ulterior) în loc să le ignore.
    keep_nonweb: bool,
    /// Rol impus pentru următoarea legătură adăugată (consumat de `add`).
    sub_force: Option<Sub>,
    /// `integrity` și `crossorigin` ale elementului în curs (consumate de `add`).
    sri: Option<(String, Option<String>)>,
}

impl<'a> Sink<'a> {
    fn add(&mut self, raw: &str, kind: LinkKind, text: &str) {
        let raw = raw.trim();
        if raw.is_empty() {
            return;
        }
        let lower = raw.to_ascii_lowercase();
        if lower.starts_with("javascript:")
            || (!self.keep_nonweb && lower.starts_with("data:"))
            || (!self.keep_nonweb && lower.starts_with("tel:"))
            || lower.starts_with("sms:")
            || lower.starts_with("blob:")
            || lower.starts_with("about:")
            || lower.starts_with("whatsapp:")
            || raw.contains("{{")
            || raw.contains("<%")
            || raw.contains("${")
        {
            return;
        }
        match self.base.join(raw) {
            Ok(u) => {
                let sri = self.sri.take();
                let sub = self
                    .sub_force
                    .take()
                    .unwrap_or_else(|| default_sub(kind, &u));
                self.out.links.push(RawLink {
                    // tipul urmează schema: un `mailto:` nu este „Pagină”
                    kind: match (u.scheme(), kind) {
                        ("mailto", _) => LinkKind::Mailto,
                        ("ftp" | "ftps", LinkKind::Page | LinkKind::Link) => LinkKind::Ftp,
                        _ => kind,
                    },
                    written: if has_scheme(raw) {
                        raw.to_string()
                    } else {
                        u.to_string()
                    },
                    url: u,
                    text: truncate_chars(text, 120),
                    rel: std::mem::take(&mut self.rel),
                    sub,
                    integrity: sri.as_ref().map(|x| x.0.clone()),
                    crossorigin: sri.and_then(|x| x.1),
                });
            }
            Err(e) => self
                .out
                .bad
                .push((raw.to_string(), kind, format!("URL ilizibil: {e}"))),
        }
    }

    fn add_srcset(&mut self, v: &str, kind: LinkKind) {
        let t = v.trim();
        if t.starts_with("data:") {
            return;
        }
        for part in t.split(',') {
            if let Some(u) = part.split_whitespace().next() {
                self.add(u, kind, "");
            }
        }
    }

    fn add_css(&mut self, css: &str, default_kind: LinkKind) {
        for c in css_import_re().captures_iter(css) {
            if let Some(m) = c.get(1).or_else(|| c.get(2)) {
                self.add(m.as_str(), LinkKind::Stylesheet, "");
            }
        }
        for c in css_url_re().captures_iter(css) {
            if let Some(m) = c.get(1).or_else(|| c.get(2)).or_else(|| c.get(3)) {
                // @import url(...) este tratat ca stil
                let s = m.start();
                let mut b = s.saturating_sub(5);
                while !css.is_char_boundary(b) {
                    b -= 1;
                }
                let before = &css[..b];
                let k = if before.trim_end().to_ascii_lowercase().ends_with("@import") {
                    LinkKind::Stylesheet
                } else {
                    default_kind
                };
                self.add(m.as_str(), k, "");
            }
        }
    }
}

pub fn extract_css(page: &Url, css: &str) -> Vec<RawLink> {
    let mut ex = Extracted::default();
    let mut s = Sink {
        base: page,
        out: &mut ex,
        rel: vec![],
        keep_nonweb: false,
        sub_force: None,
        sri: None,
    };
    s.add_css(css, LinkKind::Image);
    ex.links
}

fn parent_name(el: &ElementRef) -> String {
    el.parent()
        .and_then(ElementRef::wrap)
        .map(|p| p.value().name().to_string())
        .unwrap_or_default()
}

pub fn extract_html(page: &Url, html: &str, check_forms: bool) -> Extracted {
    extract_html_opts(page, html, check_forms, ExtractOpts::default())
}

/// Opțiuni ale extragerii; implicit comportamentul neschimbat (fără funcții noi).
#[derive(Clone, Copy, Debug, Default)]
pub struct ExtractOpts {
    /// Adresele `tel:` și `data:` se păstrează (pentru validare).
    pub keep_nonweb: bool,
    /// `link rel=alternate` cu tip RSS/Atom primește tipul `Feed` (altfel rămâne «Legătură»).
    pub detect_feeds: bool,
}

/// Ca `extract_html`, cu opțiuni explicite.
pub fn extract_html_opts(
    page: &Url,
    html: &str,
    check_forms: bool,
    opts: ExtractOpts,
) -> Extracted {
    let keep_nonweb = opts.keep_nonweb;
    // `<noscript>` este text brut pentru parserul cu scripting activ, dar conține linkuri reale
    // pentru vizitatorii fără JavaScript: îi eliminăm doar etichetele, ca să fie parcurs conținutul.
    static NOSCRIPT: OnceLock<Regex> = OnceLock::new();
    let nos = NOSCRIPT.get_or_init(|| Regex::new(r"(?i)</?noscript\b[^>]*>").unwrap());
    let unwrapped = nos.replace_all(html, "");
    let (capped, parse_truncated) = cap_parse_events(&unwrapped);
    let doc = Html::parse_document(capped);
    let all = Selector::parse("*").unwrap();
    let mut out = Extracted {
        parse_truncated,
        ..Extracted::default()
    };

    // <base href>
    let mut base = page.clone();
    if let Ok(sel) = Selector::parse("base[href]")
        && let Some(b) = doc.select(&sel).next()
        && let Some(h) = b.value().attr("href")
        && let Ok(u) = page.join(h.trim())
    {
        base = u;
    }

    // Titlu
    if let Ok(sel) = Selector::parse("title")
        && let Some(t) = doc.select(&sel).next()
    {
        let txt = truncate_chars(&t.text().collect::<String>(), 200);
        if !txt.is_empty() {
            out.title = Some(txt);
        }
    }

    if let Ok(sel) = Selector::parse("h1")
        && let Some(t) = doc.select(&sel).next()
    {
        let txt = truncate_chars(
            &t.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            200,
        );
        if !txt.is_empty() {
            out.h1 = Some(txt);
        }
    }

    let mut links: Vec<RawLink> = vec![];
    let mut bad = vec![];
    let mut anchors = HashSet::new();
    let mut meta_robots = None;
    let mut canonicals: Vec<Url> = vec![];
    let mut hreflangs: Vec<String> = vec![];
    let mut sri_notes: Vec<String> = vec![];
    {
        let mut tmp = Extracted::default();
        let mut s = Sink {
            base: &base,
            out: &mut tmp,
            rel: vec![],
            keep_nonweb,
            sub_force: None,
            sri: None,
        };

        for el in doc.select(&all) {
            let v = el.value();
            let name = v.name();

            if v.attr("integrity").is_some() {
                let applicable = match name {
                    "script" => true,
                    "link" => parse_rel(v.attr("rel").unwrap_or(""))
                        .iter()
                        .any(|t| matches!(t.as_str(), "stylesheet" | "preload" | "modulepreload")),
                    _ => false,
                };
                if !applicable {
                    sri_notes.push(name.to_string());
                }
            }

            if let Some(id) = v.attr("id")
                && !id.is_empty()
            {
                anchors.insert(id.to_string());
            }
            if name == "a"
                && let Some(n) = v.attr("name")
                && !n.is_empty()
            {
                anchors.insert(n.to_string());
            }
            if let Some(st) = v.attr("style") {
                s.add_css(st, LinkKind::Image);
            }
            for lazy in ["data-src", "data-original", "data-lazy-src"] {
                if let Some(x) = v.attr(lazy)
                    && matches!(name, "img" | "source" | "video" | "iframe")
                {
                    s.add(x, LinkKind::Image, v.attr("alt").unwrap_or(""));
                }
            }
            if let Some(x) = v.attr("data-srcset") {
                s.add_srcset(x, LinkKind::Image);
            }

            match name {
                "a" | "area" => {
                    if let Some(h) = v.attr("href") {
                        let mut text = el.text().collect::<String>();
                        if text.trim().is_empty() {
                            if let Some(alt) = v.attr("alt").or_else(|| v.attr("title")) {
                                text = alt.to_string();
                            } else {
                                if let Ok(isel) = Selector::parse("img[alt]")
                                    && let Some(im) = el.select(&isel).next()
                                {
                                    text = im.value().attr("alt").unwrap_or("").to_string();
                                }
                            }
                        }
                        if name == "a"
                            && let Some(hl) = v.attr("hreflang")
                        {
                            hreflangs.push(hl.to_string());
                        }
                        s.rel = parse_rel(v.attr("rel").unwrap_or(""));
                        s.add(h, LinkKind::Page, &text);
                        s.rel.clear();
                    }
                }
                "link" => {
                    let rel = v.attr("rel").unwrap_or("").to_ascii_lowercase();
                    if rel.contains("dns-prefetch") || rel.contains("preconnect") {
                        continue;
                    }
                    let rel_tokens = parse_rel(&rel);
                    if let Some(h) = v.attr("href") {
                        if rel_tokens.iter().any(|t| t == "canonical")
                            && let Ok(u) = base.join(h.trim())
                        {
                            canonicals.push(u);
                        }
                        if rel_tokens.iter().any(|t| t == "alternate")
                            && let Some(hl) = v.attr("hreflang")
                        {
                            hreflangs.push(hl.to_string());
                        }
                        let is_feed = opts.detect_feeds
                            && rel_tokens.iter().any(|t| t == "alternate")
                            && v.attr("type").is_some_and(crate::feed::is_feed_type);
                        let kind = if is_feed {
                            LinkKind::Feed
                        } else if rel.contains("stylesheet") {
                            LinkKind::Stylesheet
                        } else if rel.contains("icon") {
                            LinkKind::Icon
                        } else if rel.contains("preload") || rel.contains("prefetch") {
                            match v.attr("as").unwrap_or("") {
                                "style" => LinkKind::Stylesheet,
                                "script" => LinkKind::Script,
                                "image" => LinkKind::Image,
                                _ => LinkKind::Link,
                            }
                        } else {
                            LinkKind::Link
                        };
                        s.rel = parse_rel(v.attr("rel").unwrap_or(""));
                        if rel_tokens
                            .iter()
                            .any(|t| matches!(t.as_str(), "preload" | "prefetch" | "modulepreload"))
                        {
                            s.sub_force = Some(Sub::Blockable);
                        }
                        if rel_tokens.iter().any(|t| {
                            matches!(t.as_str(), "stylesheet" | "preload" | "modulepreload")
                        }) {
                            s.sri = sri_of(v);
                        }
                        s.add(h, kind, "");
                        s.rel.clear();
                        s.sub_force = None;
                        s.sri = None;
                    }
                    if let Some(x) = v.attr("imagesrcset") {
                        s.add_srcset(x, LinkKind::Image);
                    }
                }
                "img" => {
                    let alt = v.attr("alt").unwrap_or("");
                    if let Some(x) = v.attr("src") {
                        s.add(x, LinkKind::Image, alt);
                    }
                    if let Some(x) = v.attr("srcset") {
                        s.add_srcset(x, LinkKind::Image);
                    }
                }
                "source" => {
                    let kind = match parent_name(&el).as_str() {
                        "video" | "audio" => LinkKind::Media,
                        _ => LinkKind::Image,
                    };
                    if let Some(x) = v.attr("src") {
                        s.add(x, kind, "");
                    }
                    if let Some(x) = v.attr("srcset") {
                        s.add_srcset(x, kind);
                    }
                }
                "video" => {
                    if let Some(x) = v.attr("src") {
                        s.add(x, LinkKind::Media, "");
                    }
                    if let Some(x) = v.attr("poster") {
                        s.add(x, LinkKind::Image, "");
                    }
                }
                "audio" | "track" => {
                    if let Some(x) = v.attr("src") {
                        s.add(x, LinkKind::Media, "");
                    }
                }
                "embed" => {
                    if let Some(x) = v.attr("src") {
                        s.add(x, LinkKind::Object, "");
                    }
                }
                "object" => {
                    if let Some(x) = v.attr("data") {
                        s.add(x, LinkKind::Object, "");
                    }
                }
                "param" => {
                    let n = v.attr("name").unwrap_or("").to_ascii_lowercase();
                    if matches!(n.as_str(), "movie" | "src" | "url")
                        && let Some(x) = v.attr("value")
                    {
                        s.add(x, LinkKind::Object, "");
                    }
                }
                "applet" => {
                    for a in ["code", "archive", "object"] {
                        if let Some(x) = v.attr(a) {
                            s.add(x, LinkKind::Object, "");
                        }
                    }
                }
                "iframe" | "frame" => {
                    if let Some(x) = v.attr("src") {
                        s.add(x, LinkKind::Frame, "");
                    }
                }
                "script" => {
                    if let Some(x) = v.attr("src") {
                        s.sri = sri_of(v);
                        s.add(x, LinkKind::Script, "");
                        s.sri = None;
                    }
                }
                "form" => {
                    if check_forms && let Some(x) = v.attr("action") {
                        s.rel = parse_rel(v.attr("rel").unwrap_or(""));
                        s.add(x, LinkKind::Form, "");
                        s.rel.clear();
                    }
                }
                "input" => {
                    if v.attr("type").map(|t| t.eq_ignore_ascii_case("image")) == Some(true)
                        && let Some(x) = v.attr("src")
                    {
                        s.add(x, LinkKind::Image, "");
                    }
                }
                "body" | "table" | "tr" | "td" | "th" => {
                    if let Some(x) = v.attr("background") {
                        s.add(x, LinkKind::Image, "");
                    }
                }
                "blockquote" | "q" | "ins" | "del" => {
                    if let Some(x) = v.attr("cite") {
                        s.add(x, LinkKind::Link, "");
                    }
                }
                "image" | "use" => {
                    if let Some(x) = v.attr("href").or_else(|| v.attr("xlink:href"))
                        && !x.starts_with('#')
                    {
                        s.add(x, LinkKind::Image, "");
                    }
                }
                "meta" => {
                    let he = v.attr("http-equiv").unwrap_or("").to_ascii_lowercase();
                    let nm = v
                        .attr("name")
                        .or_else(|| v.attr("property"))
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let content = v.attr("content").unwrap_or("");
                    if he == "refresh" {
                        if let Some(c) = refresh_re().captures(content) {
                            s.add(c.get(1).unwrap().as_str(), LinkKind::Page, "meta refresh");
                        }
                    } else if matches!(
                        nm.as_str(),
                        "og:image" | "twitter:image" | "msapplication-tileimage"
                    ) {
                        s.sub_force = Some(Sub::None);
                        s.add(content, LinkKind::Image, "");
                        s.sub_force = None;
                    } else if nm == "robots" {
                        meta_robots = Some(content.to_ascii_lowercase());
                    }
                }
                "style" => {
                    let txt = el.text().collect::<String>();
                    s.add_css(&txt, LinkKind::Image);
                }
                _ => {}
            }
        }
        links.append(&mut tmp.links);
        bad.append(&mut tmp.bad);
    }

    out.links = links;
    out.bad = bad;
    out.anchors = anchors;
    out.meta_robots = meta_robots;
    out.canonicals = canonicals;
    out.hreflangs = hreflangs;
    out.sri_notes = sri_notes;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(e: &Extracted) -> Vec<(String, LinkKind)> {
        e.links
            .iter()
            .map(|l| (l.url.to_string(), l.kind))
            .collect()
    }

    #[test]
    fn noscript_links_are_extracted() {
        let page = Url::parse("https://ex.ro/").unwrap();
        let html = "<html><body><noscript><a href=\"/fara-js\">x</a><img src=\"/px.gif\"></noscript></body></html>";
        let k = kinds(&extract_html(&page, html, false));
        assert!(k.iter().any(|(u, _)| u == "https://ex.ro/fara-js"));
        assert!(k.iter().any(|(u, _)| u == "https://ex.ro/px.gif"));
    }

    #[test]
    fn extracts_common_elements() {
        let page = Url::parse("https://ex.ro/dir/index.html").unwrap();
        let html = r##"<html><head><title> Salut   lume </title>
            <link rel="stylesheet" href="s.css"><link rel="icon" href="/f.ico">
            <script src="app.js"></script>
            <meta http-equiv="refresh" content="3; url=/nou">
            <style>body{background:url(bg.png)} @import "extra.css";</style></head>
            <body background="b.gif" style="background-image:url('inl.jpg')">
            <a href="a.html">Link <b>A</b></a><a href="#sus">sus</a><a href="mailto:x@y.ro">m</a>
            <a href="javascript:void(0)">js</a>
            <img src="i.png" alt="poza" srcset="i2.png 2x, i3.png 3x">
            <iframe src="https://alt.ro/x"></iframe>
            <video src="v.mp4" poster="p.jpg"></video>
            <a id="ancora" name="veche"></a></body></html>"##;
        let e = extract_html(&page, html, false);
        assert_eq!(e.title.as_deref(), Some("Salut lume"));
        let k = kinds(&e);
        let has = |u: &str, kd: LinkKind| k.iter().any(|(x, y)| x == u && *y == kd);
        assert!(has("https://ex.ro/dir/s.css", LinkKind::Stylesheet));
        assert!(has("https://ex.ro/f.ico", LinkKind::Icon));
        assert!(has("https://ex.ro/dir/app.js", LinkKind::Script));
        assert!(has("https://ex.ro/nou", LinkKind::Page));
        assert!(has("https://ex.ro/dir/bg.png", LinkKind::Image));
        assert!(has("https://ex.ro/dir/extra.css", LinkKind::Stylesheet));
        assert!(has("https://ex.ro/dir/b.gif", LinkKind::Image));
        assert!(has("https://ex.ro/dir/inl.jpg", LinkKind::Image));
        assert!(has("https://ex.ro/dir/a.html", LinkKind::Page));
        assert!(has("https://ex.ro/dir/i2.png", LinkKind::Image));
        assert!(has("https://alt.ro/x", LinkKind::Frame));
        assert!(has("https://ex.ro/dir/v.mp4", LinkKind::Media));
        assert!(has("https://ex.ro/dir/p.jpg", LinkKind::Image));
        assert!(has("mailto:x@y.ro", LinkKind::Mailto));
        assert!(!k.iter().any(|(x, _)| x.starts_with("javascript")));
        assert!(e.anchors.contains("ancora") && e.anchors.contains("veche"));
        let a = e
            .links
            .iter()
            .find(|l| l.url.path().ends_with("a.html"))
            .unwrap();
        assert_eq!(a.text, "Link A");
    }

    #[test]
    fn base_href_is_respected() {
        let page = Url::parse("https://ex.ro/x/y.html").unwrap();
        let e = extract_html(
            &page,
            r#"<base href="https://cdn.ro/z/"><a href="p.html">p</a>"#,
            false,
        );
        assert_eq!(e.links[0].url.as_str(), "https://cdn.ro/z/p.html");
    }

    #[test]
    fn css_extraction() {
        let page = Url::parse("https://ex.ro/c/s.css").unwrap();
        let l = extract_css(
            &page,
            r#"@import url("a.css"); .x{background:url(../img/b.png)} .y{src:url('f.woff2')}"#,
        );
        let u: Vec<_> = l.iter().map(|x| x.url.to_string()).collect();
        assert!(u.contains(&"https://ex.ro/c/a.css".to_string()));
        assert!(u.contains(&"https://ex.ro/img/b.png".to_string()));
        assert!(u.contains(&"https://ex.ro/c/f.woff2".to_string()));
    }

    #[test]
    fn css_url_after_multibyte_does_not_panic() {
        let u = Url::parse("https://a.ro/s.css").unwrap();
        let l = extract_css(&u, "x{background:€url(a.png)}");
        assert_eq!(l.len(), 1);
        let e = extract_html(&u, "<div style=\"background:ăurl(a.png)\">", false);
        assert_eq!(e.links.len(), 1);
    }

    #[test]
    fn rel_tokens_are_a_case_insensitive_unique_set() {
        assert_eq!(parse_rel("NoFollow noopener"), vec!["nofollow", "noopener"]);
        assert!(parse_rel("  ").is_empty());
        assert!(parse_rel("").is_empty());
        assert_eq!(parse_rel("nofollow nofollow"), vec!["nofollow"]);
        assert_eq!(parse_rel("a\tb\nc\r\u{0C}d"), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn rel_on_anchor_requires_href_and_link_keeps_stylesheet() {
        let page = Url::parse("https://ex.ro/").unwrap();
        let e = extract_html(
            &page,
            r#"<a rel="nofollow">x</a><a href="/p" rel="NoFollow noopener">y</a>
               <link rel="stylesheet nofollow" href="/s.css">"#,
            false,
        );
        assert_eq!(e.links.len(), 2);
        assert_eq!(e.links[0].rel, vec!["nofollow", "noopener"]);
        assert_eq!(e.links[1].kind, LinkKind::Stylesheet);
        assert_eq!(e.links[1].rel, vec!["stylesheet", "nofollow"]);
    }

    #[test]
    fn rel_on_form_only_when_forms_are_checked() {
        let page = Url::parse("https://ex.ro/").unwrap();
        let h = r#"<form action="/f" rel="nofollow"></form>"#;
        assert!(extract_html(&page, h, false).links.is_empty());
        assert_eq!(extract_html(&page, h, true).links[0].rel, vec!["nofollow"]);
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    #[test]
    fn nul_flood_is_cut_at_the_parse_budget() {
        // 4 MB de NUL ocupau ~400 MB în parser (amplificare ≈100×)
        let html = format!("<a href=\"/ok\">x</a>{}", "\0".repeat(4 << 20));
        let (cut, truncated) = cap_parse_events(&html);
        assert!(truncated);
        assert!(cut.len() < 300_000, "tăiat la {} octeți", cut.len());
        let page = Url::parse("http://example.org/").unwrap();
        let e = extract_html(&page, &html, false);
        assert!(e.parse_truncated);
        // începutul paginii rămâne analizat
        assert!(e.links.iter().any(|l| l.url.path() == "/ok"));
    }

    #[test]
    fn lt_flood_and_dense_tags_are_cut_but_normal_pages_are_not() {
        let (_, t) = cap_parse_events(&"<".repeat(MAX_PARSE_EVENTS + 1));
        assert!(t);
        let (_, t) = cap_parse_events(&"<".repeat(MAX_PARSE_EVENTS));
        assert!(!t, "exact bugetul nu se taie");
        // o pagină mare, dar reală: 20.000 de legături (≈40.000 de semne «<»)
        let big: String = (0..20_000)
            .map(|i| format!("<li><a href=\"/p/{i}\">p</a></li>"))
            .collect();
        let page = Url::parse("http://example.org/").unwrap();
        let e = extract_html(&page, &big, false);
        assert!(!e.parse_truncated);
        assert_eq!(e.links.len(), 20_000);
    }

    #[test]
    fn cut_never_splits_a_multibyte_character() {
        let s = format!("é{}ü", "<".repeat(MAX_PARSE_EVENTS + 5));
        let (cut, t) = cap_parse_events(&s);
        assert!(t && cut.starts_with('é'));
        assert!(std::str::from_utf8(cut.as_bytes()).is_ok());
    }
}
