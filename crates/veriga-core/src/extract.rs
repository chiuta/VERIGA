//! Extragerea linkurilor din HTML și CSS.

use crate::model::LinkKind;
use crate::urlutil::truncate_chars;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashSet;
use std::sync::OnceLock;
use url::Url;

#[derive(Clone, Debug)]
pub struct RawLink {
    pub url: Url,
    pub kind: LinkKind,
    pub text: String,
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

struct Sink<'a> {
    base: &'a Url,
    out: &'a mut Extracted,
}

impl<'a> Sink<'a> {
    fn add(&mut self, raw: &str, kind: LinkKind, text: &str) {
        let raw = raw.trim();
        if raw.is_empty() {
            return;
        }
        let lower = raw.to_ascii_lowercase();
        if lower.starts_with("javascript:")
            || lower.starts_with("data:")
            || lower.starts_with("tel:")
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
            Ok(u) => self.out.links.push(RawLink {
                url: u,
                kind,
                text: truncate_chars(text, 120),
            }),
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
    let doc = Html::parse_document(html);
    let all = Selector::parse("*").unwrap();
    let mut out = Extracted::default();

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
    {
        let mut tmp = Extracted::default();
        let mut s = Sink {
            base: &base,
            out: &mut tmp,
        };

        for el in doc.select(&all) {
            let v = el.value();
            let name = v.name();

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
                        s.add(h, LinkKind::Page, &text);
                    }
                }
                "link" => {
                    let rel = v.attr("rel").unwrap_or("").to_ascii_lowercase();
                    if rel.contains("dns-prefetch") || rel.contains("preconnect") {
                        continue;
                    }
                    if let Some(h) = v.attr("href") {
                        let kind = if rel.contains("stylesheet") {
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
                        s.add(h, kind, "");
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
                        s.add(x, LinkKind::Script, "");
                    }
                }
                "form" => {
                    if check_forms && let Some(x) = v.attr("action") {
                        s.add(x, LinkKind::Form, "");
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
                        s.add(content, LinkKind::Image, "");
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
        assert!(has("mailto:x@y.ro", LinkKind::Page));
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
}
