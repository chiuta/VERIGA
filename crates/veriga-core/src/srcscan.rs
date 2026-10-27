//! Scanare *offline* a fișierelor-sursă (HTML, Markdown, export WordPress WXR,
//! sitemap XML): găsește fiecare link împreună cu poziția exactă (octeți) în
//! fișier, ca reparatorul să-l poată modifica fără a rescrie restul documentului.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SrcKind {
    Html,
    Markdown,
    Wxr,
    Sitemap,
}

/// Un link găsit într-un fișier-sursă.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Occurrence {
    pub file: PathBuf,
    /// Interval [start, end) în octeții fișierului (valoarea brută, fără ghilimele).
    pub start: usize,
    pub end: usize,
    /// URL-ul decodat (`&amp;` → `&`).
    pub url: String,
    /// Valoarea brută conține `&amp;` (se re-codifică la scriere).
    pub amp: bool,
    pub src: SrcKind,
    /// Atributul / construcția (`href`, `src`, `md`, `loc`, `menu`).
    pub attr: String,
    /// Textul ancorei (dacă e cunoscut).
    pub text: String,
    /// Link dintr-un meniu de navigare.
    pub nav: bool,
    /// Eticheta „paginii” (cale sau titlul articolului din WXR).
    pub page: String,
}

pub fn detect(path: &Path, head: &str) -> Option<SrcKind> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => Some(SrcKind::Markdown),
        "html" | "htm" | "xhtml" | "php" | "shtml" | "vue" | "svelte" | "astro" | "njk"
        | "liquid" => Some(SrcKind::Html),
        "xml" => {
            if head.contains("wp:wxr_version") {
                Some(SrcKind::Wxr)
            } else if head.contains("<urlset") || head.contains("<sitemapindex") {
                Some(SrcKind::Sitemap)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn attr_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r#"(?is)\b(href|src|action|poster|data-href|data-src)\s*=\s*(?:"([^"]*)"|'([^']*)')"#,
        )
        .unwrap()
    })
}
fn md_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"!?\[([^\]\n]*)\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)"#).unwrap()
    })
}
fn mdref_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"(?m)^[ ]{0,3}\[[^\]\n]+\]:[ \t]*<?(\S+?)>?(?:[ \t]+["'(].*)?$"#).unwrap()
    })
}
fn auto_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"<((?:https?|ftp)://[^>\s]+)>"#).unwrap())
}
fn loc_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"(?is)<loc>\s*(?:<!\[CDATA\[)?\s*(.*?)\s*(?:\]\]>)?\s*</loc>"#).unwrap()
    })
}
fn menu_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"(?s)_menu_item_url\]\]></wp:meta_key>\s*<wp:meta_value><!\[CDATA\[(.*?)\]\]></wp:meta_value>"#)
            .unwrap()
    })
}
fn title_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?s)<title>(?:<!\[CDATA\[)?(.*?)(?:\]\]>)?</title>"#).unwrap())
}
fn anchor_text_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?s)^[^>]*>(.*?)</a>"#).unwrap())
}
fn tag_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

/// Valori care nu sunt linkuri verificabile (ancore, mailto, JS, șabloane).
pub fn skip_value(v: &str) -> bool {
    let t = v.trim();
    if t.is_empty() || t.starts_with('#') {
        return true;
    }
    let l = t.to_ascii_lowercase();
    l.starts_with("mailto:")
        || l.starts_with("tel:")
        || l.starts_with("sms:")
        || l.starts_with("javascript:")
        || l.starts_with("data:")
        || l.starts_with("blob:")
        || l.contains("{{")
        || l.contains("{%")
        || l.contains("<?")
        || l.contains("${")
        // fragmente de cod (PHP/JS/șabloane), nu URL-uri
        || t.contains(['"', '$', '\\', '`', '<', '>', '{', '}'])
        || t.contains("'+")
        || t.contains("+'")
        || t.starts_with('\'')
        || t.ends_with('\'')
}

