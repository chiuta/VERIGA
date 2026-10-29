//! Configurația unei verificări. Toate câmpurile au valori implicite rezonabile
//! și sunt serializabile (se salvează în sesiune, fără parole).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Căutarea fișierelor orfane: fișiere din `dir` la care nu duce niciun link.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OrphanConfig {
    /// Directorul local al site-ului (rădăcina documentelor).
    pub dir: PathBuf,
    /// URL-ul corespunzător rădăcinii directorului (ex. `https://exemplu.ro/`).
    /// Dacă lipsește, se folosește primul URL de start (rădăcina sa).
    pub base_url: Option<String>,
    /// Expresii regulate pe calea relativă (cu `/`) a fișierelor ignorate.
    pub ignore: Vec<String>,
}

impl Default for OrphanConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::new(),
            base_url: None,
            ignore: vec![
                r"(^|/)\.".into(),
                r"(?i)(^|/)(thumbs\.db|desktop\.ini|\.ds_store)$".into(),
                r"(?i)(^|/)(robots\.txt|favicon\.ico|sitemap.*\.xml)$".into(),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// URL-uri, căi de fișiere sau directoare locale de la care începe verificarea.
    pub start: Vec<String>,
    /// Număr maxim de verificări simultane (Xenu: „threads”).
    pub threads: usize,
    /// Conexiuni simultane maxime către același server.
    pub per_host: usize,
    pub timeout_secs: u64,
    /// Reîncercări la timeout / eroare de conexiune / 502-504 / 429.
    pub retries: u32,
    /// Adâncimea maximă la care se *urmăresc* paginile (linkurile de pe ultima
    /// pagină sunt totuși verificate).
    pub max_depth: Option<u32>,
    pub max_urls: usize,
    /// Protecție la capcane (calendare, paginări fără fund): câte pagini cu același
    /// șablon de cale (cifrele înlocuite cu „N”) se parcurg. 0 = fără limită.
    pub max_per_pattern: usize,
    pub max_redirects: usize,
    /// Verifică și linkurile externe (fără a le parcurge).
    pub check_external: bool,
    /// Nu parcurge pagini din afara directorului de start.
    pub stay_in_dir: bool,
    /// Verifică imagini, CSS, JS, media, obiecte etc. (nu doar pagini).
    pub check_resources: bool,
    pub check_forms: bool,
    pub check_mailto: bool,
    pub check_ftp: bool,
    /// Verifică ancorele `#fragment` din paginile HTML parcurse.
    pub check_fragments: bool,
    /// Tratează redirecționările ca erori.
    pub treat_redirects_as_errors: bool,
    /// 401/403 = „merită o a doua privire”, nu link rupt (implicit activ).
    pub review_forbidden: bool,
    /// Regex: URL-urile care se potrivesc nu sunt verificate deloc.
    pub exclude: Vec<String>,
    /// Regex: URL-uri verificate, dar al căror conținut nu este parcurs.
    pub no_follow: Vec<String>,
    /// Regex: dacă nu e gol, se parcurg doar paginile care se potrivesc.
    pub include_only: Vec<String>,
    /// Gazde suplimentare considerate interne.
    pub internal_hosts: Vec<String>,
    /// Coduri HTTP considerate OK (ex. 999 la LinkedIn).
    pub ignore_status: Vec<u16>,
    pub user_agent: String,
    pub respect_robots: bool,
    /// Pornește și din sitemap.xml (din robots.txt sau /sitemap.xml).
    pub seed_sitemaps: bool,
    pub ignore_tls_errors: bool,
    pub cookies: bool,
    /// Nu contacta adrese private/loopback/link-local (anti-SSRF) când punctul de start e public.
    pub block_private: bool,
    pub proxy: Option<String>,
    /// Utilizator și parolă (Basic) trimise doar către gazdele interne.
    /// Nu se salvează în sesiuni.
    pub basic_auth: Option<(String, String)>,
    pub headers: Vec<(String, String)>,
    /// Pauză minimă între cererile către același server.
    pub delay_ms: u64,
    pub max_body_bytes: usize,
    /// Încearcă HEAD înainte de GET pentru resurse care nu se parcurg.
    pub head_first: bool,
    /// Pagini mai lente decât acest prag apar ca avertisment (ms).
    pub slow_ms: u64,
    pub orphan: Option<OrphanConfig>,
    /// Limba etichetelor din rapoarte: "ro" sau "en".
    pub lang: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            start: vec![],
            threads: 30,
            per_host: 8,
            timeout_secs: 30,
            retries: 2,
            max_depth: None,
            max_urls: 1_000_000,
            max_per_pattern: 5_000,
            max_redirects: 10,
            check_external: true,
            stay_in_dir: false,
            check_resources: true,
            check_forms: false,
            check_mailto: true,
            check_ftp: true,
            check_fragments: false,
            treat_redirects_as_errors: false,
            review_forbidden: true,
            exclude: vec![],
            no_follow: vec![],
            include_only: vec![],
            internal_hosts: vec![],
            ignore_status: vec![],
            user_agent: format!(
                "Veriga/{} (+link-checker; clean-room)",
                env!("CARGO_PKG_VERSION")
            ),
            respect_robots: true,
            seed_sitemaps: false,
            ignore_tls_errors: false,
            cookies: true,
            block_private: true,
            proxy: None,
            basic_auth: None,
            headers: vec![],
            delay_ms: 0,
            max_body_bytes: 8 * 1024 * 1024,
            head_first: true,
            slow_ms: 3000,
            orphan: None,
            lang: "ro".into(),
        }
    }
}

impl Config {
    pub fn with_start<S: Into<String>>(url: S) -> Self {
        Self {
            start: vec![url.into()],
            ..Self::default()
        }
    }

    /// Jetonul de produs folosit la potrivirea în robots.txt.
    pub fn robots_token(&self) -> String {
        self.user_agent
            .split(['/', ' '])
            .next()
            .unwrap_or("veriga")
            .to_ascii_lowercase()
    }
}
