//! Verificarea unui singur URL: HTTP(S), fișier local, FTP, mailto.

use crate::config::Config;
use crate::extract::{RawLink, extract_css, extract_html};
use crate::model::*;
use crate::robots::RobotsCache;
use crate::urlutil::{is_html_ext, looks_like_html};
use futures::FutureExt;
use regex::Regex;
use reqwest::Method;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LAST_MODIFIED, LOCATION, RETRY_AFTER};
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use url::Url;

pub struct Task {
    pub url: Url,
    pub kind: LinkKind,
    pub depth: u32,
    pub parse: bool,
    pub internal: bool,
    pub idx: usize,
    pub host: String,
}

pub struct Outcome {
    pub idx: usize,
    pub host: String,
    pub result: LinkResult,
    pub links: Vec<RawLink>,
    pub bad: Vec<(String, LinkKind, String)>,
    pub anchors: Option<HashSet<String>>,
}

/// Pauză minimă între cererile către aceeași gazdă.
#[derive(Default)]
pub struct HostGate {
    next: Mutex<HashMap<String, tokio::time::Instant>>,
}

impl HostGate {
    pub async fn wait(&self, host: &str, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        let at = {
            let mut m = self.next.lock().await;
            let now = tokio::time::Instant::now();
            let at = match m.get(host) {
                Some(t) if *t > now => *t,
                _ => now,
            };
            m.insert(host.to_string(), at + delay);
            at
        };
        tokio::time::sleep_until(at).await;
    }
}

pub struct Shared {
    pub cfg: Arc<Config>,
    pub client: reqwest::Client,
    pub robots: RobotsCache,
    pub gate: HostGate,
    file_roots: Vec<PathBuf>,
    /// Anti-SSRF activ (start public + `block_private`).
    pub block_private: bool,
    priv_cache: std::sync::Mutex<HashMap<String, bool>>,
    dir_cache: std::sync::Mutex<HashMap<PathBuf, Arc<HashSet<String>>>>,
}

impl Shared {
    /// Adresa țintă este privată? (rezultat cache-uit per gazdă)
    pub async fn is_private_target(&self, u: &Url) -> bool {
        if !self.block_private {
            return false;
        }
        let Some(h) = u.host_str() else { return false };
        let key = h.to_ascii_lowercase();
        if let Some(v) = self.priv_cache.lock().unwrap().get(&key) {
            return *v;
        }
        let v = crate::netguard::host_is_private(h, u.port_or_known_default().unwrap_or(80)).await;
        self.priv_cache.lock().unwrap().insert(key, v);
        v
    }

    pub fn new(cfg: Arc<Config>) -> anyhow::Result<Self> {
        let mut b = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(cfg.user_agent.clone())
            .timeout(Duration::from_secs(cfg.timeout_secs.max(1)))
            .connect_timeout(Duration::from_secs(cfg.timeout_secs.clamp(1, 20)))
            .danger_accept_invalid_certs(cfg.ignore_tls_errors)
            .cookie_store(cfg.cookies)
            .pool_max_idle_per_host(cfg.per_host.max(1))
            .pool_idle_timeout(Duration::from_secs(20));
        if let Some(p) = &cfg.proxy {
            b = b.proxy(reqwest::Proxy::all(p.as_str())?);
        }
        let file_roots = cfg
            .start
            .iter()
            .filter_map(|s| crate::urlutil::parse_start(s).ok())
            .filter(|u| u.scheme() == "file")
            .filter_map(|u| u.to_file_path().ok())
            .filter_map(|p| p.parent().map(|x| x.to_path_buf()))
            .collect();
        let starts_private = cfg.start.iter().any(|s| {
            crate::urlutil::parse_start(s)
                .ok()
                .map(|u| match u.scheme() {
                    "file" => true,
                    _ => u
                        .host_str()
                        .map(crate::netguard::host_is_private_syntactic)
                        .unwrap_or(false),
                })
                .unwrap_or(false)
        });
        let block_private = cfg.block_private && !starts_private;
        Ok(Self {
            block_private,
            priv_cache: std::sync::Mutex::new(HashMap::new()),
            client: b.build()?,
            robots: RobotsCache::default(),
            gate: HostGate::default(),
            file_roots,
            dir_cache: std::sync::Mutex::new(HashMap::new()),
            cfg,
        })
    }