fn html_mask_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?is)<!--.*?-->|<script\b.*?</script\s*>|<style\b.*?</style\s*>|<pre\b.*?</pre\s*>|<code\b.*?</code\s*>|<textarea\b.*?</textarea\s*>|<\?(?:php|=)?.*?\?>").unwrap()
    })
}
fn md_inline_code_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"``[^\n]+?``|`[^`\n]+`").unwrap())
}

/// Zone în care linkurile NU sunt reale: comentarii, script/style, pre/code, PHP,
/// blocuri de cod Markdown (``` / ~~~) și cod inline. Evităm să „reparăm” exemple din documentație.
pub fn masked_ranges(text: &str, kind: SrcKind) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = vec![];
    for m in html_mask_re().find_iter(text) {
        v.push((m.start(), m.end()));
    }
    if kind == SrcKind::Markdown {
        let mut off = 0usize;
        let mut open: Option<(usize, char, usize)> = None;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_start_matches(' ');
            let indent = line.len() - trimmed.len();
            if indent <= 3
                && let Some(c) = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')
            {
                let n = trimmed.chars().take_while(|x| *x == c).count();
                if n >= 3 {
                    match open {
                        None => open = Some((off, c, n)),
                        Some((s0, oc, on))
                            if oc == c && n >= on && trimmed[n..].trim().is_empty() =>
                        {
                            v.push((s0, off + line.len()));
                            open = None;
                        }
                        _ => {}
                    }
                }
            }
            off += line.len();
        }
        if let Some((s0, _, _)) = open {
            v.push((s0, text.len()));
        }
        for m in md_inline_code_re().find_iter(text) {
            v.push((m.start(), m.end()));
        }
    }
    v.sort();
    v
}

fn in_ranges(r: &[(usize, usize)], pos: usize) -> bool {
    let i = r.partition_point(|(s, _)| *s <= pos);
    r[..i].iter().any(|(s, e)| *s <= pos && pos < *e)
}

fn decode_amp(s: &str) -> (String, bool) {
    if s.contains("&amp;") || s.contains("&#038;") || s.contains("&#38;") {
        (
            s.replace("&amp;", "&")
                .replace("&#038;", "&")
                .replace("&#38;", "&"),
            true,
        )
    } else {
        (s.to_string(), false)
    }
}

/// Codifică înapoi o valoare înlocuitoare, în stilul valorii originale.
pub fn encode_like(new: &str, amp: bool) -> String {
    if amp {
        new.replace('&', "&amp;")
    } else {
        new.to_string()
    }
}

fn plain_text(s: &str) -> String {
    let t = tag_re().replace_all(s, "");
    t.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect()
}

fn scan_html_like(
    text: &str,
    file: &Path,
    src: SrcKind,
    page: &dyn Fn(usize) -> String,
    out: &mut Vec<Occurrence>,
) {
    for c in attr_re().captures_iter(text) {
        let g = c.get(2).or_else(|| c.get(3)).unwrap();
        let raw = g.as_str();
        if skip_value(raw) {
            continue;
        }
        let attr = c.get(1).unwrap().as_str().to_ascii_lowercase();
        let (url, amp) = decode_amp(raw.trim());
        // textul ancorei: de după `>` al tagului curent până la `</a>`
        let mut atext = String::new();
        if attr == "href" {
            let tail = &text[g.end()..text.len().min(g.end() + 600)];
            if let Some(m) = anchor_text_re().captures(tail) {
                atext = plain_text(m.get(1).unwrap().as_str());
            }
        }
        out.push(Occurrence {
            file: file.to_path_buf(),
            start: g.start(),
            end: g.end(),
            url,
            amp,
            src,
            attr,
            text: atext,
            nav: false,
            page: page(g.start()),
        });
    }
}

