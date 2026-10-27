//! Reparator: propuneri verificate live, aplicare cu copie de siguranță,
//! jurnal de modificări și anulare (individuală sau pe lot).
//!
//! Principii: (1) nu se modifică nimic fără o propunere explicită; (2) o
//! reparație automată se aplică doar dacă URL-ul nou a răspuns cu succes în
//! momentul propunerii; (3) fiecare modificare se poate anula; (4) se scriu
//! doar octeții din interval — restul fișierului rămâne identic.

use crate::model::{Session, Status};
use crate::rules::{self, Rule};
use crate::srcscan::{self, Occurrence, SrcKind};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub file: PathBuf,
    pub page: String,
    pub start: usize,
    pub end: usize,
    /// Textul brut din fișier (ce se înlocuiește).
    pub old_raw: String,
    /// Textul brut nou.
    pub new_raw: String,
    /// URL vechi / nou (pentru afișare).
    pub old: String,
    pub new: String,
    pub rule: Rule,
    /// URL-ul nou a răspuns cu succes la verificarea live.
    pub verified: bool,
    pub nav: bool,
    /// Pe câte pagini apare URL-ul vechi.
    pub pages: usize,
    pub note: String,
}

impl Proposal {
    /// Insignă de impact: apare în meniu sau pe cel puțin 3 pagini.
    pub fn impact(&self) -> bool {
        self.nav || self.pages >= 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixStatus {
    Applied,
    Undone,
    Conflict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixEntry {
    pub id: String,
    pub run: String,
    pub time: String,
    pub file: PathBuf,
    pub rule: Rule,
    pub old: String,
    pub new: String,
    pub old_raw: String,
    pub new_raw: String,
    /// Context înainte de modificare (pentru localizare la anulare).
    pub before: String,
    /// Poziția (octeți) a textului nou în fișier imediat după aplicare.
    #[serde(default)]
    pub pos: usize,
    pub status: FixStatus,
    pub backup: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FixLog {
    pub entries: Vec<FixEntry>,
}

impl FixLog {
    /// Citire tolerantă (doar pentru afișare). Pentru orice scriere folosește [`FixLog::load_checked`].
    pub fn load(path: &Path) -> Self {
        Self::load_checked(path).unwrap_or_default()
    }

    /// Lipsă → jurnal gol; fișier existent dar ilizibil/corupt → eroare (nu suprascriem istoricul de anulare).
    pub fn load_checked(path: &Path) -> anyhow::Result<Self> {
        match std::fs::read(path) {
            Ok(b) if b.iter().all(|c| c.is_ascii_whitespace()) => Ok(Self::default()),
            Ok(b) => serde_json::from_slice(&b).map_err(|e| {
                anyhow::anyhow!(
                    "jurnalul {} este corupt ({e}); nu îl suprascriu — repară-l sau mută-l, apoi reia",
                    path.display()
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(anyhow::anyhow!("nu pot citi jurnalul {}: {e}", path.display())),
        }
    }

    /// Scriere atomică (fișier temporar + redenumire); un crash nu lasă jurnalul pe jumătate scris.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(p) = path.parent()
            && !p.as_os_str().is_empty()
        {
            std::fs::create_dir_all(p)?;
        }
        let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Blocare cooperantă (un singur scriitor): evită două rulări simultane peste același jurnal.
    pub fn lock(path: &Path) -> anyhow::Result<LockGuard> {
        if let Some(p) = path.parent()
            && !p.as_os_str().is_empty()
        {
            std::fs::create_dir_all(p)?;
        }
        let lp = path.with_extension("lock");
        // blocare veche (proces mort) > 1 oră → o ignorăm
        if let Ok(m) = std::fs::metadata(&lp)
            && m.modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|e| e.as_secs() > 3600)
                .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&lp);
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lp)
        {
            Ok(mut f) => {
                use std::io::Write;
                let _ = write!(f, "{}", std::process::id());
                Ok(LockGuard(lp))
            }
            Err(_) => Err(anyhow::anyhow!(
                "altă rulare VERIGA folosește {} (dacă e o blocare rămasă, șterge {})",
                path.display(),
                lp.display()
            )),
        }
    }
}

pub struct LockGuard(PathBuf);
impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ───────────────────────── verificare live ─────────────────────────

#[derive(Clone, Debug)]
pub struct Probe {
    pub code: u16,
    pub final_url: String,
    pub hops: Vec<(String, u16)>,
    pub error: Option<String>,
}

impl Probe {
    pub fn ok(&self) -> bool {
        self.error.is_none() && (200..300).contains(&self.code)
    }
}

pub fn client(timeout: u64, ua: &str) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(timeout))
        .connect_timeout(Duration::from_secs(timeout.clamp(1, 20)))
        .user_agent(ua)
        .build()?)
}

/// Urmărește redirecționările manual; adresele private/locale sunt refuzate (anti-SSRF)
/// dacă nu se cere explicit `allow_private`. Corpul răspunsului nu este citit.
pub async fn probe(c: &reqwest::Client, url: &str, allow_private: bool) -> Probe {
    let mut cur = url.to_string();
    let mut hops = vec![];
    for _ in 0..10 {
        let Ok(u) = url::Url::parse(&cur) else {
            return Probe {
                code: 0,
                final_url: cur,
                hops,
                error: Some("URL invalid".into()),
            };
        };
        if !matches!(u.scheme(), "http" | "https") {
            return Probe {
                code: 0,
                final_url: cur,
                hops,
                error: Some("schemă neacceptată".into()),
            };
        }
        if !allow_private
            && let Some(h) = u.host_str()
            && crate::netguard::host_is_private(h, u.port_or_known_default().unwrap_or(80)).await
        {
            return Probe {
                code: 0,
                final_url: cur,
                hops,
                error: Some("adresă privată/locală blocată (anti-SSRF)".into()),
            };
        }
        let mut resp = c.head(&cur).send().await;
        let need_get = match &resp {
            Ok(r) => r.status().as_u16() >= 400,
            Err(_) => true,
        };
        if need_get {
            resp = c.get(&cur).send().await;
        }
        match resp {
            Err(e) => {
                return Probe {
                    code: 0,
                    final_url: cur,
                    hops,
                    error: Some(e.to_string()),
                };
            }
            Ok(r) => {
                let code = r.status().as_u16();
                if (300..400).contains(&code) {
                    let Some(loc) = r
                        .headers()
                        .get(reqwest::header::LOCATION)
                        .and_then(|v| v.to_str().ok())
                    else {
                        return Probe {
                            code,
                            final_url: cur,
                            hops,
                            error: Some("Location lipsă".into()),
                        };
                    };
                    let Ok(next) = u.join(loc) else {
                        return Probe {
                            code,
                            final_url: cur,
                            hops,
                            error: Some("Location invalid".into()),
                        };
                    };
                    hops.push((cur.clone(), code));
                    cur = next.to_string();
                    continue;
                }
                return Probe {
                    code,
                    final_url: cur,
                    hops,
                    error: None,
                };
            }
        }
    }
    Probe {
        code: 0,
        final_url: cur,
        hops,
        error: Some("prea multe redirecționări".into()),
    }
}

fn norm_cmp(u: &str) -> String {
    u.split('#')
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// Cheie de „aceeași resursă”: fără schemă, fără `www.`, fără `/` final, fără fragment.
fn resource_key(u: &str) -> Option<String> {
    let p = url::Url::parse(u).ok()?;
    let h = p.host_str()?.to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h).to_string();
    let port = match (p.scheme(), p.port()) {
        (_, None) | ("http", Some(80)) | ("https", Some(443)) => String::new(),
        (_, Some(x)) => format!(":{x}"),
    };
    let path = p.path().trim_end_matches('/');
    Some(format!("{h}{port}{path}?{}", p.query().unwrap_or("")))
}

/// Ținta unei reparații pare un „soft-404” (pagina principală sau o pagină de autentificare)?
fn suspicious_target(requested: &str, landed: &str) -> bool {
    let (Ok(r), Ok(l)) = (url::Url::parse(requested), url::Url::parse(landed)) else {
        return false;
    };
    let root = |p: &url::Url| p.path().trim_matches('/').is_empty() && p.query().is_none();
    if root(&l) && !root(&r) {
        return true;
    }
    static LOGIN: OnceLock<Regex> = OnceLock::new();
    let re = LOGIN.get_or_init(|| {
        Regex::new(r"(?i)(^|[/_-])(log-?in|sign-?in|sso|consent|captcha|cookie-?wall|auth(entication)?|not-?found|404|error)([/_.-]|$)").unwrap()
    });
    re.is_match(l.path()) && !re.is_match(r.path())
}

// ───────────────────────── propuneri ─────────────────────────

#[derive(Clone, Debug)]
pub struct ProposeOpts {
    /// Verifică live URL-ul nou (recomandat; fără el nimic nu e marcat „verificat”).
    pub verify: bool,
    pub timeout_secs: u64,
    pub user_agent: String,
    /// Dacă e Some, doar aceste reguli.
    pub only: Option<BTreeSet<Rule>>,
    pub lang: String,
    pub concurrency: usize,
    /// Propune `https://` pentru orice `http://` (necesită `verify`); altfel doar când site-ul amestecă schemele.
    pub upgrade_http: bool,
    /// Permite verificarea adreselor private/locale (dezactivat implicit: anti-SSRF).
    pub allow_private: bool,
}

impl Default for ProposeOpts {
    fn default() -> Self {
        Self {
            verify: true,
            timeout_secs: 20,
            user_agent: format!(
                "Veriga/{} (+link-checker; clean-room)",
                env!("CARGO_PKG_VERSION")
            ),
            only: None,
            lang: "ro".into(),
            concurrency: 12,
            upgrade_http: true,
            allow_private: false,
        }
    }
}

/// Candidatul pentru un URL: (regulă, URL nou) înainte de verificare.
fn candidate(
    url: &str,
    sess: Option<&HashMap<String, &crate::model::LinkResult>>,
    wwwproto: &HashMap<String, (Rule, String)>,
    upgrade_http: bool,
) -> Option<(Rule, String)> {
    // transformări locale repetate (ex. „http://https://a.ro/x?utm_source=1.” → mai multe pase)
    let mut cur = url.to_string();
    let mut first: Option<Rule> = None;
    for _ in 0..4 {
        match rules::local_fix(&cur) {
            Some((r, n)) => {
                first.get_or_insert(r);
                cur = n;
            }
            None => break,
        }
    }
    if let Some(r) = first {
        return Some((r, cur));
    }
    if let Some(v) = wwwproto.get(url) {
        return Some(v.clone());
    }
    if let Some(m) = sess {
        let key = norm_cmp(url);
        if let Some(res) = m.get(&key)
            && res.status == Status::Redirected
            && !res.chain.is_empty()
            && res.chain.iter().all(|h| matches!(h.code, 301 | 308))
            && let Some(f) = &res.final_url
            && norm_cmp(f) != key
        {
            return Some((Rule::Redirect, f.clone()));
        }
    }
    if upgrade_http && url.len() > 7 && url[..7].eq_ignore_ascii_case("http://") {
        return Some((Rule::Protocol, format!("https://{}", &url[7..])));
    }
    None
}

/// Decide dacă rezultatul sondei justifică o reparație automată.
fn accept(rule: Rule, old: &str, new: &str, p: &Probe, en: bool) -> (bool, String) {
    let t = |ro: String, e: String| if en { e } else { ro };
    if let Some(e) = &p.error {
        return (
            false,
            t(format!("neverificat: {e}"), format!("not verified: {e}")),
        );
    }
    if !(200..300).contains(&p.code) {
        return (
            false,
            t(
                format!("neverificat: HTTP {}", p.code),
                format!("not verified: HTTP {}", p.code),
            ),
        );
    }
    if suspicious_target(old, &p.final_url) && suspicious_target(new, &p.final_url) {
        return (
            false,
            t(
                format!(
                    "țintă suspectă (pagină principală / autentificare): {}",
                    p.final_url
                ),
                format!("suspicious target (home / login): {}", p.final_url),
            ),
        );
    }
    let same = match (resource_key(new), resource_key(&p.final_url)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if !same {
        let _ = rule;
        return (
            false,
            t(
                format!("redirecționează mai departe, către {}", p.final_url),
                format!("redirects further to {}", p.final_url),
            ),
        );
    }
    (
        true,
        t(
            format!("verificat: HTTP {}", p.code),
            format!("verified: HTTP {}", p.code),
        ),
    )
}

pub async fn propose(
    occs: &[Occurrence],
    sess: Option<&Session>,
    o: &ProposeOpts,
) -> Vec<Proposal> {
    let en = o.lang == "en";
    // inconsistențe www / protocol (calculate pe tot corpusul)
    let mut wwwproto: HashMap<String, (Rule, String)> = HashMap::new();
    for f in rules::analyze(occs, &o.lang) {
        if matches!(f.rule, Rule::WwwConsistency | Rule::Protocol)
            && let Some(s) = f.suggestion
        {
            wwwproto.entry(f.url).or_insert((f.rule, s));
        }
    }
    let sess_map: Option<HashMap<String, &crate::model::LinkResult>> =
        sess.map(|s| s.results.iter().map(|r| (norm_cmp(&r.url), r)).collect());
    // pagini per URL
    let mut pages: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for oc in occs {
        pages
            .entry(oc.url.as_str())
            .or_default()
            .insert(oc.page.as_str());
    }
    // candidați unici
    let mut cands: BTreeMap<String, (Rule, String)> = BTreeMap::new();
    for oc in occs {
        if cands.contains_key(&oc.url) {
            continue;
        }
        if let Some((r, n)) = candidate(
            &oc.url,
            sess_map.as_ref(),
            &wwwproto,
            o.upgrade_http && o.verify,
        ) && o.only.as_ref().map(|s| s.contains(&r)).unwrap_or(true)
            && n != oc.url
        {
            cands.insert(oc.url.clone(), (r, n));
        }
    }
    // verificare live (o dată per URL nou)
    let mut probes: HashMap<String, Probe> = HashMap::new();
    if o.verify {
        use futures::stream::{self, StreamExt};
        if let Ok(c) = client(o.timeout_secs, &o.user_agent) {
            let news: BTreeSet<String> = cands.values().map(|(_, n)| n.clone()).collect();
            let allow = o.allow_private;
            let res: Vec<(String, Probe)> = stream::iter(news)
                .map(|n| {
                    let c = c.clone();
                    async move {
                        let p = probe(&c, &n, allow).await;
                        (n, p)
                    }
                })
                .buffer_unordered(o.concurrency.max(1))
                .collect()
                .await;
            probes.extend(res);
        }
    }
    let mut out = vec![];
    for oc in occs {
        let Some((rule, new)) = cands.get(&oc.url) else {
            continue;
        };
        let (verified, note) = match probes.get(new) {
            Some(p) => accept(*rule, &oc.url, new, p, en),
            None => (
                false,
                if en {
                    "not verified".into()
                } else {
                    "neverificat".into()
                },
            ),
        };
        out.push(Proposal {
            id: format!("p{}", out.len() + 1),
            file: oc.file.clone(),
            page: oc.page.clone(),
            start: oc.start,
            end: oc.end,
            old_raw: String::new(),
            new_raw: srcscan::encode_like(new, oc.amp),
            old: oc.url.clone(),
            new: new.clone(),
            rule: *rule,
            verified,
            nav: oc.nav,
            pages: pages.get(oc.url.as_str()).map(|s| s.len()).unwrap_or(1),
            note,
        });
    }
    out
}

// ───────────────────────── siguranța textului inserat ─────────────────────────

/// Un URL inserat într-un fișier-sursă nu are voie să „evadeze” din context: ghilimele, `<`, `>`,
/// spații/control, backtick, backslash ar putea rupe un atribut HTML/XML; în Markdown, parantezele
/// și parantezele pătrate ar putea schimba structura linkului. Un server rău intenționat poate pune
/// astfel de caractere într-un antet `Location` — de aceea verificăm TOATE înlocuirile de URL.
pub fn unsafe_url_for(file: &Path, url: &str) -> Option<&'static str> {
    if url.is_empty() {
        return Some("URL gol");
    }
    if url.chars().any(|c| {
        c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '`' | '\\')
    }) {
        return Some(
            "URL nou conține caractere nesigure (ghilimele, <, >, spații, backtick sau backslash)",
        );
    }
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(ext.as_str(), "md" | "markdown" | "mdx")
        && url.chars().any(|c| matches!(c, '(' | ')' | '[' | ']'))
    {
        return Some("URL nou conține paranteze, nesigur în Markdown");
    }
    None
}

