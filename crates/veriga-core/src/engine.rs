//! Coordonatorul verificării: coadă pe gazde, deduplicare, descoperire de linkuri,
//! ancore, fișiere orfane, retestarea linkurilor rupte.

use crate::check::{Outcome, Shared, Task, check};
use crate::config::Config;
use crate::extract::RawLink;
use crate::model::*;
use crate::urlutil::*;
use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinSet;
use url::Url;

/// Comenzi de control: oprire și pauză.
#[derive(Default)]
pub struct Control {
    cancel: AtomicBool,
    pause: AtomicBool,
}

impl Control {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub fn pause(&self) {
        self.pause.store(true, Ordering::SeqCst);
    }
    pub fn resume(&self) {
        self.pause.store(false, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
    pub fn is_paused(&self) -> bool {
        self.pause.load(Ordering::SeqCst)
    }
}

fn compile_list(label: &str, pats: &[String]) -> Result<Vec<Regex>> {
    pats.iter()
        .map(|p| {
            Regex::new(p).with_context(|| format!("expresie regulată invalidă în {label}: {p}"))
        })
        .collect()
}

struct Coord {
    cfg: Arc<Config>,
    sh: Arc<Shared>,
    results: Vec<LinkResult>,
    index: HashMap<String, usize>,
    queues: HashMap<String, VecDeque<Task>>,
    order: VecDeque<String>,
    host_inflight: HashMap<String, usize>,
    inflight: usize,
    queued: usize,
    fragments: Vec<(String, String, Parent)>,
    anchors: HashMap<String, HashSet<String>>,
    ignored: usize,
    truncated: bool,
    excl: Vec<Regex>,
    nofollow: Vec<Regex>,
    only: Vec<Regex>,
    internal_hosts: HashSet<String>,
    file_roots: Vec<String>,
    dir_prefixes: Vec<(String, String)>,
    discover: bool,
    tx: Option<UnboundedSender<Event>>,
    started: Instant,
    done: usize,
    broken: usize,
    last_progress: Instant,
    start_urls: Vec<Url>,
}

impl Coord {
    fn new(cfg: Arc<Config>, sh: Arc<Shared>, tx: Option<UnboundedSender<Event>>) -> Result<Self> {
        Ok(Self {
            excl: compile_list("--exclude", &cfg.exclude)?,
            nofollow: compile_list("--no-follow", &cfg.no_follow)?,
            only: compile_list("--only", &cfg.include_only)?,
            internal_hosts: cfg.internal_hosts.iter().map(|h| site_key(h)).collect(),
            cfg,
            sh,
            results: vec![],
            index: HashMap::new(),
            queues: HashMap::new(),
            order: VecDeque::new(),
            host_inflight: HashMap::new(),
            inflight: 0,
            queued: 0,
            fragments: vec![],
            anchors: HashMap::new(),
            ignored: 0,
            truncated: false,
            file_roots: vec![],
            dir_prefixes: vec![],
            discover: true,
            tx,
            started: Instant::now(),
            done: 0,
            broken: 0,
            last_progress: Instant::now() - Duration::from_secs(1),
            start_urls: vec![],
        })
    }

    fn emit(&self, ev: Event) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(ev);
        }
    }

    fn emit_progress(&mut self, force: bool) {
        if !force && self.last_progress.elapsed() < Duration::from_millis(120) {
            return;
        }
        self.last_progress = Instant::now();
        self.emit(Event::Progress(Progress {
            checked: self.done,
            queued: self.queued,
            in_flight: self.inflight,
            broken: self.broken,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        }));
    }

    fn register_start(&mut self, u: &Url) {
        match u.scheme() {
            "file" => {
                let p = dir_prefix(u);
                if !self.file_roots.contains(&p) {
                    self.file_roots.push(p);
                }
            }
            _ => {
                if let Some(h) = u.host_str() {
                    self.internal_hosts.insert(site_key(h));
                    self.dir_prefixes.push((site_key(h), dir_prefix(u)));
                }
            }
        }
        self.start_urls.push(u.clone());
    }