    async fn list_dir(&self, dir: &Path) -> Option<Arc<HashSet<String>>> {
        if let Some(v) = self.dir_cache.lock().ok()?.get(dir) {
            return Some(v.clone());
        }
        let mut rd = tokio::fs::read_dir(dir).await.ok()?;
        let mut set = HashSet::new();
        while let Ok(Some(e)) = rd.next_entry().await {
            set.insert(e.file_name().to_string_lossy().to_string());
        }
        let set = Arc::new(set);
        self.dir_cache
            .lock()
            .ok()?
            .insert(dir.to_path_buf(), set.clone());
        Some(set)
    }

    /// Dacă o componentă a căii diferă de numele real doar prin majuscule
    /// (posibil pe Windows/macOS, dar linkul se strică pe servere Linux),
    /// întoarce numele real.
    pub async fn case_mismatch(&self, path: &Path) -> Option<String> {
        let root = self.file_roots.iter().find(|r| path.starts_with(r));
        let (mut cur, rel): (PathBuf, PathBuf) = match root {
            Some(r) => (r.clone(), path.strip_prefix(r).ok()?.to_path_buf()),
            None => (
                path.parent()?.to_path_buf(),
                PathBuf::from(path.file_name()?),
            ),
        };
        for comp in rel.components() {
            let name = comp.as_os_str().to_string_lossy().to_string();
            let listing = self.list_dir(&cur).await?;
            if !listing.contains(&name) {
                let lname = name.to_lowercase();
                return listing.iter().find(|n| n.to_lowercase() == lname).cloned();
            }
            cur.push(&name);
        }
        None
    }
}

enum BodyKind {
    Html,
    Css,
}

struct Body {
    kind: BodyKind,
    text: String,
}

#[derive(Default)]
struct Attempt {
    status: Option<Status>,
    http: Option<u16>,
    msg: String,
    final_url: Option<String>,
    chain: Vec<Hop>,
    ctype: Option<String>,
    size: Option<u64>,
    last_modified: Option<String>,
    body: Option<Body>,
    retryable: bool,
    retry_after_ms: Option<u64>,
}

pub async fn check(sh: Arc<Shared>, t: Task) -> Outcome {
    let idx = t.idx;
    let host = t.host.clone();
    let fallback = LinkResult::new(t.url.as_str(), t.kind, t.depth, t.internal);
    match AssertUnwindSafe(check_inner(sh, t)).catch_unwind().await {
        Ok(o) => o,
        Err(_) => {
            let mut r = fallback;
            r.status = Status::ClientError;
            r.message = "eroare internă la verificare".into();
            Outcome {
                idx,
                host,
                result: r,
                links: vec![],
                bad: vec![],
                anchors: None,
            }
        }
    }
}

async fn check_inner(sh: Arc<Shared>, t: Task) -> Outcome {
    let started = Instant::now();
    let mut r = LinkResult::new(t.url.as_str(), t.kind, t.depth, t.internal);
    let mut body: Option<Body> = None;
    match t.url.scheme() {
        "http" | "https" => body = http_check(&sh, &t, &mut r).await,
        "file" => body = file_check(&sh, &t, &mut r).await,
        "ftp" => ftp_check(&sh, &t, &mut r).await,
        "mailto" => mailto_check(&t, &mut r),
        _ => {
            r.status = Status::Skipped;
            r.message = "schemă neacceptată".into();
        }
    }
    let mut links = vec![];
    let mut bad = vec![];
    let mut anchors = None;
    if let Some(b) = body {
        let page_url = r
            .final_url
            .as_deref()
            .and_then(|u| Url::parse(u).ok())
            .unwrap_or_else(|| t.url.clone());
        match b.kind {
            BodyKind::Html => {
                let e = extract_html(&page_url, &b.text, sh.cfg.check_forms);
                r.title = e.title;
                r.h1 = e.h1;
                links = e.links;
                bad = e.bad;
                anchors = Some(e.anchors);
            }
            BodyKind::Css => {
                links = extract_css(&page_url, &b.text);
            }
        }
        r.parsed = true;
        r.links_found = links.len() as u32;
    }
    r.elapsed_ms = started.elapsed().as_millis() as u64;
    Outcome {
        idx: t.idx,
        host: t.host,
        result: r,
        links,
        bad,
        anchors,
    }
}

