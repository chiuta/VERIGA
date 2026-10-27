//! VERIGA — verificator de legături în linie de comandă.

mod guardian;

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use veriga_core::report;
use veriga_core::{Config, Control, Event, OrphanConfig, Session};

#[derive(Parser, Debug)]
#[command(
    name = "veriga",
    version,
    about = "VERIGA — verificator de legături (clean-room), pentru site-uri și directoare locale",
    long_about = "Parcurge un site (sau un director local), verifică toate legăturile — pagini, imagini, CSS, \
scripturi, cadre, media, obiecte, e-mail, FTP, ancore — și produce rapoarte HTML/CSV/TSV/JSON/text/sitemap.\n\n\
Exemple:\n  veriga https://exemplu.ro -o raport.html -o rupte.csv\n  veriga ./site --orphans ./site\n  veriga --recheck sesiune.veriga -o raport.html"
)]
struct Cli {
    /// URL-uri, fișiere HTML sau directoare locale de la care începe verificarea
    start: Vec<String>,

    /// Număr de verificări simultane
    #[arg(short = 't', long, default_value_t = 30)]
    threads: usize,
    /// Conexiuni simultane maxime către aceeași gazdă
    #[arg(long, default_value_t = 8)]
    per_host: usize,
    /// Timp maxim per cerere (secunde)
    #[arg(long, default_value_t = 30)]
    timeout: u64,
    /// Reîncercări la erori trecătoare
    #[arg(long, default_value_t = 2)]
    retries: u32,
    /// Adâncimea maximă a parcurgerii
    #[arg(short = 'd', long)]
    depth: Option<u32>,
    /// Număr maxim de URL-uri
    #[arg(long, default_value_t = 1_000_000)]
    max_urls: usize,
    /// Redirecționări maxime urmate
    #[arg(long, default_value_t = 10)]
    max_redirects: usize,

    /// Nu verifica legăturile externe
    #[arg(long)]
    no_external: bool,
    /// Nu parcurge pagini din afara directorului de start
    #[arg(long)]
    stay_in_dir: bool,
    /// Nu verifica imagini/CSS/JS/media (doar pagini)
    #[arg(long)]
    pages_only: bool,
    /// Verifică și acțiunile formularelor
    #[arg(long)]
    forms: bool,
    /// Nu verifica adresele mailto:
    #[arg(long)]
    no_mailto: bool,
    /// Nu verifica legăturile ftp://
    #[arg(long)]
    no_ftp: bool,
    /// Verifică ancorele #fragment
    #[arg(long)]
    fragments: bool,
    /// Tratează redirecționările ca erori
    #[arg(long)]
    redirects_as_errors: bool,

    /// Regex: URL-uri ignorate complet (repetabil)
    #[arg(long, value_name = "REGEX")]
    exclude: Vec<String>,
    /// Regex: URL-uri verificate, dar neparcurse (repetabil)
    #[arg(long, value_name = "REGEX")]
    no_follow: Vec<String>,
    /// Regex: parcurge doar paginile care se potrivesc (repetabil)
    #[arg(long, value_name = "REGEX")]
    only: Vec<String>,
    /// Gazdă suplimentară considerată internă (repetabil)
    #[arg(long, value_name = "HOST")]
    internal_host: Vec<String>,
    /// Cod HTTP considerat OK (repetabil, ex. 999)
    #[arg(long, value_name = "COD")]
    ignore_status: Vec<u16>,

    /// Antetul User-Agent
    #[arg(long)]
    user_agent: Option<String>,
    /// Ignoră robots.txt
    #[arg(long)]
    no_robots: bool,
    /// Pornește și din sitemap.xml
    #[arg(long)]
    sitemap: bool,
    /// Permite contactarea adreselor private/locale găsite pe un site public (implicit blocate: anti-SSRF)
    #[arg(long)]
    allow_private: bool,
    /// Acceptă certificate TLS invalide
    #[arg(long)]
    insecure: bool,
    /// Dezactivează cookie-urile
    #[arg(long)]
    no_cookies: bool,
    /// Proxy (http://, https://, socks5://)
    #[arg(long)]
    proxy: Option<String>,
    /// Autentificare Basic pentru gazdele interne: utilizator:parolă
    #[arg(long, value_name = "USER:PAROLA")]
    auth: Option<String>,
    /// Antet suplimentar „Nume: valoare” (repetabil)
    #[arg(short = 'H', long = "header", value_name = "ANTET")]
    headers: Vec<String>,
    /// Pauză minimă între cereri către aceeași gazdă (ms)
    #[arg(long, default_value_t = 0)]
    delay: u64,

