//! Tipurile de date ale rezultatelor.

use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkKind {
    Start,
    Page,
    Image,
    Stylesheet,
    Script,
    Frame,
    Media,
    Object,
    Form,
    Icon,
    Link,
    Anchor,
    Mailto,
    Ftp,
    File,
    Sitemap,
    Other,
}

impl LinkKind {
    pub fn label(self, lang: &str) -> &'static str {
        let en = lang == "en";
        match (self, en) {
            (LinkKind::Start, false) => "Start",
            (LinkKind::Start, true) => "Start",
            (LinkKind::Page, false) => "Pagină",
            (LinkKind::Page, true) => "Page",
            (LinkKind::Image, false) => "Imagine",
            (LinkKind::Image, true) => "Image",
            (LinkKind::Stylesheet, false) => "Stil CSS",
            (LinkKind::Stylesheet, true) => "Stylesheet",
            (LinkKind::Script, false) => "Script",
            (LinkKind::Script, true) => "Script",
            (LinkKind::Frame, false) => "Cadru",
            (LinkKind::Frame, true) => "Frame",
            (LinkKind::Media, false) => "Media",
            (LinkKind::Media, true) => "Media",
            (LinkKind::Object, false) => "Obiect",
            (LinkKind::Object, true) => "Object",
            (LinkKind::Form, false) => "Formular",
            (LinkKind::Form, true) => "Form",
            (LinkKind::Icon, false) => "Pictogramă",
            (LinkKind::Icon, true) => "Icon",
            (LinkKind::Link, false) => "Legătură <link>",
            (LinkKind::Link, true) => "<link>",
            (LinkKind::Anchor, false) => "Ancoră",
            (LinkKind::Anchor, true) => "Anchor",
            (LinkKind::Mailto, _) => "E-mail",
            (LinkKind::Ftp, _) => "FTP",
            (LinkKind::File, false) => "Fișier",
            (LinkKind::File, true) => "File",
            (LinkKind::Sitemap, _) => "Sitemap",
            (LinkKind::Other, false) => "Altele",
            (LinkKind::Other, true) => "Other",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Ok,
    Redirected,
    NotFound,
    Forbidden,
    ClientError,
    ServerError,
    Timeout,
    NoConnection,
    DnsFailure,
    TlsError,
    TooManyRedirects,
    BadRedirect,
    InvalidUrl,
    InvalidEmail,
    LocalMissing,
    FragmentMissing,
    FtpError,
    RobotsBlocked,
    Skipped,
    Orphan,
}

impl Status {
    /// Eroare „dură” (link rupt).
    pub fn is_error(self) -> bool {
        use Status::*;
        matches!(
            self,
            NotFound
                | Forbidden
                | ClientError
                | ServerError
                | Timeout
                | NoConnection
                | DnsFailure
                | TlsError
                | TooManyRedirects
                | BadRedirect
                | InvalidUrl
                | InvalidEmail
                | LocalMissing
                | FragmentMissing
                | FtpError
        )
    }

    /// 401/403 tratat ca „de verificat manual” (nu rupt).
    pub fn needs_review(self, cfg: &Config) -> bool {
        self == Status::Forbidden && cfg.review_forbidden
    }

    pub fn is_broken(self, cfg: &Config) -> bool {
        if self == Status::Forbidden && cfg.review_forbidden {
            // 401/403: de obicei un bot blocat, nu un link rupt → „merită o a doua privire”.
            return false;
        }
        self.is_error() || (cfg.treat_redirects_as_errors && self == Status::Redirected)
    }

