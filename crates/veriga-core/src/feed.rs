//! Extragerea legăturilor din fluxuri RSS 2.0 și Atom 1.0 (cu `xml:base`).

use crate::extract::{RawLink, Sub, extract_html};
use crate::model::LinkKind;
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedError {
    /// XML prost format; nicio legătură nu se păstrează.
    Malformed(String),
    /// Rădăcina nu este `rss` sau `feed`.
    NotAFeed(String),
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(e) => write!(f, "XML prost format: {e}"),
            Self::NotAFeed(r) => write!(f, "nu este RSS sau Atom (rădăcina «{r}»)"),
        }
    }
}

const MAX_DEPTH: usize = 128;

#[derive(Default, Debug)]
struct Node {
    /// Nume local (fără prefix).
    name: String,
    /// Nume complet, cu prefix.
    full: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<Node>,
}

impl Node {
    fn attr(&self, n: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.as_str())
    }
    fn child(&self, n: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == n)
    }
    fn kids<'a>(&'a self, n: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == n)
    }
    fn text(&self) -> &str {
        self.text.trim()
    }
}

fn open_node(e: &quick_xml::events::BytesStart<'_>) -> Result<Node, FeedError> {
    let bad = |x: &dyn std::fmt::Display| FeedError::Malformed(x.to_string());
    let mut attrs = vec![];
    for a in e.attributes() {
        let a = a.map_err(|x| bad(&x))?;
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let v = a
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|x| bad(&x))?
            .into_owned();
        attrs.push((k, v));
    }
    Ok(Node {
        name: String::from_utf8_lossy(e.local_name().as_ref()).into_owned(),
        full: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
        attrs,
        ..Default::default()
    })
}