    fn is_internal(&self, u: &Url) -> bool {
        match u.scheme() {
            "http" | "https" => u
                .host_str()
                .map(|h| self.internal_hosts.contains(&site_key(h)))
                .unwrap_or(false),
            "file" => self
                .file_roots
                .iter()
                .any(|r| u.path().starts_with(r.as_str())),
            _ => false,
        }
    }

    fn in_start_dir(&self, u: &Url) -> bool {
        if u.scheme() == "file" {
            return true;
        }
        let Some(h) = u.host_str() else { return false };
        let h = site_key(h);
        self.dir_prefixes
            .iter()
            .any(|(sh, p)| *sh == h && u.path().starts_with(p.as_str()))
    }

    /// Decide dacă un URL se verifică și dacă i se parcurge conținutul.
    fn plan(&self, url: &Url, kind: LinkKind, depth: u32) -> Option<(bool, bool)> {
        let cfg = &self.cfg;
        match url.scheme() {
            "http" | "https" | "file" => {}
            "ftp" => {
                if !cfg.check_ftp {
                    return None;
                }
            }
            "mailto" => {
                if !cfg.check_mailto {
                    return None;
                }
            }
            _ => return None,
        }
        let internal = self.is_internal(url);
        if !internal && !cfg.check_external && kind != LinkKind::Start {
            return None;
        }
        if !cfg.check_resources
            && !matches!(kind, LinkKind::Start | LinkKind::Page | LinkKind::Frame)
        {
            return None;
        }
        let key = url.as_str();
        let mut parse = internal && matches!(url.scheme(), "http" | "https" | "file");
        parse &= match kind {
            LinkKind::Start => true,
            LinkKind::Page | LinkKind::Frame => looks_like_html(url),
            LinkKind::Stylesheet => true,
            _ => false,
        };
        if parse && kind != LinkKind::Start {
            if let Some(m) = cfg.max_depth
                && depth > m
            {
                parse = false;
            }
            if self.nofollow.iter().any(|r| r.is_match(key)) {
                parse = false;
            }
            if !self.only.is_empty() && !self.only.iter().any(|r| r.is_match(key)) {
                parse = false;
            }
            if cfg.stay_in_dir && !self.in_start_dir(url) {
                parse = false;
            }
        }
        Some((internal, parse))
    }

    fn push_task(&mut self, t: Task) {
        let host = t.host.clone();
        if !self.queues.contains_key(&host) {
            self.order.push_back(host.clone());
        }
        self.queues.entry(host).or_default().push_back(t);
        self.queued += 1;
    }

    fn add_start(&mut self, url: Url) {
        let key = normalize(&url);
        if self.index.contains_key(&key) {
            return;
        }
        let (internal, parse) = self.plan(&url, LinkKind::Start, 0).unwrap_or((true, true));
        let idx = self.results.len();
        self.results
            .push(LinkResult::new(url.as_str(), LinkKind::Start, 0, internal));
        self.index.insert(key, idx);
        let host = host_key(&url);
        self.push_task(Task {
            url,
            kind: LinkKind::Start,
            depth: 0,
            parse,
            internal,
            idx,
            host,
        });
    }

    fn add_parent(&mut self, idx: usize, p: Parent) {
        let r = &mut self.results[idx];
        if r.parents.len() < 100 && !r.parents.iter().any(|x| x.url == p.url && x.text == p.text) {
            r.parents.push(p);
        }
    }

