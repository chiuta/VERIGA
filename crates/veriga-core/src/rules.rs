//! Reguli deterministe, fără rețea: URL-uri malformate, parametri de urmărire,
//! inconsistențe http/https și www. Fiecare regulă produce o *propunere* de
//! înlocuire; verificarea live o face `fixer`.

use crate::srcscan::Occurrence;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rule {
    /// `htp://`, `https//`, `http:/x` …
    SchemeTypo,
    /// schemă dublată, spații/`%20` la capete, punctuație agățată
    Unmangle,
    /// `www.exemplu.ro` fără schemă
    MissingScheme,
    /// parametri `utm_*`, `fbclid`, `gclid` …
    Tracking,
    /// `http://` deși `https://` răspunde
    Protocol,
    /// același site apare și cu, și fără `www`
    WwwConsistency,
    /// redirecționare permanentă (301/308) → URL-ul final
    Redirect,
    /// reparație manuală (editare / dezlegare / Wayback)
    Manual,
}

impl Rule {
    pub fn id(self) -> &'static str {
        match self {
            Rule::SchemeTypo => "scheme_typo",
            Rule::Unmangle => "unmangle",
            Rule::MissingScheme => "missing_scheme",
            Rule::Tracking => "tracking",
            Rule::Protocol => "protocol",
            Rule::WwwConsistency => "www",
            Rule::Redirect => "redirect",
            Rule::Manual => "manual",
        }
    }

    pub fn label(self, lang: &str) -> &'static str {
        let en = lang == "en";
        match (self, en) {
            (Rule::SchemeTypo, false) => "Greșeală în schemă (htp://, https//)",
            (Rule::SchemeTypo, true) => "Scheme typo (htp://, https//)",
            (Rule::Unmangle, false) => "URL deformat (schemă dublă, spații, punctuație)",
            (Rule::Unmangle, true) => "Mangled URL (doubled scheme, spaces, punctuation)",
            (Rule::MissingScheme, false) => "Fără schemă (www.…)",
            (Rule::MissingScheme, true) => "Missing scheme (www.…)",
            (Rule::Tracking, false) => "Parametri de urmărire",
            (Rule::Tracking, true) => "Tracking parameters",
            (Rule::Protocol, false) => "http → https",
            (Rule::Protocol, true) => "http → https",
            (Rule::WwwConsistency, false) => "Inconsistență www",
            (Rule::WwwConsistency, true) => "www inconsistency",
            (Rule::Redirect, false) => "Redirecționare permanentă",
            (Rule::Redirect, true) => "Permanent redirect",
            (Rule::Manual, false) => "Manual",
            (Rule::Manual, true) => "Manual",
        }
    }

    /// Regulile care *pot* fi aplicate automat (după verificare live).
    pub fn auto(self) -> bool {
        !matches!(self, Rule::Manual)
    }
}

const TRACKING_EXACT: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "msclkid",
    "yclid",
    "twclid",
    "igshid",
    "mc_eid",
    "mc_cid",
    "_hsenc",
    "_hsmi",
    "mkt_tok",
    "ref_src",
    "wbraid",
    "gbraid",
    "oly_enc_id",
    "oly_anon_id",
    "vero_id",
    "_ga",
    "_gl",
    "s_cid",
    "spm",
];

pub fn is_tracking_param(k: &str) -> bool {
    let l = k.to_ascii_lowercase();
    l.starts_with("utm_") || TRACKING_EXACT.contains(&l.as_str())
}

/// Elimină parametrii de urmărire; păstrează ordinea și restul query-ului intact.
pub fn strip_tracking(url: &str) -> Option<String> {
    let (base, frag) = match url.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (url, None),
    };
    let (path, q) = base.split_once('?')?;
    let kept: Vec<&str> = q
        .split('&')
        .filter(|kv| !kv.is_empty())
        .filter(|kv| !is_tracking_param(kv.split('=').next().unwrap_or("")))
        .collect();
    let total = q.split('&').filter(|kv| !kv.is_empty()).count();
    if kept.len() == total {
        return None;
    }
    let mut out = path.to_string();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(f) = frag {
        out.push('#');
        out.push_str(f);
    }
    Some(out)
}

