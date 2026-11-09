//! Conținut mixt: subresurse `http:` încărcate de pagini `https:`.

use crate::extract::{RawLink, Sub};
use serde::{Deserialize, Serialize};
use url::{Host, Url};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MixedCategory {
    /// Browserele o blochează (script, stil, iframe, font…).
    Blockable,
    /// Imagini, audio, video: de regulă doar avertisment.
    OptionallyBlockable,
}

impl MixedCategory {
    pub fn label(self, lang: &str) -> &'static str {
        match (self, lang == "en") {
            (Self::Blockable, false) => "blocabil",
            (Self::Blockable, true) => "blockable",
            (Self::OptionallyBlockable, false) => "opțional blocabil",
            (Self::OptionallyBlockable, true) => "optionally blockable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MixedFinding {
    pub page: String,
    pub target: String,
    pub category: MixedCategory,
    /// Observație: «ar fi înnoită» (upgrade-insecure-requests) sau «redirecționează la http».
    pub note: Option<String>,
}

/// Gazdă «potențial de încredere» (Secure Contexts): localhost, 127.0.0.0/8, ::1.
pub fn potentially_trustworthy(u: &Url) -> bool {
    match u.host() {
        Some(Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            d == "localhost" || d.ends_with(".localhost")
        }
        Some(Host::Ipv4(a)) => a.octets()[0] == 127,
        Some(Host::Ipv6(a)) => a.is_loopback(),
        None => false,
    }
}

pub fn category(sub: Sub) -> Option<MixedCategory> {
    match sub {
        Sub::None => None,
        Sub::Optional => Some(MixedCategory::OptionallyBlockable),
        Sub::Blockable => Some(MixedCategory::Blockable),
    }
}

/// Constatările unei pagini `https:`. Pentru pagini non-https lista este goală.
pub fn findings(page: &Url, links: &[RawLink], upgrade_insecure: bool) -> Vec<MixedFinding> {
    if page.scheme() != "https" {
        return vec![];
    }
    let mut out: Vec<MixedFinding> = vec![];
    for l in links {
        let Some(cat) = category(l.sub) else { continue };
        if l.url.scheme() != "http" || potentially_trustworthy(&l.url) {
            continue;
        }
        let mut target = l.url.clone();
        target.set_fragment(None);
        out.push(MixedFinding {
            page: page.to_string(),
            target: target.to_string(),
            category: cat,
            note: upgrade_insecure
                .then(|| "ar fi înnoită de upgrade-insecure-requests".to_string()),
        });
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{extract_css, extract_html};

    fn run(page: &str, html: &str) -> Vec<MixedFinding> {
        let p = Url::parse(page).unwrap();
        findings(&p, &extract_html(&p, html, false).links, false)
    }

    #[test]
    fn t1_image_is_optionally_blockable() {
        let f = run("https://ex.org/p", r#"<img src="http://cdn.ex/a.png">"#);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].category, MixedCategory::OptionallyBlockable);
    }

    #[test]
    fn t2_script_is_blockable() {
        let f = run(
            "https://ex.org/p",
            r#"<script src="http://cdn.ex/a.js"></script>"#,
        );
        assert_eq!(f[0].category, MixedCategory::Blockable);
        let f = run(
            "https://ex.org/p",
            r#"<link rel="stylesheet" href="http://c.ex/s.css"><iframe src="http://c.ex/f"></iframe><link rel="preload" as="image" href="http://c.ex/i.png">"#,
        );
        assert_eq!(f.len(), 3);
        assert!(f.iter().all(|x| x.category == MixedCategory::Blockable));
    }

    #[test]
    fn t3_navigation_is_not_mixed() {
        assert!(
            run(
                "https://ex.org/p",
                r#"<a href="http://alt.ex/">x</a><area href="http://a.ex/">"#
            )
            .is_empty()
        );
        // referințe informative, nu subresurse
        assert!(run(
            "https://ex.org/p",
            r#"<link rel="canonical" href="http://ex.org/p"><meta property="og:image" content="http://c.ex/i.png">"#
        )
        .is_empty());
    }

    #[test]
    fn t4_trustworthy_hosts_skipped() {
        assert!(run("https://ex.org/p", r#"<img src="http://localhost:8080/a.png"><img src="http://127.0.0.5/a.png"><img src="http://[::1]/a.png"><img src="http://app.localhost/a.png">"#).is_empty());
    }

    #[test]
    fn t5_http_page_never_mixed() {
        assert!(
            run(
                "http://ex.org/p",
                r#"<script src="http://cdn.ex/a.js"></script>"#
            )
            .is_empty()
        );
    }

    #[test]
    fn t6_css_import_is_blockable() {
        let p = Url::parse("https://ex.org/s.css").unwrap();
        let l = extract_css(
            &p,
            r#"@import url("http://f.ex/s.css"); .x{background:url(http://f.ex/b.png)} @font-face{src:url(http://f.ex/f.woff2)}"#,
        );
        let f = findings(&p, &l, false);
        let by = |s: &str| f.iter().find(|x| x.target.ends_with(s)).unwrap().category;
        assert_eq!(by("/s.css"), MixedCategory::Blockable);
        assert_eq!(by("/b.png"), MixedCategory::OptionallyBlockable);
        assert_eq!(by("/f.woff2"), MixedCategory::Blockable);
    }

    #[test]
    fn t7_srcset_one_result() {
        let f = run(
            "https://ex.org/p",
            r#"<img srcset="http://c.ex/a.png 1x, https://c.ex/b.png 2x">"#,
        );
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn relative_and_protocol_relative_inherit_scheme() {
        assert!(
            run(
                "https://ex.org/p",
                r#"<img src="/a.png"><img src="//c.ex/a.png">"#
            )
            .is_empty()
        );
    }

    #[test]
    fn upgrade_note_added_not_cancelled() {
        let p = Url::parse("https://ex.org/p").unwrap();
        let l = extract_html(&p, r#"<img src="http://c.ex/a.png">"#, false).links;
        let f = findings(&p, &l, true);
        assert_eq!(f.len(), 1);
        assert!(f[0].note.is_some());
    }
}