    fn add_link(&mut self, l: RawLink, parent: &Parent, parent_depth: u32) {
        let mut url = l.url;
        let frag = url.fragment().map(|f| f.to_string());
        url.set_fragment(None);
        let key = url.to_string();

        if let Some(f) = frag
            && self.cfg.check_fragments
            && !f.is_empty()
            && matches!(url.scheme(), "http" | "https" | "file")
            && matches!(l.kind, LinkKind::Page | LinkKind::Frame)
        {
            let p = Parent {
                url: parent.url.clone(),
                text: l.text.clone(),
                kind: l.kind,
            };
            self.fragments.push((key.clone(), f, p));
        }
        if key == parent.url {
            return; // auto-referință (ancoră în aceeași pagină)
        }
        let p = Parent {
            url: parent.url.clone(),
            text: l.text.clone(),
            kind: l.kind,
        };
        if self.excl.iter().any(|r| r.is_match(&key)) {
            self.ignored += 1;
            return;
        }
        if let Some(&i) = self.index.get(&key) {
            self.add_parent(i, p);
            return;
        }
        let depth = parent_depth + 1;
        let Some((internal, parse)) = self.plan(&url, l.kind, depth) else {
            return;
        };
        if self.results.len() >= self.cfg.max_urls {
            self.truncated = true;
            return;
        }
        let idx = self.results.len();
        let mut r = LinkResult::new(&key, l.kind, depth, internal);
        r.parents.push(p);
        self.results.push(r);
        self.index.insert(key, idx);
        let host = host_key(&url);
        self.push_task(Task {
            url,
            kind: l.kind,
            depth,
            parse,
            internal,
            idx,
            host,
        });
    }

    fn add_bad(&mut self, raw: String, kind: LinkKind, msg: String, parent: &Parent, depth: u32) {
        let key = format!("invalid:{raw}");
        let p = Parent {
            url: parent.url.clone(),
            text: String::new(),
            kind,
        };
        if let Some(&i) = self.index.get(&key) {
            self.add_parent(i, p);
            return;
        }
        let idx = self.results.len();
        let mut r = LinkResult::new(&raw, kind, depth + 1, true);
        r.status = Status::InvalidUrl;
        r.message = msg;
        r.parents.push(p);
        self.broken += 1;
        self.emit(Event::Checked(Box::new(r.clone())));
        self.results.push(r);
        self.index.insert(key, idx);
    }

    fn next_task(&mut self) -> Option<Task> {
        let n = self.order.len();
        let limit = self.cfg.per_host.max(1);
        for _ in 0..n {
            let host = self.order.pop_front()?;
            let inflight = *self.host_inflight.get(&host).unwrap_or(&0);
            match self.queues.get_mut(&host) {
                Some(q) if !q.is_empty() => {
                    if inflight < limit {
                        let t = q.pop_front().unwrap();
                        if q.is_empty() {
                            self.queues.remove(&host);
                        } else {
                            self.order.push_back(host.clone());
                        }
                        *self.host_inflight.entry(host).or_insert(0) += 1;
                        self.queued -= 1;
                        return Some(t);
                    } else {
                        self.order.push_back(host);
                    }
                }
                _ => {
                    self.queues.remove(&host);
                }
            }
        }
        None
    }

    fn handle(&mut self, o: Outcome) {
        self.inflight = self.inflight.saturating_sub(1);
        if let Some(c) = self.host_inflight.get_mut(&o.host) {
            *c = c.saturating_sub(1);
        }
        let idx = o.idx;
        let mut res = o.result;
        res.parents = std::mem::take(&mut self.results[idx].parents);
        if let Some(a) = o.anchors {
            if let Some(f) = res.final_url.as_ref().and_then(|u| Url::parse(u).ok()) {
                self.anchors.insert(normalize(&f), a.clone());
            }
            if let Ok(u) = Url::parse(&res.url) {
                self.anchors.insert(normalize(&u), a);
            }
        }
        self.done += 1;
        if res.status.is_broken(&self.cfg) {
            self.broken += 1;
        }
        self.emit(Event::Checked(Box::new(res.clone())));
        let depth = res.depth;
        let parent = Parent {
            url: res.url.clone(),
            text: String::new(),
            kind: res.kind,
        };
        self.results[idx] = res;
        if self.discover {
            for l in o.links {
                self.add_link(l, &parent, depth);
            }
            for (raw, kind, msg) in o.bad {
                self.add_bad(raw, kind, msg, &parent, depth);
            }
        }
        self.emit_progress(false);
    }

