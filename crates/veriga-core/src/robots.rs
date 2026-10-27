//! Interpretor robots.txt (RFC 9309): grupuri, Allow/Disallow cu `*` și `$`,
//! cea mai lungă potrivire câștigă, Crawl-delay, Sitemap.

use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

#[derive(Debug, Default)]
pub struct Rules {
    rules: Vec<(bool, usize, Regex)>,
    pub crawl_delay: Option<f64>,
    pub sitemaps: Vec<String>,
}

/// Normalizează un șir de cale conform RFC 9309 §2.2.2: octeții non-ASCII sunt
/// codificați procentual, secvențele `%xx` primesc hex majuscul, iar cele care
/// reprezintă caractere „unreserved” sunt decodate (comparație echivalentă).
fn norm_path(p: &str) -> String {
    let b = p.as_bytes();
    let mut out = String::with_capacity(p.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'%'
            && i + 2 < b.len()
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            let v = u8::from_str_radix(&p[i + 1..i + 3], 16).unwrap_or(0);
            if v.is_ascii_alphanumeric() || matches!(v, b'-' | b'.' | b'_' | b'~') {
                out.push(v as char);
            } else {
                out.push_str(&format!("%{v:02X}"));
            }
            i += 3;
        } else if c.is_ascii() {
            out.push(c as char);
            i += 1;
        } else {
            out.push_str(&format!("%{c:02X}"));
            i += 1;
        }
    }
    out
}

/// RFC 9309 §2.5: un parser trebuie să proceseze cel puțin 500 KiB.
pub const MAX_ROBOTS_BYTES: usize = 500 * 1024;

fn compile(pat: &str) -> Option<Regex> {
    if pat.len() > 4096 {
        return None;
    }
    let pat = norm_path(pat);
    let (body, anchored) = match pat.strip_suffix('$') {
        Some(b) => (b, true),
        None => (pat.as_str(), false),
    };
    let mut re = String::from("^");
    for (i, part) in body.split('*').enumerate() {
        if i > 0 {
            re.push_str(".*");
        }
        re.push_str(&regex::escape(part));
    }
    if anchored {
        re.push('$');
    }
    Regex::new(&re).ok()
}

impl Rules {
    pub fn allow_all() -> Self {
        Self::default()
    }

    pub fn parse(text: &str, token: &str) -> Self {
        struct Group {
            agents: Vec<String>,
            rules: Vec<(bool, String)>,
            delay: Option<f64>,
        }
        let mut groups: Vec<Group> = vec![];
        let mut cur = Group {
            agents: vec![],
            rules: vec![],
            delay: None,
        };
        let mut in_rules = false;
        let mut sitemaps = vec![];
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once(':') else {
                continue;
            };
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            match k.as_str() {
                "user-agent" => {
                    if in_rules {
                        groups.push(std::mem::replace(
                            &mut cur,
                            Group {
                                agents: vec![],
                                rules: vec![],
                                delay: None,
                            },
                        ));
                        in_rules = false;
                    }
                    cur.agents.push(v.to_ascii_lowercase());
                }
                "allow" | "disallow" => {
                    in_rules = true;
                    if !v.is_empty() {
                        cur.rules.push((k == "allow", v.to_string()));
                    }
                }
                "crawl-delay" => {
                    in_rules = true;
                    cur.delay = v.parse().ok();
                }
                "sitemap" => sitemaps.push(v.to_string()),
                _ => {}
            }
        }
        if !cur.agents.is_empty() {
            groups.push(cur);
        }

        // Tokenul de produs al crawler-ului (fără versiune), comparat exact, fără
        // diferențe de majuscule (RFC 9309 §2.2.1). Grupurile cu același agent se îmbină.
        let product = token
            .split(['/', ' '])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let matching = |name: &str| -> Vec<&Group> {
            groups
                .iter()
                .filter(|g| g.agents.iter().any(|a| a == name))
                .collect()
        };
        let mut chosen = if product.is_empty() {
            vec![]
        } else {
            matching(&product)
        };
        if chosen.is_empty() {
            chosen = matching("*");
        }
        let mut rules = vec![];
        let mut delay = None;
        for g in chosen {
            delay = delay.or(g.delay);
            for (allow, pat) in &g.rules {
                if let Some(re) = compile(pat) {
                    rules.push((*allow, norm_path(pat).len(), re));
                }
            }
        }
        Rules {
            rules,
            crawl_delay: delay,
            sitemaps,
        }
    }

    /// Interdicție totală (RFC 9309 §2.3.1.4: robots.txt indisponibil din cauza unei erori de server).
    pub fn disallow_all() -> Self {
        Rules {
            rules: vec![(false, 1, Regex::new("^").expect("regex valid"))],
            ..Self::default()
        }
    }

    pub fn allowed(&self, path_and_query: &str) -> bool {
        let path_and_query = &norm_path(path_and_query);
        let mut best: Option<(usize, bool)> = None;
        for (allow, len, re) in &self.rules {
            if re.is_match(path_and_query) {
                match best {
                    None => best = Some((*len, *allow)),
                    Some((l, a)) => {
                        if *len > l || (*len == l && *allow && !a) {
                            best = Some((*len, *allow));
                        }
                    }
                }
            }
        }
        best.map(|(_, a)| a).unwrap_or(true)
    }
}