fn parse_tree(xml: &str) -> Result<Node, FeedError> {
    let bad = |x: &dyn std::fmt::Display| FeedError::Malformed(x.to_string());
    let mut rd = Reader::from_str(xml);
    let mut stack: Vec<Node> = vec![];
    let mut root: Option<Node> = None;
    loop {
        match rd.read_event().map_err(|e| bad(&e))? {
            Event::Start(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(FeedError::Malformed("imbricare prea adâncă".into()));
                }
                stack.push(open_node(&e)?);
            }
            Event::Empty(e) => {
                let n = open_node(&e)?;
                match stack.last_mut() {
                    Some(p) => p.children.push(n),
                    None if root.is_none() => root = Some(n),
                    None => return Err(FeedError::Malformed("mai multe rădăcini".into())),
                }
            }
            Event::End(_) => {
                let n = stack
                    .pop()
                    .ok_or_else(|| FeedError::Malformed("etichetă de închidere în plus".into()))?;
                match stack.last_mut() {
                    Some(p) => p.children.push(n),
                    None if root.is_none() => root = Some(n),
                    None => return Err(FeedError::Malformed("mai multe rădăcini".into())),
                }
            }
            Event::Text(t) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&t.decode().map_err(|e| bad(&e))?);
                }
            }
            Event::CData(t) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&t.decode().map_err(|e| bad(&e))?);
                }
            }
            Event::GeneralRef(r) => {
                if let Some(n) = stack.last_mut() {
                    if let Some(c) = r.resolve_char_ref().map_err(|e| bad(&e))? {
                        n.text.push(c);
                    } else {
                        match r.decode().map_err(|e| bad(&e))?.as_ref() {
                            "amp" => n.text.push('&'),
                            "lt" => n.text.push('<'),
                            "gt" => n.text.push('>'),
                            "quot" => n.text.push('"'),
                            "apos" => n.text.push('\''),
                            _ => {} // entitățile definite de document nu se expandează
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(FeedError::Malformed("document neînchis".into()));
    }
    root.ok_or_else(|| FeedError::Malformed("document gol".into()))
}

struct Out {
    links: Vec<RawLink>,
}

impl Out {
    fn add(&mut self, base: &Url, href: &str, kind: LinkKind, role: &str, rel: Option<&str>) {
        let href = href.trim();
        if href.is_empty() {
            return;
        }
        let Ok(u) = base.join(href) else { return };
        self.links.push(RawLink {
            written: if href.contains("://") {
                href.to_string()
            } else {
                u.to_string()
            },
            url: u,
            kind,
            text: format!("Feed: {role}"),
            rel: rel
                .map(|r| vec![r.to_ascii_lowercase()])
                .unwrap_or_default(),
            sub: Sub::None,
            integrity: None,
            crossorigin: None,
        });
    }

    /// Legături dintr-un fragment HTML (descriere, conținut).
    fn add_html(&mut self, base: &Url, html: &str) {
        for l in extract_html(base, html, false).links {
            let mut l = l;
            l.text = format!("Feed: {}", l.text);
            l.sub = Sub::None;
            l.integrity = None;
            self.links.push(l);
        }
    }

    /// Legături dintr-un subarbore XHTML.
    fn add_xhtml(&mut self, base: &Url, n: &Node) {
        let b = node_base(n, base);
        let attr_kind = |name: &str| match name {
            "a" | "area" => Some(("href", LinkKind::Page)),
            "img" => Some(("src", LinkKind::Image)),
            "iframe" | "frame" => Some(("src", LinkKind::Frame)),
            "script" => Some(("src", LinkKind::Script)),
            "audio" | "video" | "source" => Some(("src", LinkKind::Media)),
            _ => None,
        };
        if let Some((a, k)) = attr_kind(&n.name)
            && let Some(v) = n.attr(a)
        {
            self.add(&b, v, k, "xhtml", None);
        }
        for c in &n.children {
            self.add_xhtml(&b, c);
        }
    }
}

/// Baza unui element: `xml:base` propriu, rezolvat față de baza moștenită.
fn node_base(n: &Node, inherited: &Url) -> Url {
    n.attr("xml:base")
        .and_then(|b| inherited.join(b.trim()).ok())
        .unwrap_or_else(|| inherited.clone())
}

fn rss_item(it: &Node, base: &Url, chan_link: &Url, out: &mut Out) {
    let b = node_base(it, base);
    let mut item_link = chan_link.clone();
    if let Some(l) = it.child("link") {
        let lb = node_base(l, &b);
        out.add(&lb, l.text(), LinkKind::Page, "item/link", None);
        if let Ok(u) = lb.join(l.text()) {
            item_link = u;
        }
    }
    if let Some(c) = it.child("comments") {
        out.add(
            &node_base(c, &b),
            c.text(),
            LinkKind::Page,
            "item/comments",
            None,
        );
    }
    for e in it.kids("enclosure") {
        if let Some(u) = e.attr("url") {
            out.add(
                &node_base(e, &b),
                u,
                LinkKind::Media,
                "item/enclosure",
                None,
            );
        }
    }
    for s in it.kids("source") {
        if let Some(u) = s.attr("url") {
            out.add(&node_base(s, &b), u, LinkKind::Page, "item/source", None);
        }
    }
    if let Some(g) = it.child("guid") {
        let perma = !g
            .attr("isPermaLink")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"));
        // adresă = absolută; un identificator oarecare nu este legătură
        if perma && let Ok(u) = Url::parse(g.text()) {
            out.add(&b, u.as_str(), LinkKind::Page, "item/guid", None);
        }
    }
    for n in &it.children {
        if n.name == "description" || n.full == "content:encoded" || n.name == "encoded" {
            out.add_html(&item_link, &n.text);
        }
    }
}

fn rss(root: &Node, feed: &Url) -> Vec<RawLink> {
    let mut out = Out { links: vec![] };
    let rb = node_base(root, feed);
    let Some(ch) = root.child("channel") else {
        return out.links;
    };
    let cb = node_base(ch, &rb);
    let mut chan_link = cb.clone();
    if let Some(l) = ch.child("link") {
        out.add(&cb, l.text(), LinkKind::Page, "channel/link", None);
        if let Ok(u) = cb.join(l.text()) {
            chan_link = u;
        }
    }
    if let Some(im) = ch.child("image") {
        let ib = node_base(im, &cb);
        if let Some(u) = im.child("url") {
            out.add(&ib, u.text(), LinkKind::Image, "channel/image/url", None);
        }
        if let Some(l) = im.child("link") {
            out.add(&ib, l.text(), LinkKind::Page, "channel/image/link", None);
        }
    }
    if let Some(d) = ch.child("description") {
        out.add_html(&chan_link, &d.text);
    }
    for it in ch.kids("item") {
        rss_item(it, &cb, &chan_link, &mut out);
    }
    out.links
}

fn atom_text_links(n: &Node, base: &Url, out: &mut Out) {
    match n
        .attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("text/html") => out.add_html(base, &n.text),
        Some("xhtml") | Some("application/xhtml+xml") => {
            for c in &n.children {
                out.add_xhtml(base, c);
            }
        }
        _ => {}
    }
}

fn atom_entry_or_feed(n: &Node, base: &Url, out: &mut Out) {
    for l in n.kids("link") {
        let lb = node_base(l, base);
        if let Some(h) = l.attr("href") {
            let rel = l
                .attr("rel")
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .unwrap_or("alternate");
            let kind = if rel.eq_ignore_ascii_case("enclosure") {
                LinkKind::Media
            } else {
                LinkKind::Page
            };
            out.add(&lb, h, kind, &format!("link/{rel}"), Some(rel));
        }
    }
}

fn atom(root: &Node, feed: &Url) -> Vec<RawLink> {
    let mut out = Out { links: vec![] };
    let fb = node_base(root, feed);
    atom_entry_or_feed(root, &fb, &mut out);
    for sub in root.kids("subtitle") {
        atom_text_links(sub, &fb, &mut out);
    }
    for e in root.kids("entry") {
        let eb = node_base(e, &fb);
        atom_entry_or_feed(e, &eb, &mut out);
        for c in e.kids("content") {
            let cb = node_base(c, &eb);
            if let Some(src) = c.attr("src") {
                out.add(&cb, src, LinkKind::Page, "entry/content", None);
            }
            atom_text_links(c, &cb, &mut out);
        }
        if let Some(s) = e.child("summary") {
            atom_text_links(s, &node_base(s, &eb), &mut out);
        }
    }
    out.links
}

/// Legăturile unui flux RSS 2.0 sau Atom 1.0 aflat la `feed_url`.
pub fn extract_links(feed_url: &Url, xml: &str) -> Result<Vec<RawLink>, FeedError> {
    let root = parse_tree(xml)?;
    match root.name.as_str() {
        "rss" => Ok(rss(&root, feed_url)),
        "feed" => Ok(atom(&root, feed_url)),
        other => Err(FeedError::NotAFeed(other.to_string())),
    }
}

/// Valoarea `type` a unui `link rel=alternate` care indică un flux.
pub fn is_feed_type(t: &str) -> bool {
    let t = t
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    t == "application/rss+xml" || t == "application/atom+xml"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links(url: &str, xml: &str) -> Vec<(String, LinkKind)> {
        extract_links(&Url::parse(url).unwrap(), xml)
            .unwrap()
            .into_iter()
            .map(|l| (l.url.to_string(), l.kind))
            .collect()
    }

    #[test]
    fn t1_rss_link_and_enclosure() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<rss version="2.0"><channel><item><link>https://ex.org/p1</link><enclosure url="https://ex.org/a.mp3" length="1" type="audio/mpeg"/></item></channel></rss>"#,
        );
        assert_eq!(l.len(), 2);
        assert!(l.contains(&("https://ex.org/p1".into(), LinkKind::Page)));
        assert!(l.contains(&("https://ex.org/a.mp3".into(), LinkKind::Media)));
    }

    #[test]
    fn rss_channel_fields_and_guid_permalink_default() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<rss><channel><link>https://ex.org/</link><image><url>https://ex.org/l.png</url><link>https://ex.org/</link></image>
               <item><guid>https://ex.org/g</guid><comments>https://ex.org/c</comments><source url="https://o.org/f"/></item></channel></rss>"#,
        );
        let urls: Vec<_> = l.iter().map(|x| x.0.as_str()).collect();
        for u in [
            "https://ex.org/",
            "https://ex.org/l.png",
            "https://ex.org/g",
            "https://ex.org/c",
            "https://o.org/f",
        ] {
            assert!(urls.contains(&u), "{u} in {urls:?}");
        }
    }

    #[test]
    fn t2_guid_not_permalink() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<rss><channel><item><guid isPermaLink="false">abc123</guid></item></channel></rss>"#,
        );
        assert!(l.is_empty());
    }

    #[test]
    fn t3_atom_link_default_alternate() {
        let l = extract_links(
            &Url::parse("https://ex.org/f.xml").unwrap(),
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><link href="/a"/></feed>"#,
        )
        .unwrap();
        assert_eq!(l[0].url.as_str(), "https://ex.org/a");
        assert_eq!(l[0].rel, vec!["alternate"]);
    }

    #[test]
    fn t4_xml_base() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<feed xmlns="http://www.w3.org/2005/Atom" xml:base="https://cdn.ex/z/"><link href="p.html"/></feed>"#,
        );
        assert_eq!(l[0].0, "https://cdn.ex/z/p.html");
        // xml:base pe un descendent se aplică peste cel moștenit
        let l = links(
            "https://ex.org/f.xml",
            r#"<feed xmlns="http://www.w3.org/2005/Atom" xml:base="https://cdn.ex/z/"><entry xml:base="sub/"><link href="p.html"/></entry></feed>"#,
        );
        assert_eq!(l[0].0, "https://cdn.ex/z/sub/p.html");
    }

    #[test]
    fn t5_cdata_html_in_description() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<rss><channel><item><link>https://ex.org/i</link><description><![CDATA[<p><a href="http://x.ex/q">q</a> <a href="rel.html">r</a></p>]]></description></item></channel></rss>"#,
        );
        assert!(l.iter().any(|x| x.0 == "http://x.ex/q"));
        // relative la item/link
        assert!(l.iter().any(|x| x.0 == "https://ex.org/rel.html"));
    }

    #[test]
    fn escaped_html_is_decoded_once_by_xml() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<rss><channel><item><description>&lt;a href=&quot;http://x.ex/e?a=1&amp;amp;b=2&quot;&gt;x&lt;/a&gt;</description></item></channel></rss>"#,
        );
        assert!(l.iter().any(|x| x.0 == "http://x.ex/e?a=1&b=2"), "{l:?}");
    }

    #[test]
    fn t6_not_a_feed() {
        let e = extract_links(
            &Url::parse("https://ex.org/").unwrap(),
            "<html><body/></html>",
        );
        assert_eq!(e.unwrap_err(), FeedError::NotAFeed("html".into()));
    }

    #[test]
    fn t7_atom_content_src_and_html_type() {
        let l = links(
            "https://ex.org/f.xml",
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><content src="http://x.ex/doc.pdf" type="application/pdf"/>
               <summary type="html">&lt;a href="http://s.ex/1"&gt;s&lt;/a&gt;</summary>
               <content type="xhtml"><div xmlns="http://www.w3.org/1999/xhtml"><a href="http://s.ex/2">x</a></div></content></entry></feed>"#,
        );
        for u in ["http://x.ex/doc.pdf", "http://s.ex/1", "http://s.ex/2"] {
            assert!(l.iter().any(|x| x.0 == u), "{u} in {l:?}");
        }
    }

    #[test]
    fn atom_link_roles() {
        let l = extract_links(
            &Url::parse("https://ex.org/f.xml").unwrap(),
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><link rel="self" href="/f.xml"/><entry><link rel="enclosure" href="/m.mp3"/><link rel="replies" href="/r"/><link rel="http://ex.org/custom" href="/c"/></entry></feed>"#,
        )
        .unwrap();
        assert_eq!(l.len(), 4);
        assert_eq!(l[1].kind, LinkKind::Media);
        assert_eq!(l[3].rel, vec!["http://ex.org/custom"]);
    }

    #[test]
    fn malformed_xml_gives_error_no_links() {
        let e = extract_links(
            &Url::parse("https://ex.org/f.xml").unwrap(),
            "<rss><channel><item><link>https://ex.org/p</link></channel></rss>",
        );
        assert!(matches!(e, Err(FeedError::Malformed(_))));
        assert!(extract_links(&Url::parse("https://ex.org/").unwrap(), "<rss><channel>").is_err());
    }

    #[test]
    fn custom_entities_are_not_expanded() {
        let l = extract_links(
            &Url::parse("https://ex.org/f.xml").unwrap(),
            r#"<?xml version="1.0"?><!DOCTYPE rss [<!ENTITY x "http://evil.ex/">]><rss><channel><link>&x;</link></channel></rss>"#,
        );
        assert!(l.map(|v| v.is_empty()).unwrap_or(true));
    }

    #[test]
    fn feed_type_detection() {
        assert!(is_feed_type("application/rss+xml"));
        assert!(is_feed_type(" Application/Atom+XML; charset=utf-8"));
        assert!(!is_feed_type("text/html"));
    }
}