    /// Director local pentru căutarea fișierelor orfane
    #[arg(long, value_name = "DIR")]
    orphans: Option<PathBuf>,
    /// URL-ul corespunzător rădăcinii directorului de orfani
    #[arg(long, value_name = "URL")]
    orphan_base: Option<String>,

    /// Reverifică doar linkurile rupte dintr-o sesiune salvată
    #[arg(long, value_name = "FISIER")]
    recheck: Option<PathBuf>,
    /// Fișier de ieșire; formatul se alege după extensie: .html .csv .tsv .json .txt .xml (sitemap) .tree .veriga (sesiune). Repetabil.
    #[arg(short = 'o', long = "out", value_name = "FISIER")]
    out: Vec<PathBuf>,
    /// În CSV/TSV exportă doar legăturile rupte
    #[arg(long)]
    only_broken: bool,
    /// Limba rapoartelor: ro sau en
    #[arg(long, default_value = "ro")]
    lang: String,
    /// Fără progres și fără listă pe ecran
    #[arg(short = 'q', long)]
    quiet: bool,
    /// Cod de ieșire 0 chiar dacă s-au găsit linkuri rupte
    #[arg(long)]
    no_fail: bool,
}

#[allow(clippy::field_reassign_with_default)]
fn build_config(c: &Cli) -> Result<Config> {
    let mut cfg = Config::default();
    cfg.start = c.start.clone();
    cfg.threads = c.threads;
    cfg.per_host = c.per_host;
    cfg.timeout_secs = c.timeout;
    cfg.retries = c.retries;
    cfg.max_depth = c.depth;
    cfg.max_urls = c.max_urls;
    cfg.max_redirects = c.max_redirects;
    cfg.check_external = !c.no_external;
    cfg.stay_in_dir = c.stay_in_dir;
    cfg.check_resources = !c.pages_only;
    cfg.check_forms = c.forms;
    cfg.check_mailto = !c.no_mailto;
    cfg.check_ftp = !c.no_ftp;
    cfg.check_fragments = c.fragments;
    cfg.treat_redirects_as_errors = c.redirects_as_errors;
    cfg.exclude = c.exclude.clone();
    cfg.no_follow = c.no_follow.clone();
    cfg.include_only = c.only.clone();
    cfg.internal_hosts = c.internal_host.clone();
    cfg.ignore_status = c.ignore_status.clone();
    if let Some(ua) = &c.user_agent {
        cfg.user_agent = ua.clone();
    }
    cfg.respect_robots = !c.no_robots;
    cfg.seed_sitemaps = c.sitemap;
    cfg.ignore_tls_errors = c.insecure;
    cfg.block_private = !c.allow_private;
    cfg.cookies = !c.no_cookies;
    cfg.proxy = c.proxy.clone();
    if let Some(a) = &c.auth {
        let (u, p) = a
            .split_once(':')
            .ok_or_else(|| anyhow!("--auth trebuie să aibă forma utilizator:parolă"))?;
        cfg.basic_auth = Some((u.to_string(), p.to_string()));
    }
    for h in &c.headers {
        let (k, v) = h
            .split_once(':')
            .ok_or_else(|| anyhow!("antet invalid „{h}” (formă: Nume: valoare)"))?;
        cfg.headers
            .push((k.trim().to_string(), v.trim().to_string()));
    }
    cfg.delay_ms = c.delay;
    if let Some(d) = &c.orphans {
        cfg.orphan = Some(OrphanConfig {
            dir: d.clone(),
            base_url: c.orphan_base.clone(),
            ..Default::default()
        });
    }
    cfg.lang = if c.lang == "en" {
        "en".into()
    } else {
        "ro".into()
    };
    Ok(cfg)
}