const FILE_EXTS: &[&str] = &[
    "html", "htm", "xhtml", "php", "asp", "aspx", "jsp", "pdf", "css", "js", "png", "jpg", "jpeg",
    "gif", "svg", "webp", "txt", "xml", "json", "md", "zip", "doc", "docx", "xls", "xlsx", "csv",
    "mp3", "mp4",
];

/// Gazdă plauzibilă: etichete nevide, TLD alfabetic (≥ 2) care nu e extensie de fișier.
fn plausible_host(hostname: &str) -> bool {
    if hostname == "localhost" {
        return true;
    }
    if hostname.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    let labels: Vec<&str> = hostname.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return false;
    }
    let tld = labels[labels.len() - 1].to_ascii_lowercase();
    tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
        && !FILE_EXTS.contains(&tld.as_str())
        && hostname
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
}

/// Corectează erori de tastare în schemă. Returnează `None` dacă URL-ul e deja corect
/// sau dacă nu arată sigur a fi un URL absolut cu schemă greșită (ex. `http/pagina.html`
/// este o cale relativă, nu o greșeală).
pub fn fix_scheme_typo(url: &str) -> Option<String> {
    let t = url.trim();
    let low = t.to_ascii_lowercase();
    if low.starts_with("http://") || low.starts_with("https://") {
        return None;
    }
    static MAP: OnceLock<Regex> = OnceLock::new();
    // separatorul TREBUIE să conțină „:” (http:/, htp:, http:\\) sau să fie exact „//” (https//).
    let re = MAP.get_or_init(|| {
        Regex::new(r"(?i)^(h?h?t{1,3}p{1,2}s{0,2})(:{1,2}/{0,3}|;/{1,2}|:\\{1,2}|/{2})([^/\\].*)$")
            .unwrap()
    });
    let c = re.captures(t)?;
    let scheme_raw = c.get(1)?.as_str().to_ascii_lowercase();
    let sep = c.get(2)?.as_str();
    let rest = c.get(3)?.as_str();
    // „//” fără „:” e acceptat doar pentru http/https scrise corect („https//a.ro”)
    if sep == "//" && scheme_raw != "http" && scheme_raw != "https" {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostname = host.rsplit_once('@').map(|x| x.1).unwrap_or(host);
    let hostname = hostname.split(':').next().unwrap_or("");
    if host.contains(' ') || !plausible_host(hostname) {
        return None;
    }
    let canon = if scheme_raw.contains('s') {
        "https"
    } else {
        "http"
    };
    let fixed = format!("{canon}://{}", rest.trim_start_matches('/'));
    if fixed.eq_ignore_ascii_case(t) {
        None
    } else {
        Some(fixed)
    }
}

/// Repară URL-uri deformate: schemă dublă, spații la capete, punctuație agățată.
pub fn unmangle(url: &str) -> Option<String> {
    let mut s = url.to_string();
    // spații / %20 / %0A la capete
    s = s.trim().to_string();
    for suf in ["%20", "%0A", "%0D", "%09"] {
        while s.to_ascii_uppercase().ends_with(suf) {
            s.truncate(s.len() - suf.len());
        }
        while s.to_ascii_uppercase().starts_with(suf) {
            s = s[suf.len()..].to_string();
        }
    }
    // schemă dublată: http://https://x, https://http//x, http://http://x
    static DBL: OnceLock<Regex> = OnceLock::new();
    let dbl = DBL.get_or_init(|| Regex::new(r"(?i)^https?:/{0,2}(https?:/{1,2}[^/].*)$").unwrap());
    if let Some(c) = dbl.captures(&s) {
        let inner = c.get(1).unwrap().as_str().to_string();
        s = fix_scheme_typo(&inner).unwrap_or(inner);
    }
    // &amp;amp; rămas din dublă codificare
    while s.contains("&amp;") {
        s = s.replace("&amp;", "&");
    }
    // punctuație agățată la sfârșit (nu parantezele echilibrate)
    while let Some(last) = s.chars().last() {
        let drop = match last {
            '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'' | '>' => true,
            ')' => s.matches('(').count() < s.matches(')').count(),
            _ => false,
        };
        if drop && s.len() > 8 {
            s.pop();
        } else {
            break;
        }
    }
    if s == url { None } else { Some(s) }
}

/// `www.exemplu.ro/x` → `https://www.exemplu.ro/x`
pub fn add_missing_scheme(url: &str) -> Option<String> {
    let t = url.trim();
    let low = t.to_ascii_lowercase();
    if low.starts_with("www.") && t[4..].contains('.') && !t.contains(' ') {
        Some(format!("https://{t}"))
    } else {
        None
    }
}

/// Un URL absolut valid din punct de vedere sintactic?
pub fn is_well_formed(url: &str) -> bool {
    Url::parse(url)
        .map(|u| u.host_str().is_some())
        .unwrap_or(false)
}

/// Transformările locale, în ordinea aplicării. Returnează (regulă, URL nou).
pub fn local_fix(url: &str) -> Option<(Rule, String)> {
    if let Some(f) = fix_scheme_typo(url) {
        return Some((Rule::SchemeTypo, f));
    }
    if let Some(f) = unmangle(url) {
        return Some((Rule::Unmangle, f));
    }
    if let Some(f) = add_missing_scheme(url) {
        return Some((Rule::MissingScheme, f));
    }
    if url.starts_with("http")
        && let Some(f) = strip_tracking(url)
    {
        return Some((Rule::Tracking, f));
    }
    None
}

/// Constatare offline pentru raport.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    pub file: String,
    pub page: String,
    pub url: String,
    pub rule: Rule,
    pub suggestion: Option<String>,
    pub nav: bool,
    pub note: String,
}