/// Scanează conținutul unui fișier. `text` trebuie să fie UTF-8.
pub fn scan_text(text: &str, file: &Path, kind: SrcKind) -> Vec<Occurrence> {
    let mut out = vec![];
    let fname = file.display().to_string();
    match kind {
        SrcKind::Html => {
            scan_html_like(text, file, kind, &|_| fname.clone(), &mut out);
        }
        SrcKind::Markdown => {
            for c in md_re().captures_iter(text) {
                let g = c.get(2).unwrap();
                if skip_value(g.as_str()) {
                    continue;
                }
                out.push(Occurrence {
                    file: file.to_path_buf(),
                    start: g.start(),
                    end: g.end(),
                    url: g.as_str().to_string(),
                    amp: false,
                    src: kind,
                    attr: "md".into(),
                    text: c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    nav: false,
                    page: fname.clone(),
                });
            }
            for c in mdref_re().captures_iter(text) {
                let g = c.get(1).unwrap();
                if skip_value(g.as_str()) {
                    continue;
                }
                out.push(Occurrence {
                    file: file.to_path_buf(),
                    start: g.start(),
                    end: g.end(),
                    url: g.as_str().to_string(),
                    amp: false,
                    src: kind,
                    attr: "mdref".into(),
                    text: String::new(),
                    nav: false,
                    page: fname.clone(),
                });
            }
            for c in auto_re().captures_iter(text) {
                let g = c.get(1).unwrap();
                out.push(Occurrence {
                    file: file.to_path_buf(),
                    start: g.start(),
                    end: g.end(),
                    url: g.as_str().to_string(),
                    amp: false,
                    src: kind,
                    attr: "auto".into(),
                    text: String::new(),
                    nav: false,
                    page: fname.clone(),
                });
            }
            // HTML încorporat în Markdown
            let mut inner = vec![];
            scan_html_like(text, file, kind, &|_| fname.clone(), &mut inner);
            out.extend(inner);
        }
        SrcKind::Wxr => {
            let titles: Vec<(usize, String)> = title_re()
                .captures_iter(text)
                .map(|c| {
                    (
                        c.get(0).unwrap().start(),
                        plain_text(c.get(1).unwrap().as_str()),
                    )
                })
                .collect();
            let page_of = |pos: usize| -> String {
                let i = titles.partition_point(|(p, _)| *p <= pos);
                let t = if i > 0 { titles[i - 1].1.as_str() } else { "" };
                if t.is_empty() {
                    fname.clone()
                } else {
                    format!("{fname} › {t}")
                }
            };
            scan_html_like(text, file, kind, &page_of, &mut out);
            for c in menu_re().captures_iter(text) {
                let g = c.get(1).unwrap();
                if skip_value(g.as_str()) {
                    continue;
                }
                out.push(Occurrence {
                    file: file.to_path_buf(),
                    start: g.start(),
                    end: g.end(),
                    url: g.as_str().trim().to_string(),
                    amp: false,
                    src: kind,
                    attr: "menu".into(),
                    text: String::new(),
                    nav: true,
                    page: format!("{fname} › meniu"),
                });
            }
        }
        SrcKind::Sitemap => {
            for c in loc_re().captures_iter(text) {
                let g = c.get(1).unwrap();
                let (url, amp) = decode_amp(g.as_str());
                out.push(Occurrence {
                    file: file.to_path_buf(),
                    start: g.start(),
                    end: g.end(),
                    url,
                    amp,
                    src: kind,
                    attr: "loc".into(),
                    text: String::new(),
                    nav: false,
                    page: fname.clone(),
                });
            }
        }
    }
    out.sort_by_key(|o| o.start);
    out.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    let mask = masked_ranges(text, kind);
    if !mask.is_empty() {
        out.retain(|o| !in_ranges(&mask, o.start));
    }
    out
}