    pub fn label(self, lang: &str) -> &'static str {
        let en = lang == "en";
        use Status::*;
        match (self, en) {
            (Pending, false) => "În așteptare",
            (Pending, true) => "Pending",
            (Ok, false) => "OK",
            (Ok, true) => "OK",
            (Redirected, false) => "Redirecționat",
            (Redirected, true) => "Redirected",
            (NotFound, false) => "Nu există (404)",
            (NotFound, true) => "Not found (404)",
            (Forbidden, false) => "Acces interzis",
            (Forbidden, true) => "Forbidden",
            (ClientError, false) => "Eroare client (4xx)",
            (ClientError, true) => "Client error (4xx)",
            (ServerError, false) => "Eroare server (5xx)",
            (ServerError, true) => "Server error (5xx)",
            (Timeout, false) => "Timp depășit",
            (Timeout, true) => "Timeout",
            (NoConnection, false) => "Fără conexiune",
            (NoConnection, true) => "No connection",
            (DnsFailure, false) => "Gazdă inexistentă (DNS)",
            (DnsFailure, true) => "DNS failure",
            (TlsError, false) => "Eroare TLS/certificat",
            (TlsError, true) => "TLS/certificate error",
            (TooManyRedirects, false) => "Prea multe redirecționări",
            (TooManyRedirects, true) => "Too many redirects",
            (BadRedirect, false) => "Redirecționare invalidă",
            (BadRedirect, true) => "Bad redirect",
            (InvalidUrl, false) => "URL invalid",
            (InvalidUrl, true) => "Invalid URL",
            (InvalidEmail, false) => "E-mail invalid",
            (InvalidEmail, true) => "Invalid e-mail",
            (LocalMissing, false) => "Fișier local lipsă",
            (LocalMissing, true) => "Local file missing",
            (FragmentMissing, false) => "Ancoră lipsă",
            (FragmentMissing, true) => "Anchor missing",
            (FtpError, false) => "Eroare FTP",
            (FtpError, true) => "FTP error",
            (RobotsBlocked, false) => "Blocat de robots.txt",
            (RobotsBlocked, true) => "Blocked by robots.txt",
            (Skipped, false) => "Omis",
            (Skipped, true) => "Skipped",
            (Orphan, false) => "Fișier orfan",
            (Orphan, true) => "Orphan file",
        }
    }
}

/// Pagina (sau resursa) care conține un link.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Parent {
    pub url: String,
    pub text: String,
    pub kind: LinkKind,
}

/// O etapă dintr-un lanț de redirecționări.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hop {
    pub url: String,
    pub code: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkResult {
    pub url: String,
    pub final_url: Option<String>,
    pub http_status: Option<u16>,
    pub status: Status,
    pub message: String,
    pub kind: LinkKind,
    pub internal: bool,
    pub depth: u32,
    pub content_type: Option<String>,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
    pub title: Option<String>,
    pub elapsed_ms: u64,
    pub chain: Vec<Hop>,
    pub parents: Vec<Parent>,
    pub parsed: bool,
    pub links_found: u32,
    /// Primul `<h1>` al paginii (pentru detectarea derivei de conținut).
    #[serde(default)]
    pub h1: Option<String>,
}

impl LinkResult {
    pub fn new(url: &str, kind: LinkKind, depth: u32, internal: bool) -> Self {
        Self {
            url: url.to_string(),
            final_url: None,
            http_status: None,
            status: Status::Pending,
            message: String::new(),
            kind,
            internal,
            depth,
            content_type: None,
            size: None,
            last_modified: None,
            title: None,
            elapsed_ms: 0,
            chain: vec![],
            parents: vec![],
            parsed: false,
            links_found: 0,
            h1: None,
        }
    }

    pub fn is_page(&self) -> bool {
        self.content_type
            .as_deref()
            .map(|c| c.contains("html"))
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub checked: usize,
    pub queued: usize,
    pub in_flight: usize,
    pub broken: usize,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug)]
pub enum Event {
    Started,
    Checked(Box<LinkResult>),
    Progress(Progress),
    Phase(String),
    Finished,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub tool: String,
    pub started: String,
    pub finished: String,
    pub config: Config,
    pub results: Vec<LinkResult>,
    pub ignored: usize,
    pub truncated: bool,
    pub cancelled: bool,
}

impl Session {
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let mut s = self.clone();
        s.config.basic_auth = None;
        // scriere atomică: nu lăsăm o sesiune pe jumătate scrisă la întrerupere
        let tmp = path.with_extension(format!("veriga.{}.tmp", std::process::id()));
        std::fs::write(&tmp, serde_json::to_vec(&s)?)?;
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(serde_json::from_slice(&data)?)
    }

    pub fn broken(&self) -> impl Iterator<Item = &LinkResult> {
        let cfg = &self.config;
        self.results.iter().filter(move |r| r.status.is_broken(cfg))
    }

    pub fn broken_count(&self) -> usize {
        self.broken().count()
    }
}