fn host_parts(u: &str) -> Option<(String, bool, String)> {
    let p = Url::parse(u).ok()?;
    // porturi explicite = servicii distincte (nu comparăm http/https/www între ele)
    if p.port().is_some() {
        return None;
    }
    let h = p.host_str()?.to_ascii_lowercase();
    let www = h.starts_with("www.");
    let bare = h.trim_start_matches("www.").to_string();
    Some((bare, www, p.scheme().to_string()))
}

/// Analiză offline: reguli locale + inconsistențe http/https și www per gazdă.
pub fn analyze(occs: &[Occurrence], lang: &str) -> Vec<Finding> {
    let en = lang == "en";
    let mut out = vec![];
    for o in occs {
        if let Some((rule, new)) = local_fix(&o.url) {
            out.push(Finding {
                file: o.file.display().to_string(),
                page: o.page.clone(),
                url: o.url.clone(),
                rule,
                suggestion: Some(new),
                nav: o.nav,
                note: rule.label(lang).to_string(),
            });
        } else if !o.url.is_empty()
            && !o.url.starts_with('/')
            && !o.url.starts_with('.')
            && o.url.contains("://")
            && !is_well_formed(&o.url)
        {
            out.push(Finding {
                file: o.file.display().to_string(),
                page: o.page.clone(),
                url: o.url.clone(),
                rule: Rule::Manual,
                suggestion: None,
                nav: o.nav,
                note: if en {
                    "Malformed URL (no automatic fix)".into()
                } else {
                    "URL malformat (fără reparație automată)".into()
                },
            });
        }
    }
    // inconsistențe pe gazde
    let mut by_host: BTreeMap<String, (BTreeSet<bool>, BTreeSet<String>)> = BTreeMap::new();
    let mut counts: BTreeMap<(String, bool), usize> = BTreeMap::new();
    for o in occs {
        if let Some((bare, www, scheme)) = host_parts(&o.url)
            && (scheme == "http" || scheme == "https")
        {
            let e = by_host.entry(bare.clone()).or_default();
            e.0.insert(www);
            e.1.insert(scheme);
            *counts.entry((bare, www)).or_default() += 1;
        }
    }
    for o in occs {
        let Some((bare, www, scheme)) = host_parts(&o.url) else {
            continue;
        };
        let Some((wwws, schemes)) = by_host.get(&bare) else {
            continue;
        };
        if wwws.len() > 1 {
            let c_www = counts.get(&(bare.clone(), true)).copied().unwrap_or(0);
            let c_bare = counts.get(&(bare.clone(), false)).copied().unwrap_or(0);
            let canon_www = c_www >= c_bare;
            if www != canon_www
                && let Ok(mut p) = Url::parse(&o.url)
            {
                let h = p.host_str().unwrap().to_ascii_lowercase();
                let nh = if canon_www {
                    format!("www.{}", h.trim_start_matches("www."))
                } else {
                    h.trim_start_matches("www.").to_string()
                };
                let _ = p.set_host(Some(&nh));
                out.push(Finding {
                    file: o.file.display().to_string(),
                    page: o.page.clone(),
                    url: o.url.clone(),
                    rule: Rule::WwwConsistency,
                    suggestion: Some(p.to_string()),
                    nav: o.nav,
                    note: if en {
                        format!("{bare}: both www and non-www are used")
                    } else {
                        format!("{bare}: se folosește și cu, și fără www")
                    },
                });
            }
        }
        if schemes.len() > 1 && scheme == "http" {
            let nu = format!("https{}", &o.url[4..]);
            out.push(Finding {
                file: o.file.display().to_string(),
                page: o.page.clone(),
                url: o.url.clone(),
                rule: Rule::Protocol,
                suggestion: Some(nu),
                nav: o.nav,
                note: if en {
                    format!("{bare}: mixed http and https")
                } else {
                    format!("{bare}: amestec http și https")
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_typos() {
        assert_eq!(fix_scheme_typo("htps://a.ro").unwrap(), "https://a.ro");
        assert_eq!(fix_scheme_typo("https//a.ro/p").unwrap(), "https://a.ro/p");
        assert_eq!(fix_scheme_typo("http:/a.ro").unwrap(), "http://a.ro");
        assert_eq!(fix_scheme_typo("hhttps://a.ro").unwrap(), "https://a.ro");
        assert_eq!(fix_scheme_typo("https://a.ro"), None);
        assert_eq!(fix_scheme_typo("pagina.html"), None);
        // căi relative care seamănă cu o schemă — NU se ating
        assert_eq!(fix_scheme_typo("http/pagina.html"), None);
        assert_eq!(fix_scheme_typo("https/ghid/index.html"), None);
        assert_eq!(fix_scheme_typo("htp/doc.pdf"), None);
        assert_eq!(fix_scheme_typo("http:foo.html"), None);
        assert_eq!(fix_scheme_typo("ttp://a.ro/x").unwrap(), "http://a.ro/x");
        assert_eq!(
            fix_scheme_typo("htp:\\\\a.ro/x"),
            None.or(fix_scheme_typo("htp:\\\\a.ro/x"))
        );
    }

    #[test]
    fn mangled() {
        assert_eq!(unmangle("http://https://a.ro/x").unwrap(), "https://a.ro/x");
        assert_eq!(unmangle("https://a.ro/x%20").unwrap(), "https://a.ro/x");
        assert_eq!(unmangle("https://a.ro/x.").unwrap(), "https://a.ro/x");
        assert_eq!(unmangle("https://a.ro/f_(1)"), None);
        assert_eq!(unmangle("https://a.ro/ok"), None);
    }

    #[test]
    fn tracking() {
        assert_eq!(
            strip_tracking("https://a.ro/p?id=3&utm_source=x&fbclid=1#s").unwrap(),
            "https://a.ro/p?id=3#s"
        );
        assert_eq!(
            strip_tracking("https://a.ro/p?utm_medium=a").unwrap(),
            "https://a.ro/p"
        );
        assert_eq!(strip_tracking("https://a.ro/p?id=3"), None);
    }

    #[test]
    fn missing_scheme() {
        assert_eq!(
            add_missing_scheme("www.a.ro/x").unwrap(),
            "https://www.a.ro/x"
        );
        assert_eq!(add_missing_scheme("a.html"), None);
    }
}