// ───────────────────────── reparații manuale ─────────────────────────

/// Înlocuire manuală a unui URL cu altul (în toate aparițiile).
pub fn manual_replace(occs: &[Occurrence], old: &str, new: &str) -> Result<Vec<Proposal>, String> {
    if let Some(why) = occs
        .iter()
        .filter(|o| o.url == old)
        .find_map(|o| unsafe_url_for(&o.file, new))
    {
        return Err(why.to_string());
    }
    Ok(manual_replace_inner(occs, old, new))
}

fn manual_replace_inner(occs: &[Occurrence], old: &str, new: &str) -> Vec<Proposal> {
    let pages = occs
        .iter()
        .filter(|o| o.url == old)
        .map(|o| o.page.clone())
        .collect::<BTreeSet<_>>()
        .len();
    occs.iter()
        .filter(|o| o.url == old)
        .enumerate()
        .map(|(i, o)| Proposal {
            id: format!("m{}", i + 1),
            file: o.file.clone(),
            page: o.page.clone(),
            start: o.start,
            end: o.end,
            old_raw: String::new(),
            new_raw: srcscan::encode_like(new, o.amp),
            old: o.url.clone(),
            new: new.to_string(),
            rule: Rule::Manual,
            verified: false,
            nav: o.nav,
            pages,
            note: "manual".into(),
        })
        .collect()
}