    async fn drive(&mut self, ctl: &Control) -> bool {
        let mut set: JoinSet<Outcome> = JoinSet::new();
        loop {
            if ctl.is_cancelled() {
                set.abort_all();
                while set.join_next().await.is_some() {}
                return true;
            }
            if !ctl.is_paused() {
                while self.inflight < self.cfg.threads.max(1) {
                    match self.next_task() {
                        Some(t) => {
                            self.inflight += 1;
                            set.spawn(check(self.sh.clone(), t));
                        }
                        None => break,
                    }
                }
            }
            if self.inflight == 0 && self.queued == 0 {
                return false;
            }
            tokio::select! {
                r = set.join_next(), if !set.is_empty() => {
                    match r {
                        Some(Ok(o)) => self.handle(o),
                        Some(Err(_)) => { self.inflight = self.inflight.saturating_sub(1); }
                        None => {}
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(80)) => {
                    self.emit_progress(false);
                }
            }
        }
    }

    async fn seed_sitemaps(&mut self) {
        fn unescape(s: &str) -> String {
            s.replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
        }
        let loc_re = Regex::new(r"(?is)<loc>\s*([^<]+?)\s*</loc>").unwrap();
        let token = self.cfg.robots_token();
        let mut origins: HashSet<String> = HashSet::new();
        let roots = self.start_urls.clone();
        for u in roots {
            if !matches!(u.scheme(), "http" | "https") {
                continue;
            }
            let origin = format!(
                "{}://{}",
                u.scheme(),
                host_key(&u)
                    .trim_end_matches(":443")
                    .trim_end_matches(":80")
            );
            if !origins.insert(origin.clone()) {
                continue;
            }
            let rules = self.sh.robots.get(&self.sh.client, &u, &token).await;
            let mut sms = rules.sitemaps.clone();
            if sms.is_empty() {
                sms.push(format!("{origin}/sitemap.xml"));
            }
            let mut stack: Vec<(String, u32)> = sms.into_iter().map(|s| (s, 0)).collect();
            let mut fetched = 0;
            while let Some((sm, d)) = stack.pop() {
                if fetched >= 20 {
                    break;
                }
                fetched += 1;
                let Ok(resp) = self.sh.client.get(&sm).send().await else {
                    continue;
                };
                if !resp.status().is_success() {
                    continue;
                }
                let Ok(text) = resp.text().await else {
                    continue;
                };
                let is_index = text.contains("<sitemapindex");
                let parent = Parent {
                    url: sm.clone(),
                    text: "sitemap".into(),
                    kind: LinkKind::Sitemap,
                };
                for c in loc_re.captures_iter(&text) {
                    let l = unescape(&c[1]);
                    if is_index {
                        if d < 2 {
                            stack.push((l, d + 1));
                        }
                    } else if let Ok(url) = Url::parse(&l) {
                        self.add_link(
                            RawLink {
                                url,
                                kind: LinkKind::Page,
                                text: String::new(),
                            },
                            &parent,
                            0,
                        );
                    }
                }
            }
        }
    }

    fn check_fragments(&mut self) {
        let frs = std::mem::take(&mut self.fragments);
        for (page, frag, parent) in frs {
            if frag.is_empty() || frag.eq_ignore_ascii_case("top") {
                continue;
            }
            let Some(set) = self.anchors.get(&page) else {
                continue;
            };
            let decoded = percent_encoding::percent_decode_str(&frag)
                .decode_utf8_lossy()
                .to_string();
            if set.contains(&frag) || set.contains(&decoded) {
                continue;
            }
            let full = format!("{page}#{frag}");
            let key = format!("frag:{full}");
            if let Some(&i) = self.index.get(&key) {
                self.add_parent(i, parent);
                continue;
            }
            let depth = self
                .index
                .get(&page)
                .map(|&i| self.results[i].depth)
                .unwrap_or(1);
            let mut r = LinkResult::new(&full, LinkKind::Anchor, depth, true);
            r.status = Status::FragmentMissing;
            r.message = format!("ancora #{decoded} nu există în pagina țintă");
            r.parents.push(parent);
            let idx = self.results.len();
            self.broken += 1;
            self.emit(Event::Checked(Box::new(r.clone())));
            self.results.push(r);
            self.index.insert(key, idx);
        }
    }

    fn find_orphans(&mut self) -> Result<()> {
        let Some(oc) = self.cfg.orphan.clone() else {
            return Ok(());
        };
        if oc.dir.as_os_str().is_empty() {
            return Ok(());
        }
        let dir = std::fs::canonicalize(&oc.dir)
            .with_context(|| format!("director orfani inexistent: {}", oc.dir.display()))?;
        let dir_str = dir.to_string_lossy().to_string();
        let dir = std::path::PathBuf::from(dir_str.trim_start_matches(r"\\?\"));
        let base: Url = match &oc.base_url {
            Some(b) => {
                let mut u = parse_start(b)
                    .or_else(|_| Url::parse(b))
                    .map_err(|e| anyhow!("{e}"))?;
                if !u.path().ends_with('/') {
                    let p = format!("{}/", u.path());
                    u.set_path(&p);
                }
                u.set_query(None);
                u.set_fragment(None);
                u
            }
            None => {
                let first = self
                    .start_urls
                    .first()
                    .ok_or_else(|| anyhow!("fără URL de start"))?;
                if first.scheme() == "file" {
                    Url::from_directory_path(&dir).map_err(|_| anyhow!("director invalid"))?
                } else {
                    let mut u = first.clone();
                    let p = dir_prefix(first);
                    u.set_path(&p);
                    u.set_query(None);
                    u.set_fragment(None);
                    u
                }
            }
        };
        let ignore = compile_list("orphan.ignore", &oc.ignore)?;

        let mut seen: HashSet<String> = HashSet::new();
        for r in &self.results {
            if let Ok(u) = Url::parse(&r.url) {
                let mut n = u.clone();
                n.set_fragment(None);
                seen.insert(n.to_string());
                n.set_query(None);
                seen.insert(n.to_string());
            }
            if let Some(f) = r.final_url.as_ref().and_then(|f| Url::parse(f).ok()) {
                let mut n = f;
                n.set_fragment(None);
                n.set_query(None);
                seen.insert(n.to_string());
            }
        }

        const SET: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
            .remove(b'/')
            .remove(b'-')
            .remove(b'_')
            .remove(b'.')
            .remove(b'~');
        let mut found = vec![];
        for entry in walkdir::WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(rel) = entry.path().strip_prefix(&dir) else {
                continue;
            };
            let rel_s = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            if ignore.iter().any(|r| r.is_match(&rel_s)) {
                continue;
            }
            let enc = percent_encoding::utf8_percent_encode(&rel_s, SET).to_string();
            let Ok(cand) = base.join(&enc) else { continue };
            let mut ok = seen.contains(cand.as_str());
            let name = rel
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !ok
                && is_index_name(&name)
                && let Ok(d) = cand.join("./")
            {
                ok = seen.contains(d.as_str());
            }
            if !ok {
                found.push((cand, entry.path().to_path_buf()));
            }
        }
        found.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (cand, path) in found {
            let mut r = LinkResult::new(cand.as_str(), LinkKind::File, 0, true);
            r.status = Status::Orphan;
            r.message = path.display().to_string();
            r.size = std::fs::metadata(&path).ok().map(|m| m.len());
            r.content_type = mime_guess::from_path(&path)
                .first()
                .map(|m| m.essence_str().to_string());
            self.emit(Event::Checked(Box::new(r.clone())));
            self.results.push(r);
        }
        Ok(())
    }

    fn mark_unchecked(&mut self) {
        for r in &mut self.results {
            if r.status == Status::Pending {
                r.status = Status::Skipped;
                r.message = "neverificat (oprit înainte de finalizare)".into();
            }
        }
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Rulează o verificare completă.
pub async fn run(
    cfg: Config,
    ctl: Arc<Control>,
    tx: Option<UnboundedSender<Event>>,
) -> Result<Session> {
    if cfg.start.is_empty() {
        bail!("nu a fost indicat niciun URL, fișier sau director de start");
    }
    let cfg = Arc::new(cfg);
    let sh = Arc::new(Shared::new(cfg.clone())?);
    let mut co = Coord::new(cfg.clone(), sh, tx)?;
    let started = now_rfc3339();
    co.emit(Event::Started);

    let mut roots = vec![];
    for s in &cfg.start {
        let u = parse_start(s)?;
        co.register_start(&u);
        roots.push(u);
    }
    for u in roots {
        co.add_start(u);
    }
    if cfg.seed_sitemaps {
        co.emit(Event::Phase("sitemap".into()));
        co.seed_sitemaps().await;
    }
    co.emit(Event::Phase("verificare".into()));
    let cancelled = co.drive(&ctl).await;
    co.emit_progress(true);

    if !cancelled {
        co.emit(Event::Phase("ancore și orfani".into()));
        if cfg.check_fragments {
            co.check_fragments();
        }
        co.find_orphans()?;
    }
    co.mark_unchecked();
    co.emit(Event::Finished);

    let mut config = (*cfg).clone();
    config.basic_auth = None;
    Ok(Session {
        version: 1,
        tool: "veriga".into(),
        started,
        finished: now_rfc3339(),
        config,
        results: co.results,
        ignored: co.ignored,
        truncated: co.truncated,
        cancelled,
    })
}

/// Reverifică doar linkurile rupte dintr-o sesiune salvată.
pub async fn recheck(
    session: Session,
    ctl: Arc<Control>,
    tx: Option<UnboundedSender<Event>>,
) -> Result<Session> {
    let cfg = Arc::new(session.config.clone());
    let sh = Arc::new(Shared::new(cfg.clone())?);
    let mut co = Coord::new(cfg.clone(), sh, tx)?;
    co.discover = false;
    co.emit(Event::Started);
    co.results = session.results;
    for (i, r) in co.results.iter().enumerate() {
        co.index.insert(r.url.clone(), i);
    }
    for s in &cfg.start {
        if let Ok(u) = parse_start(s) {
            co.register_start(&u);
        }
    }
    let todo: Vec<usize> = co
        .results
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.status.is_broken(&cfg)
                && !matches!(r.kind, LinkKind::Anchor | LinkKind::File)
                && Url::parse(&r.url).is_ok()
        })
        .map(|(i, _)| i)
        .collect();
    for i in todo {
        let url = Url::parse(&co.results[i].url)?;
        let internal = co.is_internal(&url);
        let kind = co.results[i].kind;
        let depth = co.results[i].depth;
        co.results[i].status = Status::Pending;
        let host = host_key(&url);
        co.push_task(Task {
            url,
            kind,
            depth,
            parse: false,
            internal,
            idx: i,
            host,
        });
    }
    let started = session.started;
    let cancelled = co.drive(&ctl).await;
    co.emit_progress(true);
    co.mark_unchecked();
    co.emit(Event::Finished);
    Ok(Session {
        version: 1,
        tool: "veriga".into(),
        started,
        finished: now_rfc3339(),
        config: session.config,
        results: co.results,
        ignored: session.ignored,
        truncated: session.truncated,
        cancelled,
    })
}