fn write_outputs(s: &Session, outs: &[PathBuf], only_broken: bool) -> Result<()> {
    for p in outs {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let data: Vec<u8> = match ext.as_str() {
            "html" | "htm" => report::to_html(s).into_bytes(),
            "csv" => report::to_csv(s, b',', only_broken).into_bytes(),
            "tsv" => report::to_csv(s, b'\t', only_broken).into_bytes(),
            "json" => report::to_json(s).into_bytes(),
            "txt" | "log" => report::to_text(s).into_bytes(),
            "xml" => report::to_sitemap_xml(s).into_bytes(),
            "tree" => report::to_tree_text(s, 12).into_bytes(),
            "veriga" => {
                s.save(p)?;
                continue;
            }
            other => bail!(
                "extensie de ieșire necunoscută „.{other}” pentru {}",
                p.display()
            ),
        };
        std::fs::write(p, data).with_context(|| format!("nu pot scrie {}", p.display()))?;
        if !s.config.start.is_empty() {
            eprintln!("→ {}", p.display());
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(a) = std::env::args().nth(1)
        && guardian::NAMES.contains(&a.as_str())
    {
        return guardian::dispatch().await;
    }
    let cli = Cli::parse();
    if cli.recheck.is_none() && cli.start.is_empty() {
        bail!("indică cel puțin un URL, fișier sau director de start (vezi --help)");
    }
    // validăm ieșirile ÎNAINTE de o verificare care poate dura ore (nu pierdem rezultatele la final)
    for p in &cli.out {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ![
            "html", "htm", "csv", "tsv", "json", "txt", "log", "xml", "tree", "veriga",
        ]
        .contains(&ext.as_str())
        {
            bail!(
                "extensie de ieșire necunoscută „.{ext}” pentru {} (acceptate: .html .csv .tsv .json .txt .xml .tree .veriga)",
                p.display()
            );
        }
        if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty())
            && !dir.is_dir()
        {
            bail!("directorul de ieșire {} nu există", dir.display());
        }
    }
    let quiet = cli.quiet;
    let tty = std::io::stderr().is_terminal();

    let ctl = Control::new();
    {
        let c = ctl.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                eprintln!("\nOprire cerută — se salvează rezultatele parțiale…");
                c.cancel();
            }
        });
    }
    let (tx, mut rx) = unbounded_channel();
    let lang = if cli.lang == "en" { "en" } else { "ro" };
    let printer = tokio::spawn(async move {
        let mut last_line = 0usize;
        while let Some(ev) = rx.recv().await {
            match ev {
                Event::Checked(r) if !quiet => {
                    if r.status.is_error() {
                        if tty && last_line > 0 {
                            eprint!("\r{}\r", " ".repeat(last_line));
                        }
                        let code = r
                            .http_status
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "---".into());
                        eprintln!("[{code}] {} — {}", r.url, r.status.label(lang));
                        if let Some(p) = r.parents.first() {
                            eprintln!("        ↳ {}", p.url);
                        }
                    }
                }
                Event::Progress(p) if !quiet && tty => {
                    let line = format!(
                        "  verificate: {} · în coadă: {} · active: {} · rupte: {} · {:.0}s",
                        p.checked,
                        p.queued,
                        p.in_flight,
                        p.broken,
                        p.elapsed_ms as f64 / 1000.0
                    );
                    last_line = line.chars().count();
                    eprint!("\r{line}");
                    let _ = std::io::stderr().flush();
                }
                Event::Finished if tty && last_line > 0 => {
                    eprint!("\r{}\r", " ".repeat(last_line));
                }
                _ => {}
            }
        }
    });

    let session = if let Some(p) = &cli.recheck {
        let s =
            Session::load(p).with_context(|| format!("nu pot încărca sesiunea {}", p.display()))?;
        veriga_core::recheck(s, ctl.clone(), Some(tx)).await?
    } else {
        let cfg = build_config(&cli)?;
        veriga_core::run(cfg, ctl.clone(), Some(tx)).await?
    };
    let _ = printer.await;

    if cli.out.is_empty() && !cli.quiet {
        println!("{}", report::to_text(&session));
    } else if !cli.quiet {
        let sm = report::Summary::from(&session);
        eprintln!(
            "Gata: {} URL-uri · {} OK · {} rupte · {} redirecționări · {} orfane",
            sm.total, sm.ok, sm.broken, sm.redirected, sm.orphans
        );
    }
    write_outputs(&session, &cli.out, cli.only_broken)?;

    let broken = session.broken_count();
    let _ = Arc::strong_count(&ctl);
    if broken > 0 && !cli.no_fail {
        std::process::exit(2);
    }
    Ok(())
}
