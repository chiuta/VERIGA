//! Legături canonice (RFC 6596) și etichete de limbă alternativă (BCP 47, RFC 5646).

use crate::model::Status;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeoKind {
    CanonicalBroken,
    CanonicalRedirected,
    CanonicalMultiple,
    HreflangInvalid,
    /// `x-default`: convenție externă, nu etichetă RFC 5646 (informare).
    HreflangConvention,
}

impl SeoKind {
    pub fn label(self, lang: &str) -> &'static str {
        let en = lang == "en";
        match (self, en) {
            (Self::CanonicalBroken, false) => "canonică ruptă",
            (Self::CanonicalBroken, true) => "broken canonical",
            (Self::CanonicalRedirected, false) => "canonică redirecționată",
            (Self::CanonicalRedirected, true) => "redirected canonical",
            (Self::CanonicalMultiple, false) => "canonice multiple",
            (Self::CanonicalMultiple, true) => "multiple canonicals",
            (Self::HreflangInvalid, false) => "hreflang invalid",
            (Self::HreflangInvalid, true) => "invalid hreflang",
            (Self::HreflangConvention, false) => "hreflang: convenție externă",
            (Self::HreflangConvention, true) => "hreflang: external convention",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SeoFinding {
    pub page: String,
    pub kind: SeoKind,
    /// Adresa canonică sau eticheta de limbă în cauză.
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LangTag {
    Valid,
    /// `x-default`.
    Convention,
    Invalid,
}

fn alpha(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic())
}
fn alnum(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())
}
fn digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Validează sintactic o etichetă de limbă după gramatica RFC 5646 §2.1 (fără registru IANA).
pub fn check_language_tag(tag: &str) -> LangTag {
    if tag.eq_ignore_ascii_case("x-default") {
        return LangTag::Convention;
    }
    if tag.is_empty() {
        return LangTag::Invalid;
    }
    let parts: Vec<&str> = tag.split('-').collect();
    let n = parts.len();
    // utilizare privată în întregime: x-…
    if parts[0].eq_ignore_ascii_case("x") {
        return if n >= 2 && parts[1..].iter().all(|p| alnum(p) && p.len() <= 8) {
            LangTag::Valid
        } else {
            LangTag::Invalid
        };
    }
    let mut i = 0;
    // limbă
    let lang = parts[0];
    if !alpha(lang) || !(2..=8).contains(&lang.len()) {
        return LangTag::Invalid;
    }
    i += 1;
    if (2..=3).contains(&lang.len()) {
        // extlang: până la 3 subetichete de 3 litere
        let mut k = 0;
        while i < n && k < 3 && parts[i].len() == 3 && alpha(parts[i]) {
            i += 1;
            k += 1;
        }
    }
    // script
    if i < n && parts[i].len() == 4 && alpha(parts[i]) {
        i += 1;
    }
    // regiune
    if i < n
        && ((parts[i].len() == 2 && alpha(parts[i])) || (parts[i].len() == 3 && digits(parts[i])))
    {
        i += 1;
    }
    // variante
    while i < n {
        let p = parts[i];
        let ok = (alnum(p) && (5..=8).contains(&p.len()))
            || (p.len() == 4 && p.as_bytes()[0].is_ascii_digit() && alnum(p));
        if !ok {
            break;
        }
        i += 1;
    }
    // extensii
    while i < n {
        let p = parts[i];
        if p.len() == 1 && alnum(p) && !p.eq_ignore_ascii_case("x") {
            i += 1;
            let start = i;
            while i < n && alnum(parts[i]) && (2..=8).contains(&parts[i].len()) {
                i += 1;
            }
            if i == start {
                return LangTag::Invalid;
            }
        } else {
            break;
        }
    }
    // utilizare privată
    if i < n && parts[i].eq_ignore_ascii_case("x") {
        i += 1;
        let start = i;
        while i < n && alnum(parts[i]) && parts[i].len() <= 8 {
            i += 1;
        }
        if i == start {
            return LangTag::Invalid;
        }
    }
    if i == n {
        LangTag::Valid
    } else {
        LangTag::Invalid
    }
}

/// Constatările ce țin doar de pagină: canonice multiple și etichete hreflang.
pub fn page_findings(page: &str, canonicals: &[Url], hreflangs: &[String]) -> Vec<SeoFinding> {
    let mut out = vec![];
    let mut distinct: Vec<String> = vec![];
    for c in canonicals {
        let k = crate::urlutil::normalize(c);
        if !distinct.contains(&k) {
            distinct.push(k);
        }
    }
    if distinct.len() > 1 {
        out.push(SeoFinding {
            page: page.to_string(),
            kind: SeoKind::CanonicalMultiple,
            value: distinct.join(" | "),
        });
    }
    let mut seen: Vec<&str> = vec![];
    for h in hreflangs {
        let h = h.trim();
        if seen.contains(&h) {
            continue;
        }
        seen.push(h);
        match check_language_tag(h) {
            LangTag::Valid => {}
            LangTag::Convention => out.push(SeoFinding {
                page: page.to_string(),
                kind: SeoKind::HreflangConvention,
                value: h.to_string(),
            }),
            LangTag::Invalid => out.push(SeoFinding {
                page: page.to_string(),
                kind: SeoKind::HreflangInvalid,
                value: h.to_string(),
            }),
        }
    }
    out
}

/// Constatarea pentru o țintă canonică, în funcție de starea ei; `None` dacă e în regulă.
pub fn canonical_target_finding(
    page: &str,
    target: &str,
    status: Status,
    cfg: &crate::Config,
) -> Option<SeoFinding> {
    let kind = if status == Status::Redirected {
        SeoKind::CanonicalRedirected
    } else if status.is_broken(cfg) {
        SeoKind::CanonicalBroken
    } else {
        return None;
    };
    Some(SeoFinding {
        page: page.to_string(),
        kind,
        value: target.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(t: &str) -> bool {
        check_language_tag(t) == LangTag::Valid
    }

    #[test]
    fn t4_valid_tags() {
        assert!(valid("zh-Hant-TW"));
        assert!(valid("de-DE-1996"));
        assert!(valid("en"));
        assert!(valid("EN-us"));
        assert!(valid("zh-yue"));
        assert!(valid("en-a-bbb-x-a-ccc"));
        assert!(valid("x-private"));
        assert!(valid("sl-rozaj-biske"));
        assert!(valid("es-419"));
    }

    #[test]
    fn t5_invalid_tags() {
        assert!(!valid("en_US"));
        assert!(!valid("e"));
        assert!(!valid(""));
        assert!(!valid("en-"));
        assert!(!valid("en--US"));
        assert!(!valid("en-US-"));
        assert!(!valid("en-x"));
        assert!(!valid("123"));
        assert!(!valid("en US"));
    }

    #[test]
    fn t6_grammatical_but_unregistered() {
        assert!(valid("english"));
    }

    #[test]
    fn x_default_is_convention_not_invalid() {
        assert_eq!(check_language_tag("x-default"), LangTag::Convention);
        let f = page_findings("p", &[], &["x-default".into()]);
        assert_eq!(f[0].kind, SeoKind::HreflangConvention);
    }

    #[test]
    fn t7_multiple_canonicals() {
        let a = Url::parse("https://ex.org/a").unwrap();
        let b = Url::parse("https://ex.org/b").unwrap();
        let f = page_findings("p", &[a.clone(), b], &[]);
        assert_eq!(f[0].kind, SeoKind::CanonicalMultiple);
        // aceeași adresă repetată nu e «multiplă»
        assert!(page_findings("p", &[a.clone(), a], &[]).is_empty());
    }

    #[test]
    fn page_findings_flag_bad_hreflang() {
        let f = page_findings(
            "p",
            &[],
            &["en_US".into(), "ro".into(), "".into(), "en_US".into()],
        );
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|x| x.kind == SeoKind::HreflangInvalid));
    }

    #[test]
    fn t2_t3_target_status() {
        let cfg = crate::Config::default();
        assert!(canonical_target_finding("p", "t", Status::Ok, &cfg).is_none());
        assert_eq!(
            canonical_target_finding("p", "t", Status::NotFound, &cfg)
                .unwrap()
                .kind,
            SeoKind::CanonicalBroken
        );
        assert_eq!(
            canonical_target_finding("p", "t", Status::Redirected, &cfg)
                .unwrap()
                .kind,
            SeoKind::CanonicalRedirected
        );
    }
}