/// Parcurge fișiere/directoare și returnează toate aparițiile.
pub fn scan_paths(paths: &[PathBuf]) -> anyhow::Result<Vec<Occurrence>> {
    let mut out = vec![];
    for p in paths {
        let files: Vec<PathBuf> = if p.is_dir() {
            walkdir::WalkDir::new(p)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| {
                    !e.path().components().any(|c| {
                        matches!(
                            c.as_os_str().to_str(),
                            Some(".veriga-backup" | ".git" | "node_modules")
                        )
                    })
                })
                .map(|e| e.into_path())
                .collect()
        } else {
            vec![p.clone()]
        };
        for f in files {
            // fișierele uriașe sau ilizibile se sar (protecție de memorie)
            if std::fs::metadata(&f)
                .map(|m| m.len() > 50 * 1024 * 1024)
                .unwrap_or(true)
            {
                continue;
            }
            let Ok(bytes) = std::fs::read(&f) else {
                continue;
            };
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            let head: String = text.chars().take(4000).collect();
            if let Some(k) = detect(&f, &head) {
                out.extend(scan_text(&text, &f, k));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_spans() {
        let t = r#"<p><a href="http://a.ro/x?a=1&amp;b=2">Bun</a> <a href='#top'>sus</a> <img src="i.png"></p>"#;
        let o = scan_text(t, Path::new("a.html"), SrcKind::Html);
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].url, "http://a.ro/x?a=1&b=2");
        assert!(o[0].amp);
        assert_eq!(&t[o[0].start..o[0].end], "http://a.ro/x?a=1&amp;b=2");
        assert_eq!(o[0].text, "Bun");
    }

    #[test]
    fn markdown() {
        let t = "Vezi [aici](https://a.ro/p \"t\") și <https://b.ro/>.\n\n[r]: https://c.ro/z\n";
        let o = scan_text(t, Path::new("a.md"), SrcKind::Markdown);
        let urls: Vec<_> = o.iter().map(|x| x.url.as_str()).collect();
        assert_eq!(
            urls,
            vec!["https://a.ro/p", "https://b.ro/", "https://c.ro/z"]
        );
        for x in &o {
            assert_eq!(&t[x.start..x.end], x.url);
        }
    }

    #[test]
    fn wxr_menu_and_pages() {
        let t = r#"<rss><wp:wxr_version>1.2</wp:wxr_version><item><title>Despre</title><content:encoded><![CDATA[<a href="http://x.ro/a">a</a>]]></content:encoded></item>
<item><title>Meniu</title><wp:postmeta><wp:meta_key><![CDATA[_menu_item_url]]></wp:meta_key><wp:meta_value><![CDATA[http://x.ro/m]]></wp:meta_value></wp:postmeta></item></rss>"#;
        let o = scan_text(t, Path::new("e.xml"), SrcKind::Wxr);
        assert_eq!(o.len(), 2);
        assert!(o[0].page.ends_with("Despre"));
        assert!(o[1].nav && o[1].url == "http://x.ro/m");
    }

    #[test]
    fn code_and_comments_are_ignored() {
        let h = r#"<!-- <a href="htp://x.ro/c">c</a> --><pre><a href="htp://x.ro/p">p</a></pre><script>var a='<a href="http://x.ro/s">';</script><p><a href="htp://x.ro/real">r</a></p><?php echo '<a href="'.$u.'">'; ?>"#;
        let o = scan_text(h, Path::new("a.html"), SrcKind::Html);
        assert_eq!(
            o.len(),
            1,
            "{:?}",
            o.iter().map(|x| &x.url).collect::<Vec<_>>()
        );
        assert_eq!(o[0].url, "htp://x.ro/real");
        let m = "```\n[x](htp://a.ro/f)\n```\n\nIar `[y](htp://a.ro/i)` și [z](htp://a.ro/real)\n~~~md\n[w](htp://a.ro/t)\n";
        let o = scan_text(m, Path::new("a.md"), SrcKind::Markdown);
        assert_eq!(
            o.len(),
            1,
            "{:?}",
            o.iter().map(|x| &x.url).collect::<Vec<_>>()
        );
        assert_eq!(o[0].url, "htp://a.ro/real");
    }

    #[test]
    fn code_fragments_are_not_urls() {
        for v in ["\".$u.\"", "$url", "{{ url }}", "'+x+'", "a\\b", "`x`"] {
            assert!(skip_value(v), "{v}");
        }
        assert!(!skip_value("https://a.ro/O'Brien"));
    }

    #[test]
    fn sitemap_loc() {
        let t = "<urlset><url><loc> https://a.ro/?a=1&amp;b=2 </loc></url></urlset>";
        let o = scan_text(t, Path::new("s.xml"), SrcKind::Sitemap);
        assert_eq!(o[0].url, "https://a.ro/?a=1&b=2");
    }
}