/// „Dezleagă” un link: `<a href="x">text</a>` → `text`, `[text](x)` → `text`.
pub fn unlink(occs: &[Occurrence], old: &str) -> Vec<Proposal> {
    let mut out = vec![];
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let pages = occs
        .iter()
        .filter(|o| o.url == old)
        .map(|o| o.page.clone())
        .collect::<BTreeSet<_>>()
        .len();
    for o in occs.iter().filter(|o| o.url == old) {
        let text = cache
            .entry(o.file.clone())
            .or_insert_with(|| std::fs::read_to_string(&o.file).unwrap_or_default())
            .clone();
        let span = match o.attr.as_str() {
            "href" => {
                // eticheta trebuie să fie exact <a …> (nu <area>, <abbr>, <article>) și
                // atributul să aparțină ei (fără „>” între deschiderea tagului și atribut)
                let lo = text[..o.start]
                    .rfind("<a")
                    .filter(|p| o.start - p < 600)
                    .filter(|p| {
                        text[p + 2..]
                            .chars()
                            .next()
                            .map(|c| c.is_whitespace())
                            .unwrap_or(false)
                    })
                    .filter(|p| !text[*p..o.start].contains('>'));
                let close = text[o.end..].find("</a>").filter(|p| *p < 4000);
                match (lo, close) {
                    (Some(a), Some(c)) => {
                        let tag_end = text[o.end..].find('>').map(|p| o.end + p + 1);
                        tag_end
                            .filter(|te| *te <= o.end + c)
                            .map(|te| (a, o.end + c + 4, text[te..o.end + c].to_string()))
                    }
                    _ => None,
                }
            }
            "md" => {
                let open = text[..o.start].rfind("](");
                let lb = open.and_then(|p| text[..p].rfind('['));
                let rb = text[o.end..].find(')').map(|p| o.end + p + 1);
                match (lb, open, rb) {
                    (Some(a), Some(p), Some(b))
                        if !text[a..a + 1].is_empty() && !(a > 0 && &text[a - 1..a] == "!") =>
                    {
                        Some((a, b, text[a + 1..p].to_string()))
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some((s, e, inner)) = span {
            out.push(Proposal {
                id: format!("u{}", out.len() + 1),
                file: o.file.clone(),
                page: o.page.clone(),
                start: s,
                end: e,
                old_raw: text[s..e].to_string(),
                new_raw: inner.clone(),
                old: o.url.clone(),
                new: format!("«{}»", inner.chars().take(40).collect::<String>()),
                rule: Rule::Manual,
                verified: false,
                nav: o.nav,
                pages,
                note: "unlink".into(),
            });
        }
    }
    out
}

/// Caută o arhivă în Wayback Machine (opt-in; nu se apelează automat).
pub async fn wayback_lookup(c: &reqwest::Client, url: &str) -> Option<String> {
    let q = format!(
        "https://archive.org/wayback/available?url={}",
        percent_encoding::utf8_percent_encode(url, percent_encoding::NON_ALPHANUMERIC)
    );
    let body = c.get(q).send().await.ok()?.bytes().await.ok()?;
    let v: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let u = v.pointer("/archived_snapshots/closest/url")?.as_str()?;
    Some(u.replacen("http://", "https://", 1))
}

// ───────────────────────── aplicare / anulare ─────────────────────────

pub struct ApplyOpts {
    pub backup_dir: PathBuf,
    pub run: String,
    /// Aplică și propunerile neverificate (doar manuale sau cu --force).
    pub allow_unverified: bool,
}

#[derive(Debug, Default)]
pub struct ApplyReport {
    pub applied: Vec<FixEntry>,
    pub skipped: Vec<(String, String)>,
}

/// Scriere atomică: păstrează permisiunile, urmează legăturile simbolice (nu le înlocuiește
/// cu un fișier obișnuit) și nu lasă fișiere temporare la eroare.
fn write_atomic(path: &Path, data: &str) -> anyhow::Result<()> {
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "f".into());
    let tmp = target.with_file_name(format!(".{name}.veriga-{}.tmp", std::process::id()));
    let res = (|| -> anyhow::Result<()> {
        std::fs::write(&tmp, data)?;
        if let Ok(m) = std::fs::metadata(&target) {
            let _ = std::fs::set_permissions(&tmp, m.permissions());
        }
        std::fs::rename(&tmp, &target)?;
        Ok(())
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// Hash FNV-1a 64 pentru nume unice de copii de siguranță (evită coliziunile a/b__c vs a__b/c).
fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn run_id() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn apply(props: &[Proposal], log: &mut FixLog, o: &ApplyOpts) -> ApplyReport {
    let mut rep = ApplyReport::default();
    let mut by_file: BTreeMap<&Path, Vec<&Proposal>> = BTreeMap::new();
    for p in props {
        if p.rule.auto() && p.rule != Rule::Manual && !p.verified && !o.allow_unverified {
            rep.skipped.push((p.id.clone(), "neverificat live".into()));
            continue;
        }
        by_file.entry(p.file.as_path()).or_default().push(p);
    }
    for (file, mut ps) in by_file {
        let Ok(orig) = std::fs::read_to_string(file) else {
            for p in ps {
                rep.skipped.push((p.id.clone(), "fișier ilizibil".into()));
            }
            continue;
        };
        ps.sort_by_key(|p| std::cmp::Reverse(p.start));
        let mut text = orig.clone();
        let mut done: Vec<(usize, FixEntry)> = vec![];
        let mut last_start = usize::MAX;
        for p in ps {
            if p.end > last_start {
                rep.skipped.push((p.id.clone(), "suprapunere".into()));
                continue;
            }
            let Some(cur) = orig.get(p.start..p.end) else {
                rep.skipped
                    .push((p.id.clone(), "fișier modificat între timp".into()));
                continue;
            };
            if p.old_raw.is_empty() {
                rep.skipped
                    .push((p.id.clone(), "propunere incompletă (old_raw lipsă)".into()));
                continue;
            }
            if p.note != "unlink"
                && let Some(why) = unsafe_url_for(file, &p.new_raw.replace("&amp;", "&"))
            {
                rep.skipped.push((p.id.clone(), why.into()));
                continue;
            }
            if cur != p.old_raw {
                rep.skipped
                    .push((p.id.clone(), "fișier modificat între timp".into()));
                continue;
            }
            text.replace_range(p.start..p.end, &p.new_raw);
            last_start = p.start;
            done.push((
                p.start,
                FixEntry {
                    id: format!("{}-{}", o.run, log.entries.len() + done.len() + 1),
                    run: o.run.clone(),
                    time: chrono::Local::now().to_rfc3339(),
                    file: std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf()),
                    rule: p.rule,
                    old: p.old.clone(),
                    new: p.new.clone(),
                    old_raw: cur.to_string(),
                    new_raw: p.new_raw.clone(),
                    before: String::new(),
                    pos: 0,
                    status: FixStatus::Applied,
                    backup: None,
                },
            ));
        }
        if done.is_empty() {
            continue;
        }
        // copie de siguranță (o singură dată per rulare și fișier)
        let full = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        let rel = format!(
            "{:016x}-{}",
            fnv64(&full.to_string_lossy()),
            full.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        );
        let bpath = o.backup_dir.join(&o.run).join(rel);
        let backup = if std::fs::create_dir_all(bpath.parent().unwrap()).is_ok()
            && std::fs::write(&bpath, &orig).is_ok()
        {
            Some(bpath)
        } else {
            for (_, e) in &done {
                rep.skipped
                    .push((e.id.clone(), "copia de siguranță a eșuat".into()));
            }
            continue;
        };
        // TOCTOU: dacă fișierul s-a schimbat între citire și scriere, nu-l atingem
        if std::fs::read_to_string(file).ok().as_deref() != Some(orig.as_str()) {
            for (_, e) in &done {
                rep.skipped.push((
                    e.id.clone(),
                    "fișierul s-a modificat în timpul lucrului".into(),
                ));
            }
            continue;
        }
        if let Err(e) = write_atomic(file, &text) {
            for (_, en) in &done {
                rep.skipped
                    .push((en.id.clone(), format!("scriere eșuată: {e}")));
            }
            continue;
        }
        done.sort_by_key(|(s, _)| *s);
        // context calculat pe textul FINAL (pozițiile se deplasează după înlocuiri anterioare)
        let mut shift: isize = 0;
        for (s0, e) in done.iter_mut() {
            let ns = (*s0 as isize + shift) as usize;
            let from = ns.saturating_sub(24);
            let from = (from..=ns)
                .find(|i| text.is_char_boundary(*i))
                .unwrap_or(ns);
            e.before = text[from..ns].to_string();
            e.pos = ns;
            shift += e.new_raw.len() as isize - e.old_raw.len() as isize;
        }
        for (_, mut e) in done {
            e.backup = backup.clone();
            log.entries.push(e.clone());
            rep.applied.push(e);
        }
    }
    rep
}

#[derive(Debug, Default)]
pub struct UndoReport {
    pub undone: Vec<String>,
    pub conflicts: Vec<(String, String)>,
}

fn find_unique(text: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let mut it = text.match_indices(needle);
    let first = it.next()?.0;
    if it.next().is_some() {
        None
    } else {
        Some(first)
    }
}

/// Anulează intrările cu ID-urile date (sau toate din rularea `run`).
pub fn undo(log: &mut FixLog, ids: Option<&[String]>, run: Option<&str>) -> UndoReport {
    let mut rep = UndoReport::default();
    let mut idx: Vec<usize> = log
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.status == FixStatus::Applied)
        .filter(|(_, e)| ids.map(|v| v.contains(&e.id)).unwrap_or(true))
        .filter(|(_, e)| run.map(|r| e.run == r).unwrap_or(true))
        .map(|(i, _)| i)
        .collect();
    idx.reverse(); // cele mai noi primele
    let mut texts: HashMap<PathBuf, String> = HashMap::new();
    for i in idx {
        let e = log.entries[i].clone();
        let t = texts
            .entry(e.file.clone())
            .or_insert_with(|| std::fs::read_to_string(&e.file).unwrap_or_default());
        let ctx = format!("{}{}", e.before, e.new_raw);
        // 1) poziția înregistrată, confirmată de context; 2) context unic; 3) text nou unic
        let at_pos = (e.pos >= e.before.len()
            && t.get(e.pos - e.before.len()..e.pos + e.new_raw.len()) == Some(ctx.as_str()))
        .then_some(e.pos);
        let pos = at_pos
            .or_else(|| find_unique(t, &ctx).map(|p| p + e.before.len()))
            .or_else(|| find_unique(t, &e.new_raw));
        match pos {
            Some(p) if t.get(p..p + e.new_raw.len()) == Some(e.new_raw.as_str()) => {
                t.replace_range(p..p + e.new_raw.len(), &e.old_raw);
                log.entries[i].status = FixStatus::Undone;
                rep.undone.push(e.id.clone());
            }
            _ => {
                log.entries[i].status = FixStatus::Conflict;
                rep.conflicts.push((
                    e.id.clone(),
                    "textul a fost modificat sau este ambiguu".into(),
                ));
            }
        }
    }
    for (f, t) in texts {
        let _ = write_atomic(&f, &t);
    }
    rep
}

// ───────────────────────── raport propuneri (CSV) ─────────────────────────

/// Neutralizează celulele care ar fi interpretate ca formule în Excel/Calc.
pub fn csv_safe(s: &str) -> String {
    match s.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{s}"),
        _ => s.to_string(),
    }
}

pub fn proposals_csv(props: &[Proposal], lang: &str) -> String {
    let en = lang == "en";
    let mut w = csv::Writer::from_writer(vec![]);
    let _ = w.write_record(if en {
        [
            "ID", "File", "Page", "Rule", "Old URL", "New URL", "Verified", "Impact", "Pages",
            "Note",
        ]
    } else {
        [
            "ID",
            "Fișier",
            "Pagină",
            "Regulă",
            "URL vechi",
            "URL nou",
            "Verificat",
            "Impact",
            "Pagini",
            "Notă",
        ]
    });
    for p in props {
        let _ = w.write_record([
            csv_safe(&p.id),
            csv_safe(&p.file.display().to_string()),
            csv_safe(&p.page),
            csv_safe(p.rule.id()),
            csv_safe(&p.old),
            csv_safe(&p.new),
            (if p.verified { "da" } else { "nu" }).to_string(),
            (if p.impact() { "da" } else { "nu" }).to_string(),
            p.pages.to_string(),
            csv_safe(&p.note),
        ]);
    }
    String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
}

pub fn log_csv(log: &FixLog) -> String {
    let mut w = csv::Writer::from_writer(vec![]);
    let _ = w.write_record(["id", "run", "time", "file", "rule", "old", "new", "status"]);
    for e in &log.entries {
        let _ = w.write_record([
            csv_safe(&e.id),
            csv_safe(&e.run),
            csv_safe(&e.time),
            csv_safe(&e.file.display().to_string()),
            csv_safe(e.rule.id()),
            csv_safe(&e.old),
            csv_safe(&e.new),
            format!("{:?}", e.status),
        ]);
    }
    String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
}

/// Citește fișiere, scanează și umple `old_raw` din conținutul real (apelat de CLI/GUI).
/// Completează `old_raw` din fișier și ELIMINĂ propunerile „învechite”: dacă textul de la
/// poziția scanată nu mai este URL-ul scanat (fișier editat între scanare și aplicare),
/// poziția nu mai e de încredere.
pub fn fill_old_raw(props: &mut Vec<Proposal>) {
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    props.retain_mut(|p| {
        if !p.old_raw.is_empty() {
            return true;
        }
        let t = cache
            .entry(p.file.clone())
            .or_insert_with(|| std::fs::read_to_string(&p.file).unwrap_or_default());
        match t.get(p.start..p.end) {
            Some(s)
                if s.replace("&amp;", "&")
                    .replace("&#038;", "&")
                    .replace("&#38;", "&")
                    .trim()
                    == p.old =>
            {
                p.old_raw = s.to_string();
                true
            }
            _ => false,
        }
    });
}

#[allow(dead_code)]
fn _unused(_: Regex, _: SrcKind) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(file: &Path, text: &str, needle: &str, new: &str, rule: Rule) -> Proposal {
        let s = text.find(needle).unwrap();
        Proposal {
            id: "t1".into(),
            file: file.to_path_buf(),
            page: "p".into(),
            start: s,
            end: s + needle.len(),
            old_raw: needle.into(),
            new_raw: new.into(),
            old: needle.into(),
            new: new.into(),
            rule,
            verified: true,
            nav: false,
            pages: 1,
            note: String::new(),
        }
    }

    #[test]
    fn apply_and_undo_roundtrip() {
        let d = std::env::temp_dir().join(format!("veriga-fix-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.html");
        let src = r#"<a href="htp://a.ro/x">1</a> <a href="htp://a.ro/x">2</a>"#;
        std::fs::write(&f, src).unwrap();
        let occ = srcscan::scan_text(src, &f, SrcKind::Html);
        let mut ps: Vec<Proposal> = occ
            .iter()
            .map(|o| Proposal {
                id: format!("p{}", o.start),
                file: f.clone(),
                page: o.page.clone(),
                start: o.start,
                end: o.end,
                old_raw: o.url.clone(),
                new_raw: "http://a.ro/x".into(),
                old: o.url.clone(),
                new: "http://a.ro/x".into(),
                rule: Rule::SchemeTypo,
                verified: true,
                nav: false,
                pages: 1,
                note: String::new(),
            })
            .collect();
        fill_old_raw(&mut ps);
        let mut log = FixLog::default();
        let r = apply(
            &ps,
            &mut log,
            &ApplyOpts {
                backup_dir: d.join(".veriga-backup"),
                run: "r1".into(),
                allow_unverified: false,
            },
        );
        assert_eq!(r.applied.len(), 2, "{:?}", r.skipped);
        let now = std::fs::read_to_string(&f).unwrap();
        assert_eq!(now, src.replace("htp://", "http://"));
        let u = undo(&mut log, None, Some("r1"));
        assert_eq!(u.undone.len(), 2, "{:?}", u.conflicts);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
        assert!(log.entries.iter().all(|e| e.status == FixStatus::Undone));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unverified_skipped_and_stale_detected() {
        let d = std::env::temp_dir().join(format!("veriga-fix2-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("b.md");
        let src = "[x](htp://a.ro/y)";
        std::fs::write(&f, src).unwrap();
        let mut p = prop(&f, src, "htp://a.ro/y", "http://a.ro/y", Rule::SchemeTypo);
        p.verified = false;
        let mut log = FixLog::default();
        let o = ApplyOpts {
            backup_dir: d.join("bk"),
            run: "r".into(),
            allow_unverified: false,
        };
        assert_eq!(apply(&[p.clone()], &mut log, &o).applied.len(), 0);
        p.verified = true;
        std::fs::write(&f, "XX[x](htp://a.ro/y)").unwrap(); // s-a schimbat între timp
        let r = apply(&[p], &mut log, &o);
        assert_eq!(r.applied.len(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unlink_html_and_md() {
        let d = std::env::temp_dir().join(format!("veriga-fix3-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("c.md");
        let src = "Vezi [pagina](https://x.ro/mort) și <a href=\"https://x.ro/mort\">alta</a>.";
        std::fs::write(&f, src).unwrap();
        let occ = srcscan::scan_text(src, &f, SrcKind::Markdown);
        let ps = unlink(&occ, "https://x.ro/mort");
        assert_eq!(ps.len(), 2);
        let mut log = FixLog::default();
        let o = ApplyOpts {
            backup_dir: d.join("bk"),
            run: "r".into(),
            allow_unverified: true,
        };
        let r = apply(&ps, &mut log, &o);
        assert_eq!(r.applied.len(), 2, "{:?}", r.skipped);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "Vezi pagina și alta.");
        assert_eq!(undo(&mut log, None, None).undone.len(), 2);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn many_edits_roundtrip_with_shifting_context() {
        let d = std::env::temp_dir().join(format!("veriga-fix4-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("d.html");
        let src = r#"<a href="http://x.ro/g">a</a> <a href="http://x.ro/g">b</a> <a href="http://x.ro/g">c</a>"#;
        std::fs::write(&f, src).unwrap();
        let occ = srcscan::scan_text(src, &f, SrcKind::Html);
        let ps = unlink(&occ, "http://x.ro/g");
        assert_eq!(ps.len(), 3);
        let mut log = FixLog::default();
        let o = ApplyOpts {
            backup_dir: d.join("bk"),
            run: "r".into(),
            allow_unverified: true,
        };
        assert_eq!(apply(&ps, &mut log, &o).applied.len(), 3);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "a b c");
        let u = undo(&mut log, None, None);
        assert_eq!(u.undone.len(), 3, "{:?}", u.conflicts);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unlink_ignores_area_and_other_a_tags() {
        let d = std::env::temp_dir().join(format!("veriga-fix5-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("m.html");
        let src = r#"<a href="http://x.ro/ok">bun</a><map><area href="http://x.ro/g" shape="rect"></map><abbr title="t">A</abbr> <a href="http://x.ro/h">h</a>"#;
        std::fs::write(&f, src).unwrap();
        let occ = srcscan::scan_text(src, &f, SrcKind::Html);
        // <area> nu are </a>; nu trebuie să producă o propunere (și să nu mănânce textul de după)
        assert!(unlink(&occ, "http://x.ro/g").is_empty());
        assert_eq!(unlink(&occ, "http://x.ro/h").len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn backup_names_do_not_collide() {
        let d = std::env::temp_dir().join(format!("veriga-fix6-{}", std::process::id()));
        std::fs::create_dir_all(d.join("a")).unwrap();
        std::fs::create_dir_all(d.join("a__b")).unwrap();
        let (f1, f2) = (d.join("a/b__c.html"), d.join("a__b/c.html"));
        let s1 = r#"<a href="htp://x.ro/1">1</a>"#;
        let s2 = r#"<a href="htp://x.ro/2">2</a>"#;
        std::fs::write(&f1, s1).unwrap();
        std::fs::write(&f2, s2).unwrap();
        let mut ps = vec![];
        for (f, s, n) in [(&f1, s1, "1"), (&f2, s2, "2")] {
            let mut p = Vec::new();
            for o in srcscan::scan_text(s, f, SrcKind::Html) {
                p.push(Proposal {
                    id: format!("p{n}"),
                    file: f.clone(),
                    page: String::new(),
                    start: o.start,
                    end: o.end,
                    old_raw: o.url.clone(),
                    new_raw: format!("http://x.ro/{n}"),
                    old: o.url.clone(),
                    new: format!("http://x.ro/{n}"),
                    rule: Rule::SchemeTypo,
                    verified: true,
                    nav: false,
                    pages: 1,
                    note: String::new(),
                });
            }
            ps.extend(p);
        }
        let mut log = FixLog::default();
        let r = apply(
            &ps,
            &mut log,
            &ApplyOpts {
                backup_dir: d.join("bk"),
                run: "r".into(),
                allow_unverified: false,
            },
        );
        assert_eq!(r.applied.len(), 2);
        let b1 = log.entries[0].backup.clone().unwrap();
        let b2 = log.entries[1].backup.clone().unwrap();
        assert_ne!(b1, b2);
        assert_eq!(std::fs::read_to_string(b1).unwrap(), s1);
        assert_eq!(std::fs::read_to_string(b2).unwrap(), s2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_permissions_survive() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let d = std::env::temp_dir().join(format!("veriga-fix7-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let real = d.join("real.html");
        let link = d.join("link.html");
        let src = r#"<a href="htp://x.ro/1">1</a>"#;
        std::fs::write(&real, src).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&real, &link).unwrap();
        let o = &srcscan::scan_text(src, &link, SrcKind::Html)[0];
        let p = Proposal {
            id: "p".into(),
            file: link.clone(),
            page: String::new(),
            start: o.start,
            end: o.end,
            old_raw: o.url.clone(),
            new_raw: "http://x.ro/1".into(),
            old: o.url.clone(),
            new: "http://x.ro/1".into(),
            rule: Rule::SchemeTypo,
            verified: true,
            nav: false,
            pages: 1,
            note: String::new(),
        };
        let mut log = FixLog::default();
        assert_eq!(
            apply(
                &[p],
                &mut log,
                &ApplyOpts {
                    backup_dir: d.join("bk"),
                    run: "r".into(),
                    allow_unverified: false
                }
            )
            .applied
            .len(),
            1
        );
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "legătura simbolică a fost înlocuită"
        );
        assert_eq!(
            std::fs::read_to_string(&real).unwrap(),
            r#"<a href="http://x.ro/1">1</a>"#
        );
        assert_eq!(
            std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn corrupt_log_is_never_overwritten() {
        let d = std::env::temp_dir().join(format!("veriga-fix8-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("log.json");
        std::fs::write(&p, b"{ not json").unwrap();
        assert!(FixLog::load_checked(&p).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"{ not json");
        assert!(
            FixLog::load_checked(&d.join("missing.json"))
                .unwrap()
                .entries
                .is_empty()
        );
        let g = FixLog::lock(&p).unwrap();
        assert!(FixLog::lock(&p).is_err(), "a doua blocare trebuie refuzată");
        drop(g);
        assert!(FixLog::lock(&p).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn soft_redirect_and_resource_checks() {
        let ok = |code, fin: &str| Probe {
            code,
            final_url: fin.into(),
            hops: vec![],
            error: None,
        };
        // aceeași resursă (http→https, www, slash final): acceptat
        assert!(
            accept(
                Rule::Protocol,
                "http://a.ro/x",
                "https://a.ro/x",
                &ok(200, "https://www.a.ro/x/"),
                false
            )
            .0
        );
        // candidatul redirecționează în alt loc: refuzat
        assert!(
            !accept(
                Rule::Protocol,
                "http://a.ro/x",
                "https://a.ro/x",
                &ok(200, "https://a.ro/y"),
                false
            )
            .0
        );
        // aterizare pe pagina principală: refuzat
        assert!(
            !accept(
                Rule::Redirect,
                "http://a.ro/vechi",
                "https://a.ro/nou",
                &ok(200, "https://a.ro/"),
                false
            )
            .0
        );
        // pagină de autentificare: refuzat
        assert!(
            !accept(
                Rule::Redirect,
                "http://a.ro/vechi",
                "https://a.ro/nou",
                &ok(200, "https://a.ro/login"),
                false
            )
            .0
        );
        assert!(
            !accept(
                Rule::Tracking,
                "https://a.ro/x?utm_source=1",
                "https://a.ro/x",
                &ok(404, "https://a.ro/x"),
                false
            )
            .0
        );
    }

    #[test]
    fn injection_via_new_url_is_refused() {
        let d = std::env::temp_dir().join(format!("veriga-fix9-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("i.html");
        let src = "<a href='http://x.ro/v'>v</a>";
        std::fs::write(&f, src).unwrap();
        let occ = srcscan::scan_text(src, &f, SrcKind::Html);
        // manual: ghilimele / <script> / spații / backtick
        for bad in [
            "http://x.ro/n' onmouseover='alert(1)",
            "http://x.ro/\"><script>",
            "http://x.ro/a b",
            "http://x.ro/`x`",
            "http://x.ro/a\\b",
        ] {
            assert!(manual_replace(&occ, "http://x.ro/v", bad).is_err(), "{bad}");
        }
        assert!(manual_replace(&occ, "http://x.ro/v", "http://x.ro/nou?a=1&b=2").is_ok());
        // o propunere automată (ex. redirect cu Location ostil) este refuzată la aplicare
        let o = &occ[0];
        let mut p = Proposal {
            id: "p".into(),
            file: f.clone(),
            page: String::new(),
            start: o.start,
            end: o.end,
            old_raw: o.url.clone(),
            new_raw: "http://x.ro/n' onmouseover='alert(1)".into(),
            old: o.url.clone(),
            new: "x".into(),
            rule: Rule::Redirect,
            verified: true,
            nav: false,
            pages: 1,
            note: String::new(),
        };
        let mut log = FixLog::default();
        let opt = ApplyOpts {
            backup_dir: d.join("bk"),
            run: "r".into(),
            allow_unverified: true,
        };
        let r = apply(&[p.clone()], &mut log, &opt);
        assert_eq!(r.applied.len(), 0);
        assert!(r.skipped[0].1.contains("nesigure"), "{:?}", r.skipped);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
        // Markdown: parantezele sunt refuzate
        let m = d.join("i.md");
        std::fs::write(&m, "[a](http://x.ro/v)").unwrap();
        p.file = m.clone();
        p.new_raw = "http://x.ro/a)[evil](http://phish".into();
        let moc = srcscan::scan_text("[a](http://x.ro/v)", &m, SrcKind::Markdown);
        p.start = moc[0].start;
        p.end = moc[0].end;
        assert_eq!(apply(&[p], &mut log, &opt).applied.len(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_proposals_are_dropped_not_misapplied() {
        let d = std::env::temp_dir().join(format!("veriga-fix10-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("s.html");
        let src = r#"<a href="htp://x.ro/1">1</a>"#;
        std::fs::write(&f, src).unwrap();
        let o = srcscan::scan_text(src, &f, SrcKind::Html).remove(0);
        let p = Proposal {
            id: "p".into(),
            file: f.clone(),
            page: String::new(),
            start: o.start,
            end: o.end,
            old_raw: String::new(),
            new_raw: "http://x.ro/1".into(),
            old: o.url.clone(),
            new: "http://x.ro/1".into(),
            rule: Rule::SchemeTypo,
            verified: true,
            nav: false,
            pages: 1,
            note: String::new(),
        };
        // fișierul se editează între scanare și completare: pozițiile nu mai sunt de încredere
        std::fs::write(&f, format!("<p>text nou adăugat</p>{src}")).unwrap();
        let mut v = vec![p];
        fill_old_raw(&mut v);
        assert!(v.is_empty(), "propunerea învechită trebuia eliminată");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn csv_guard() {
        assert_eq!(csv_safe("=HYPERLINK(1)"), "'=HYPERLINK(1)");
        assert_eq!(csv_safe("ok"), "ok");
        assert_eq!(csv_safe("@x"), "'@x");
    }

    #[test]
    fn redirect_candidate_from_session() {
        use crate::model::{Hop, LinkKind, LinkResult};
        let mut r = LinkResult::new("http://a.ro/vechi", LinkKind::Page, 1, false);
        r.status = Status::Redirected;
        r.final_url = Some("https://a.ro/nou".into());
        r.chain = vec![Hop {
            url: "http://a.ro/vechi".into(),
            code: 301,
        }];
        let s = Session {
            version: 1,
            tool: "t".into(),
            started: String::new(),
            finished: String::new(),
            config: crate::Config::default(),
            results: vec![r],
            ignored: 0,
            truncated: false,
            cancelled: false,
        };
        let m: HashMap<String, &crate::model::LinkResult> =
            s.results.iter().map(|r| (norm_cmp(&r.url), r)).collect();
        let c = candidate("http://a.ro/vechi", Some(&m), &HashMap::new(), false).unwrap();
        assert_eq!(c, (Rule::Redirect, "https://a.ro/nou".to_string()));
    }
}