// ───────────────────────────── HTTP ─────────────────────────────

fn decode(buf: &[u8], ct: Option<&str>) -> String {
    static SNIFF: OnceLock<Regex> = OnceLock::new();
    let mut label: Option<String> = None;
    if let Some(c) = ct {
        let lc = c.to_ascii_lowercase();
        if let Some(i) = lc.find("charset=") {
            let v = c[i + 8..]
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|ch| ch == '"' || ch == '\'')
                .to_string();
            if !v.is_empty() {
                label = Some(v);
            }
        }
    }
    if label.is_none() {
        if buf.starts_with(&[0xEF, 0xBB, 0xBF]) {
            label = Some("utf-8".into());
        } else {
            let head: String = buf
                .iter()
                .take(2048)
                .map(|b| if b.is_ascii() { *b as char } else { ' ' })
                .collect();
            let re = SNIFF.get_or_init(|| {
                Regex::new(r#"(?i)charset\s*=\s*["']?([A-Za-z0-9_\-:.]+)"#).unwrap()
            });
            if let Some(c) = re.captures(&head) {
                label = Some(c[1].to_string());
            }
        }
    }
    let enc = label
        .and_then(|l| encoding_rs::Encoding::for_label(l.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    enc.decode(buf).0.into_owned()
}

fn classify_error(e: &reqwest::Error) -> (Status, String, bool) {
    let mut chain = e.to_string();
    let mut src: Option<&(dyn std::error::Error + 'static)> = std::error::Error::source(e);
    while let Some(s) = src {
        chain.push_str(" | ");
        chain.push_str(&s.to_string());
        src = s.source();
    }
    let l = chain.to_ascii_lowercase();
    if e.is_timeout() || l.contains("timed out") {
        return (Status::Timeout, "timp de răspuns depășit".into(), true);
    }
    if l.contains("dns")
        || l.contains("failed to lookup")
        || l.contains("name or service not known")
        || l.contains("no such host")
        || l.contains("nodename nor servname")
        || l.contains("temporary failure in name resolution")
        || l.contains("no address associated")
    {
        return (
            Status::DnsFailure,
            "gazda nu a fost găsită (DNS)".into(),
            true,
        );
    }
    if l.contains("certificate")
        || l.contains("tls")
        || l.contains("ssl")
        || l.contains("handshake")
    {
        return (
            Status::TlsError,
            format!("eroare TLS: {}", short(&chain)),
            false,
        );
    }
    if e.is_redirect() {
        return (
            Status::TooManyRedirects,
            "buclă de redirecționări".into(),
            false,
        );
    }
    if e.is_builder() {
        return (Status::InvalidUrl, short(&chain), false);
    }
    (Status::NoConnection, short(&chain), true)
}

fn short(s: &str) -> String {
    let t = s.replace('\n', " ");
    if t.chars().count() > 200 {
        let mut o: String = t.chars().take(200).collect();
        o.push('…');
        o
    } else {
        t
    }
}

fn status_from_code(code: u16) -> Status {
    match code {
        200..=299 | 304 => Status::Ok,
        401 | 403 => Status::Forbidden,
        404 | 410 => Status::NotFound,
        400..=499 => Status::ClientError,
        500..=599 => Status::ServerError,
        _ => Status::ClientError,
    }
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "authorization"
            | "cookie"
            | "proxy-authorization"
            | "x-api-key"
            | "x-auth-token"
            | "x-csrf-token"
            | "x-access-token"
    )
}

async fn http_once(sh: &Shared, t: &Task, delay: Duration) -> Attempt {
    let cfg = &sh.cfg;
    let mut a = Attempt::default();
    let mut cur = t.url.clone();
    let mut method = if t.parse || !cfg.head_first {
        Method::GET
    } else {
        Method::HEAD
    };
    let mut hops = 0usize;
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(cur.to_string());

    loop {
        if sh.is_private_target(&cur).await {
            a.status = Some(Status::Skipped);
            a.msg = "adresă privată/locală blocată (anti-SSRF; vezi --allow-private)".into();
            return a;
        }
        let hk = crate::urlutil::host_key(&cur);
        sh.gate.wait(&hk, delay).await;
        let mut rb = sh.client.request(method.clone(), cur.clone());
        if t.parse && method == Method::GET {
            rb = rb.header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,text/css,application/xml;q=0.9,*/*;q=0.8",
            );
        }
        // Antetele sensibile și autentificarea pleacă doar către site-ul intern de origine,
        // niciodată către alte gazde (inclusiv după redirecționări).
        let same_site = match (cur.host_str(), t.url.host_str()) {
            (Some(a), Some(b)) => crate::urlutil::site_key(a) == crate::urlutil::site_key(b),
            _ => false,
        };
        let trusted = t.internal && same_site;
        for (k, v) in &cfg.headers {
            if !trusted && is_sensitive_header(k) {
                continue;
            }
            rb = rb.header(k.as_str(), v.as_str());
        }
        if trusted && let Some((u, p)) = &cfg.basic_auth {
            rb = rb.basic_auth(u, Some(p));
        }
        let resp = match rb.send().await {
            Ok(r) => r,
            Err(e) => {
                let (st, msg, retry) = classify_error(&e);
                a.status = Some(st);
                a.msg = msg;
                a.retryable = retry;
                return a;
            }
        };
        let code = resp.status().as_u16();

        if (300..400).contains(&code) && code != 304 {
            let Some(loc) = resp
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
            else {
                a.status = Some(Status::BadRedirect);
                a.http = Some(code);
                a.msg = format!("HTTP {code} fără antet Location");
                return a;
            };
            a.chain.push(Hop {
                url: cur.to_string(),
                code,
            });
            hops += 1;
            if hops > cfg.max_redirects {
                a.status = Some(Status::TooManyRedirects);
                a.http = Some(code);
                a.msg = format!("peste {} redirecționări", cfg.max_redirects);
                return a;
            }
            let next = match cur.join(&loc) {
                Ok(n) => n,
                Err(_) => {
                    a.status = Some(Status::BadRedirect);
                    a.http = Some(code);
                    a.msg = format!("țintă de redirecționare invalidă: {}", short(&loc));
                    return a;
                }
            };
            if !visited.insert(next.to_string()) {
                a.status = Some(Status::TooManyRedirects);
                a.http = Some(code);
                a.msg = "buclă de redirecționări".into();
                return a;
            }
            if !matches!(next.scheme(), "http" | "https") {
                // o redirecționare spre file:/javascript:/data: dintr-un server la distanță nu e
                // niciodată legitimă și nu se urmează; o raportăm ca invalidă, nu ca „OK”.
                a.status = Some(Status::BadRedirect);
                a.http = Some(code);
                a.final_url = Some(format!("{}:…", next.scheme()));
                a.msg = format!(
                    "redirecționare către schemă neacceptată ({})",
                    next.scheme()
                );
                return a;
            }
            cur = next;
            continue;
        }

        if method == Method::HEAD && code >= 400 {
            // Multe servere răspund greșit la HEAD: reîncercăm cu GET.
            method = Method::GET;
            continue;
        }

        let headers = resp.headers().clone();
        let ct = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let lm = headers
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let cl = headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        a.retry_after_ms = headers
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after_ms);
        a.http = Some(code);
        a.ctype = ct.clone().map(|c| {
            c.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        });
        a.last_modified = lm;
        a.size = cl;
        if !a.chain.is_empty() || cur != t.url {
            a.final_url = Some(cur.to_string());
        }

        let mut st = status_from_code(code);
        if cfg.ignore_status.contains(&code) {
            st = Status::Ok;
            a.msg = format!("HTTP {code} ignorat conform configurației");
        } else if st != Status::Ok {
            a.msg = format!(
                "HTTP {code} {}",
                resp.status().canonical_reason().unwrap_or("")
            )
            .trim()
            .to_string();
        }
        a.retryable = matches!(code, 429 | 502 | 503 | 504);

        if st == Status::Ok && !a.chain.is_empty() {
            st = Status::Redirected;
            a.msg = format!("{} → {}", a.chain.first().map(|h| h.code).unwrap_or(0), cur);
        }
        a.status = Some(st);

        // Corpul: doar pentru pagini/CSS care trebuie parcurse.
        if t.parse && code < 300 {
            let lower = ct.as_deref().unwrap_or("").to_ascii_lowercase();
            let kind = if lower.contains("html") || lower.contains("xhtml") {
                Some(BodyKind::Html)
            } else if lower.contains("css") {
                Some(BodyKind::Css)
            } else if lower.is_empty()
                && matches!(t.kind, LinkKind::Page | LinkKind::Start | LinkKind::Frame)
                && looks_like_html(&cur)
            {
                Some(BodyKind::Html)
            } else {
                None
            };
            if let Some(kind) = kind {
                let mut resp = resp;
                let mut buf: Vec<u8> = Vec::new();
                loop {
                    match resp.chunk().await {
                        Ok(Some(c)) => {
                            buf.extend_from_slice(&c);
                            if buf.len() >= cfg.max_body_bytes {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            if buf.is_empty() {
                                let (st, msg, retry) = classify_error(&e);
                                a.status = Some(st);
                                a.msg = msg;
                                a.retryable = retry;
                                return a;
                            }
                            break;
                        }
                    }
                }
                a.size = Some(a.size.unwrap_or(0).max(buf.len() as u64));
                a.body = Some(Body {
                    kind,
                    text: decode(&buf, ct.as_deref()),
                });
            }
        }
        return a;
    }
}

async fn http_check(sh: &Shared, t: &Task, r: &mut LinkResult) -> Option<Body> {
    let cfg = &sh.cfg;
    let mut delay = Duration::from_millis(cfg.delay_ms);
    if t.internal && cfg.respect_robots {
        let rules = sh.robots.get(&sh.client, &t.url, &cfg.robots_token()).await;
        let mut pq = t.url.path().to_string();
        if let Some(q) = t.url.query() {
            pq.push('?');
            pq.push_str(q);
        }
        if !rules.allowed(&pq) {
            r.status = Status::RobotsBlocked;
            r.message = "exclus de robots.txt".into();
            return None;
        }
        if let Some(d) = rules.crawl_delay {
            delay = delay.max(Duration::from_secs_f64(d.clamp(0.0, 30.0)));
        }
    }
    let attempts = cfg.retries + 1;
    let mut last = Attempt::default();
    for n in 0..attempts {
        last = http_once(sh, t, delay).await;
        if last.retryable && n + 1 < attempts {
            let wait = last
                .retry_after_ms
                .unwrap_or(600 * (n as u64 + 1))
                .min(10_000);
            tokio::time::sleep(Duration::from_millis(wait)).await;
            continue;
        }
        break;
    }
    r.status = last.status.unwrap_or(Status::NoConnection);
    r.http_status = last.http;
    r.message = last.msg;
    r.final_url = last.final_url;
    r.chain = last.chain;
    r.content_type = last.ctype;
    r.size = last.size;
    r.last_modified = last.last_modified;
    last.body
}

// ───────────────────────────── fișiere locale ─────────────────────────────

async fn file_check(sh: &Shared, t: &Task, r: &mut LinkResult) -> Option<Body> {
    let path = match t.url.to_file_path() {
        Ok(p) => p,
        Err(_) => {
            r.status = Status::InvalidUrl;
            r.message = "cale de fișier invalidă".into();
            return None;
        }
    };
    match tokio::fs::metadata(&path).await {
        Err(e) => {
            r.status = Status::LocalMissing;
            r.message = format!("{}: {e}", path.display());
            None
        }
        Ok(_) if sh.case_mismatch(&path).await.is_some() => {
            let actual = sh.case_mismatch(&path).await.unwrap_or_default();
            r.status = Status::LocalMissing;
            r.message = format!(
                "diferență de majuscule: există „{actual}” — pe servere Linux linkul va eșua"
            );
            None
        }
        Ok(m) if m.is_dir() => {
            r.status = Status::Ok;
            r.message = "director".into();
            None
        }
        Ok(m) => {
            r.status = Status::Ok;
            r.size = Some(m.len());
            r.content_type = mime_guess::from_path(&path)
                .first()
                .map(|m| m.essence_str().to_string());
            if let Ok(mt) = m.modified() {
                let dt: chrono::DateTime<chrono::Utc> = mt.into();
                r.last_modified = Some(dt.to_rfc2822());
            }
            if t.parse {
                let is_css = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("css"))
                    == Some(true);
                if (is_html_ext(&path) || is_css)
                    && let Ok(bytes) = tokio::fs::read(&path).await
                {
                    let cut = &bytes[..bytes.len().min(sh.cfg.max_body_bytes)];
                    return Some(Body {
                        kind: if is_css {
                            BodyKind::Css
                        } else {
                            BodyKind::Html
                        },
                        text: decode(cut, None),
                    });
                }
            }
            None
        }
    }
}

// ───────────────────────────── mailto ─────────────────────────────

fn email_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"^[^@\s<>()\[\]\\,;:"]+@[^@\s<>()\[\]\\,;:"]+\.[^@\s<>()\[\]\\,;:".]{2,}$"#)
            .unwrap()
    })
}

fn mailto_check(t: &Task, r: &mut LinkResult) {
    let s = t.url.as_str();
    let rest = s.strip_prefix("mailto:").unwrap_or("");
    let (addr, query) = match rest.split_once('?') {
        Some((a, q)) => (a, q),
        None => (rest, ""),
    };
    let dec = percent_encoding::percent_decode_str(addr)
        .decode_utf8_lossy()
        .to_string();
    if dec.trim().is_empty() {
        if query.is_empty() {
            r.status = Status::InvalidEmail;
            r.message = "adresă de e-mail lipsă".into();
        } else {
            r.status = Status::Ok;
            r.message = "mailto fără destinatar".into();
        }
        return;
    }
    for a in dec.split(',') {
        let a = a.trim();
        if a.is_empty() {
            continue;
        }
        if !email_re().is_match(a) {
            r.status = Status::InvalidEmail;
            r.message = format!("adresă invalidă: {}", short(a));
            return;
        }
    }
    r.status = Status::Ok;
    r.message = "sintaxă validă (existența căsuței nu se verifică)".into();
}

// ───────────────────────────── FTP ─────────────────────────────

async fn ftp_reply<R: tokio::io::AsyncBufRead + Unpin>(
    rd: &mut R,
) -> std::io::Result<(u16, String)> {
    let mut first_code: Option<String> = None;
    let mut text = String::new();
    loop {
        let mut line = String::new();
        let n = rd.read_line(&mut line).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "conexiune închisă",
            ));
        }
        let l = line.trim_end();
        text.push_str(l);
        text.push(' ');
        if l.len() >= 3 && l.as_bytes()[..3].iter().all(|b| b.is_ascii_digit()) {
            let code = &l[..3];
            match &first_code {
                None => {
                    first_code = Some(code.to_string());
                    if l.len() == 3 || l.as_bytes()[3] == b' ' {
                        break;
                    }
                }
                Some(c) => {
                    if c == code && (l.len() == 3 || l.as_bytes()[3] == b' ') {
                        break;
                    }
                }
            }
        } else if first_code.is_none() {
            break;
        }
    }
    let code = first_code.and_then(|c| c.parse::<u16>().ok()).unwrap_or(0);
    Ok((code, text.trim().to_string()))
}

async fn ftp_cmd<W: tokio::io::AsyncWrite + Unpin, R: tokio::io::AsyncBufRead + Unpin>(
    wr: &mut W,
    rd: &mut R,
    cmd: &str,
) -> std::io::Result<(u16, String)> {
    wr.write_all(cmd.as_bytes()).await?;
    wr.write_all(b"\r\n").await?;
    wr.flush().await?;
    ftp_reply(rd).await
}

async fn ftp_probe(u: &Url) -> Result<(Option<u64>, String), (Status, String)> {
    let host = u.host_str().unwrap_or("").to_string();
    let port = u.port().unwrap_or(21);
    let stream = tokio::net::TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| {
            let m = e.to_string().to_ascii_lowercase();
            if m.contains("lookup") || m.contains("resolve") || m.contains("name or service") {
                (Status::DnsFailure, "gazda FTP nu a fost găsită".to_string())
            } else {
                (Status::NoConnection, short(&e.to_string()))
            }
        })?;
    let io = |e: std::io::Error| (Status::FtpError, short(&e.to_string()));
    let (rd, mut wr) = stream.into_split();
    let mut rd = BufReader::new(rd);
    let (code, txt) = ftp_reply(&mut rd).await.map_err(io)?;
    if code != 220 {
        return Err((
            Status::FtpError,
            format!("salut neașteptat: {}", short(&txt)),
        ));
    }
    let user = percent_encoding::percent_decode_str(u.username())
        .decode_utf8_lossy()
        .to_string();
    let user = if user.is_empty() {
        "anonymous".to_string()
    } else {
        user
    };
    let pass = u
        .password()
        .map(|p| {
            percent_encoding::percent_decode_str(p)
                .decode_utf8_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "anonymous@veriga.invalid".to_string());
    let (code, txt) = ftp_cmd(&mut wr, &mut rd, &format!("USER {user}"))
        .await
        .map_err(io)?;
    match code {
        230 => {}
        331 | 332 => {
            let (c2, t2) = ftp_cmd(&mut wr, &mut rd, &format!("PASS {pass}"))
                .await
                .map_err(io)?;
            if c2 != 230 {
                return Err((
                    Status::FtpError,
                    format!("autentificare refuzată: {}", short(&t2)),
                ));
            }
        }
        _ => {
            return Err((
                Status::FtpError,
                format!("autentificare refuzată: {}", short(&txt)),
            ));
        }
    }
    let path = percent_encoding::percent_decode_str(u.path())
        .decode_utf8_lossy()
        .to_string();
    let result: Result<(Option<u64>, String), (Status, String)>;
    if path.is_empty() || path == "/" || path.ends_with('/') {
        let (c, t) = ftp_cmd(&mut wr, &mut rd, &format!("CWD {path}"))
            .await
            .map_err(io)?;
        result = if c == 250 || c == 200 {
            Ok((None, "director FTP".into()))
        } else {
            Err((
                Status::FtpError,
                format!("director inexistent: {}", short(&t)),
            ))
        };
    } else {
        let _ = ftp_cmd(&mut wr, &mut rd, "TYPE I").await.map_err(io)?;
        let (c, t) = ftp_cmd(&mut wr, &mut rd, &format!("SIZE {path}"))
            .await
            .map_err(io)?;
        if c == 213 {
            let size = t
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u64>().ok());
            result = Ok((size, "fișier FTP".into()));
        } else if c == 550 {
            let (c2, _) = ftp_cmd(&mut wr, &mut rd, &format!("CWD {path}"))
                .await
                .map_err(io)?;
            result = if c2 == 250 {
                Ok((None, "director FTP".into()))
            } else {
                Err((Status::FtpError, "fișierul nu există (550)".into()))
            };
        } else if c == 500 || c == 502 || c == 504 {
            result = Ok((
                None,
                "serverul nu acceptă SIZE; existența nu a putut fi confirmată".into(),
            ));
        } else {
            result = Err((Status::FtpError, short(&t)));
        }
    }
    let _ = ftp_cmd(&mut wr, &mut rd, "QUIT").await;
    result
}

async fn ftp_check(sh: &Shared, t: &Task, r: &mut LinkResult) {
    let to = Duration::from_secs(sh.cfg.timeout_secs.max(1));
    match tokio::time::timeout(to, ftp_probe(&t.url)).await {
        Err(_) => {
            r.status = Status::Timeout;
            r.message = "timp de răspuns FTP depășit".into();
        }
        Ok(Ok((size, msg))) => {
            r.status = Status::Ok;
            r.size = size;
            r.message = msg;
        }
        Ok(Err((st, msg))) => {
            r.status = st;
            r.message = msg;
        }
    }
}

/// `Retry-After` (RFC 9110 §10.2.3): număr de secunde sau dată HTTP (IMF-fixdate).
fn parse_retry_after_ms(v: &str) -> Option<u64> {
    let v = v.trim();
    if let Ok(secs) = v.parse::<u64>() {
        return Some(secs.saturating_mul(1000));
    }
    let when = chrono::DateTime::parse_from_rfc2822(v).ok()?;
    let ms = (when.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_milliseconds();
    Some(ms.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ssrf_guard_blocks_private_targets_from_public_start() {
        let cfg = Arc::new(Config::with_start("https://example.com/"));
        let sh = Shared::new(cfg).unwrap();
        assert!(sh.block_private);
        for u in [
            "http://127.0.0.1:9/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://10.0.0.5/",
            "http://localhost/",
            "http://router/",
        ] {
            assert!(sh.is_private_target(&Url::parse(u).unwrap()).await, "{u}");
        }
        assert!(
            !sh.is_private_target(&Url::parse("http://8.8.8.8/").unwrap())
                .await
        );
        // start local ⇒ protecția se dezactivează (intranet legitim)
        let local = Shared::new(Arc::new(Config::with_start("http://localhost:8080/"))).unwrap();
        assert!(!local.block_private);
        // și poate fi oprită explicit
        let mut c = Config::with_start("https://example.com/");
        c.block_private = false;
        assert!(!Shared::new(Arc::new(c)).unwrap().block_private);
    }

    #[test]
    fn retry_after_accepts_seconds_and_http_date() {
        assert_eq!(parse_retry_after_ms("3"), Some(3000));
        assert_eq!(
            parse_retry_after_ms("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(0)
        ); // în trecut
        let fut = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc2822();
        let ms = parse_retry_after_ms(&fut).unwrap();
        assert!((25_000..=31_000).contains(&ms), "{ms}");
        assert_eq!(parse_retry_after_ms("curând"), None);
    }

    #[test]
    fn sensitive_headers_are_recognized() {
        for h in [
            "Authorization",
            "cookie",
            " X-Api-Key ",
            "Proxy-Authorization",
        ] {
            assert!(is_sensitive_header(h), "{h}");
        }
        assert!(!is_sensitive_header("Accept-Language"));
    }

    #[tokio::test]
    async fn detects_case_mismatch() {
        let dir = std::env::temp_dir().join(format!("veriga-case-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Img")).unwrap();
        std::fs::write(dir.join("Img").join("Logo.png"), "x").unwrap();
        std::fs::write(dir.join("index.html"), "x").unwrap();
        let mut cfg = Config::with_start(dir.join("index.html").to_string_lossy().to_string());
        cfg.start = vec![dir.join("index.html").to_string_lossy().to_string()];
        let sh = Shared::new(Arc::new(cfg)).unwrap();
        assert_eq!(
            sh.case_mismatch(&dir.join("Img").join("logo.png"))
                .await
                .as_deref(),
            Some("Logo.png")
        );
        assert_eq!(
            sh.case_mismatch(&dir.join("img").join("Logo.png"))
                .await
                .as_deref(),
            Some("Img")
        );
        assert_eq!(
            sh.case_mismatch(&dir.join("Img").join("Logo.png")).await,
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