#[derive(Default)]
pub struct RobotsCache {
    map: Mutex<HashMap<String, Arc<Rules>>>,
}

impl RobotsCache {
    pub async fn get(&self, client: &reqwest::Client, url: &Url, token: &str) -> Arc<Rules> {
        let origin = format!(
            "{}://{}",
            url.scheme(),
            match url.port() {
                Some(p) => format!("{}:{}", url.host_str().unwrap_or(""), p),
                None => url.host_str().unwrap_or("").to_string(),
            }
        );
        if let Some(r) = self.map.lock().await.get(&origin) {
            return r.clone();
        }
        let robots_url = format!("{origin}/robots.txt");
        // RFC 9309 §2.3.1: 2xx → se interpretează; 4xx → fără restricții;
        // 5xx / eroare de rețea → „indisponibil”, deci interdicție totală.
        let rules = match client.get(&robots_url).send().await {
            Ok(mut resp) if resp.status().is_success() => {
                let ct_ok = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|c| !c.contains("html"))
                    .unwrap_or(true);
                if ct_ok {
                    let mut buf: Vec<u8> = Vec::new();
                    loop {
                        match resp.chunk().await {
                            Ok(Some(c)) => {
                                buf.extend_from_slice(&c);
                                if buf.len() >= MAX_ROBOTS_BYTES {
                                    buf.truncate(MAX_ROBOTS_BYTES);
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    Rules::parse(&String::from_utf8_lossy(&buf), token)
                } else {
                    Rules::allow_all()
                }
            }
            Ok(resp) if resp.status().is_server_error() => Rules::disallow_all(),
            Ok(_) => Rules::allow_all(),
            Err(_) => Rules::disallow_all(),
        };
        let rules = Arc::new(rules);
        self.map.lock().await.insert(origin, rules.clone());
        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TXT: &str = "User-agent: *\nDisallow: /privat/\nAllow: /privat/public/\nDisallow: /*.pdf$\nCrawl-delay: 2\n\nUser-agent: veriga\nDisallow: /doar-veriga\n\nSitemap: https://ex.ro/sitemap.xml\n";

    #[test]
    fn wildcard_group() {
        let r = Rules::parse(TXT, "altbot");
        assert!(!r.allowed("/privat/x"));
        assert!(r.allowed("/privat/public/y"));
        assert!(!r.allowed("/doc/a.pdf"));
        assert!(r.allowed("/doc/a.pdf.html"));
        assert_eq!(r.crawl_delay, Some(2.0));
        assert_eq!(r.sitemaps, vec!["https://ex.ro/sitemap.xml"]);
    }

    #[test]
    fn groups_merge_and_encoding_is_normalized() {
        let t = "\u{feff}User-agent: veriga\nDisallow: /a\n\nUser-agent: VERIGA\nDisallow: /b%7e\nDisallow: /ș\n";
        let r = Rules::parse(t, "Veriga/1.0 (+https://x)");
        assert!(!r.allowed("/a/1"));
        assert!(!r.allowed("/b~x")); // %7e ≡ ~
        assert!(!r.allowed("/%C8%99")); // „ș” codificat UTF-8
        assert!(r.allowed("/c"));
        // tokenul nu se potrivește „prin conținere”
        let r2 = Rules::parse("User-agent: bot\nDisallow: /", "robotics/1");
        assert!(r2.allowed("/x"));
    }

    #[test]
    fn unreachable_means_disallow_all() {
        assert!(!Rules::disallow_all().allowed("/orice"));
    }

    #[test]
    fn specific_group_wins() {
        let r = Rules::parse(TXT, "veriga");
        assert!(r.allowed("/privat/x"));
        assert!(!r.allowed("/doar-veriga/a"));
    }
}
