//! VERIGA — verificator de legături în linie de comandă.

mod guardian;

use anyhow::{Context, Result, anyhow, bail};
use clap::{CommandFactory, FromArgMatches, Parser};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use veriga_core::i18n::{self, tr};
use veriga_core::report;
use veriga_core::trf;
use veriga_core::{Config, Control, Event, OrphanConfig, Session};

#[derive(Parser, Debug)]
#[command(
    name = "veriga",
    version,
    about = "VERIGA — verificator de legături (clean-room), pentru site-uri și directoare locale",
    long_about = "Parcurge un site (sau un director local), verifică toate legăturile — pagini, imagini, CSS, \
scripturi, cadre, media, obiecte, e-mail, FTP, ancore — și produce rapoarte HTML/CSV/TSV/JSON/text/sitemap.\n\n\
Exemple:\n  veriga https://exemplu.ro -o raport.html -o rupte.csv\n  veriga ./site --orphans ./site\n  veriga --recheck sesiune.veriga -o raport.html",
    after_help = "Subcomenzi (Guardian) / Subcommands — `veriga <subcomandă> --help`:\n  \
check       verificare de linkuri (implicit) / link check (default)\n  \
lint        analiză offline a surselor / offline source lint\n  \
fix         propune/aplică reparații / propose/apply fixes\n  \
undo        anulează reparații / undo fixes\n  \
log404      404 din jurnale de acces / 404s from access logs\n  \
guardian    verificare + reparare + istoric / check + fix + history\n  \
compliance  raport de conformitate / compliance report\n  \
history     istoricul verificărilor / check history\n  \
watch       verificare periodică / periodic checks\n  \
wizard      configurare ghidată / guided setup\n\n\
Un director cu numele unei subcomenzi se verifică prin `veriga ./lint` sau `veriga -- lint`.\n\
A directory named like a subcommand is checked with `veriga ./lint` or `veriga -- lint`.\n\n\
Coduri de ieșire / Exit codes: 0 ok · 1 eroare de utilizare / usage error · \
2 linkuri rupte / broken links · 3 verificare oprită sau trunchiată (--max-urls, --max-per-pattern), \
are prioritate față de 2 / run cancelled or truncated, takes precedence over 2"
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
    /// Capcane: pagini parcurse maxim per șablon de cale (cifre → N); 0 = fără limită
    #[arg(long, default_value_t = 5_000)]
    max_per_pattern: usize,
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
    /// Proxy gratuit și etic dintr-un preset: tor, tor-browser, warp (vezi --list-proxy-presets)
    #[arg(long, value_name = "PRESET", conflicts_with = "proxy")]
    proxy_preset: Option<String>,
    /// Afișează presetele de proxy, cerințele și avertismentele lor, apoi iese
    #[arg(long)]
    list_proxy_presets: bool,
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
    /// Limba interfeței și a rapoartelor: ro sau en (implicit: din LANG / VERIGA_LANG)
    #[arg(long)]
    lang: Option<String>,
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
    cfg.retries = c.retries.min(veriga_core::check::MAX_RETRIES);
    cfg.max_depth = c.depth;
    cfg.max_urls = c.max_urls;
    cfg.max_per_pattern = c.max_per_pattern;
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
    if let Some(id) = &c.proxy_preset {
        let p = veriga_core::proxy::preset(id).ok_or_else(|| {
            let ids: Vec<_> = veriga_core::proxy::PRESETS.iter().map(|p| p.id).collect();
            anyhow!(trf!(
                "preset de proxy necunoscut „{id}”; disponibile: {}",
                "unknown proxy preset “{id}”; available: {}",
                ids.join(", ")
            ))
        })?;
        veriga_core::proxy::apply(&mut cfg, p);
        eprintln!(
            "Proxy: {} ({}). {} {}",
            tr(p.label, p.label_en),
            p.url,
            tr(p.requires_ro, p.requires_en),
            tr(p.caveat_ro, p.caveat_en)
        );
    }
    if let Some(a) = &c.auth {
        let (u, p) = a.split_once(':').ok_or_else(|| {
            anyhow!(tr(
                "--auth trebuie să aibă forma utilizator:parolă",
                "--auth must have the form user:password"
            ))
        })?;
        cfg.basic_auth = Some((u.to_string(), p.to_string()));
    }
    for h in &c.headers {
        let (k, v) = h.split_once(':').ok_or_else(|| {
            anyhow!(trf!(
                "antet invalid „{h}” (formă: Nume: valoare)",
                "invalid header “{h}” (form: Name: value)"
            ))
        })?;
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
    cfg.lang = i18n::code().into();
    Ok(cfg)
}

/// Suprascrie în configurația unei sesiuni încărcate câmpurile relevante pentru securitate
/// cu valorile curente (linie de comandă / implicite), ca un fișier .veriga străin să nu
/// poată impune proxy, TLS nesigur, dezactivarea anti-SSRF sau antete.
fn adopt_security(loaded: &mut Config, cur: &Config) {
    loaded.proxy = cur.proxy.clone();
    loaded.ignore_tls_errors = cur.ignore_tls_errors;
    loaded.block_private = cur.block_private;
    loaded.user_agent = cur.user_agent.clone();
    loaded.headers = cur.headers.clone();
    loaded.basic_auth = cur.basic_auth.clone();
    loaded.retries = loaded.retries.min(veriga_core::check::MAX_RETRIES);
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
            other => bail!(trf!(
                "extensie de ieșire necunoscută „.{other}” pentru {}",
                "unknown output extension “.{other}” for {}",
                p.display()
            )),
        };
        std::fs::write(p, data)
            .with_context(|| trf!("nu pot scrie {}", "cannot write {}", p.display()))?;
        if !s.config.start.is_empty() {
            eprintln!("→ {}", p.display());
        }
    }
    Ok(())
}

/// Limba din `--lang X` / `--lang=X`, altfel din mediu (`VERIGA_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG`).
pub(crate) fn pick_lang(argv: &[std::ffi::OsString]) -> String {
    let mut it = argv.iter().skip(1).map(|a| a.to_string_lossy().to_string());
    while let Some(a) = it.next() {
        if a == "--" {
            break;
        }
        if a == "--lang" {
            if let Some(v) = it.next() {
                return v;
            }
        } else if let Some(v) = a.strip_prefix("--lang=") {
            return v.to_string();
        }
    }
    i18n::detect().to_string()
}

/// Textele de ajutor în engleză (sursa derivată este în română): id argument → (ajutor, nume valoare).
const HELP_EN: &[(&str, &str, Option<&str>)] = &[
    (
        "start",
        "URLs, HTML files or local directories to start from",
        None,
    ),
    ("threads", "Number of simultaneous checks", None),
    (
        "per_host",
        "Maximum simultaneous connections to the same host",
        None,
    ),
    ("timeout", "Maximum time per request (seconds)", None),
    ("retries", "Retries on transient errors", None),
    ("depth", "Maximum crawl depth", None),
    ("max_urls", "Maximum number of URLs", None),
    (
        "max_per_pattern",
        "Traps: pages crawled at most per path template (digits → N); 0 = unlimited",
        None,
    ),
    ("max_redirects", "Maximum redirects followed", None),
    ("no_external", "Do not check external links", None),
    (
        "stay_in_dir",
        "Do not crawl pages outside the start directory",
        None,
    ),
    (
        "pages_only",
        "Do not check images/CSS/JS/media (pages only)",
        None,
    ),
    ("forms", "Also check form actions", None),
    ("no_mailto", "Do not check mailto: addresses", None),
    ("no_ftp", "Do not check ftp:// links", None),
    ("fragments", "Check #fragment anchors", None),
    ("redirects_as_errors", "Treat redirects as errors", None),
    (
        "exclude",
        "Regex: URLs ignored completely (repeatable)",
        Some("REGEX"),
    ),
    (
        "no_follow",
        "Regex: URLs checked but not crawled (repeatable)",
        Some("REGEX"),
    ),
    (
        "only",
        "Regex: crawl only pages that match (repeatable)",
        Some("REGEX"),
    ),
    (
        "internal_host",
        "Additional host treated as internal (repeatable)",
        Some("HOST"),
    ),
    (
        "ignore_status",
        "HTTP code treated as OK (repeatable, e.g. 999)",
        Some("CODE"),
    ),
    ("user_agent", "The User-Agent header", None),
    ("no_robots", "Ignore robots.txt", None),
    ("sitemap", "Also start from sitemap.xml", None),
    (
        "allow_private",
        "Allow contacting private/local addresses found on a public site (blocked by default: anti-SSRF)",
        None,
    ),
    ("insecure", "Accept invalid TLS certificates", None),
    ("no_cookies", "Disable cookies", None),
    ("proxy", "Proxy (http://, https://, socks5://)", None),
    (
        "proxy_preset",
        "Free, ethical proxy from a preset: tor, tor-browser, warp (see --list-proxy-presets)",
        Some("PRESET"),
    ),
    (
        "list_proxy_presets",
        "Show the proxy presets, their requirements and warnings, then exit",
        None,
    ),
    (
        "auth",
        "Basic authentication for internal hosts: user:password",
        Some("USER:PASSWORD"),
    ),
    (
        "headers",
        "Extra header \"Name: value\" (repeatable)",
        Some("HEADER"),
    ),
    (
        "delay",
        "Minimum pause between requests to the same host (ms)",
        None,
    ),
    (
        "orphans",
        "Local directory to search for orphan files",
        Some("DIR"),
    ),
    (
        "orphan_base",
        "URL corresponding to the root of the orphan directory",
        Some("URL"),
    ),
    (
        "recheck",
        "Re-check only the broken links of a saved session",
        Some("FILE"),
    ),
    (
        "out",
        "Output file; the format follows the extension: .html .csv .tsv .json .txt .xml (sitemap) .tree .veriga (session). Repeatable.",
        Some("FILE"),
    ),
    (
        "only_broken",
        "In CSV/TSV export only the broken links",
        None,
    ),
    (
        "lang",
        "Interface and report language: ro or en (default: from LANG / VERIGA_LANG)",
        None,
    ),
    ("quiet", "No progress and no on-screen list", None),
    (
        "no_fail",
        "Exit code 0 even when broken links were found",
        None,
    ),
];

const ABOUT_EN: &str = "VERIGA — link checker (clean-room), for websites and local directories";
const LONG_ABOUT_EN: &str = "Crawls a website (or a local directory), checks every link — pages, images, CSS, \
scripts, frames, media, objects, e-mail, FTP, anchors — and produces HTML/CSV/TSV/JSON/text/sitemap reports.\n\n\
Examples:\n  veriga https://example.org -o report.html -o broken.csv\n  veriga ./site --orphans ./site\n  veriga --recheck session.veriga -o report.html";

/// Comanda clap, cu textele de ajutor în limba curentă.
pub(crate) fn localized_command() -> clap::Command {
    let mut cmd = Cli::command();
    if i18n::is_en() {
        cmd = cmd.about(ABOUT_EN).long_about(LONG_ABOUT_EN);
        for (id, help, vn) in HELP_EN {
            let (help, vn) = (*help, *vn);
            cmd = cmd.mut_arg(*id, move |a| {
                let a = a.help(help);
                match vn {
                    Some(v) => a.value_name(v),
                    None => a,
                }
            });
        }
    }
    cmd
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(a) = std::env::args().nth(1)
        && guardian::NAMES.contains(&a.as_str())
    {
        return guardian::dispatch().await;
    }
    // `veriga check URL` ≡ `veriga URL` (forma folosită în Docker și documentație)
    let mut argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if argv.get(1).is_some_and(|a| a == "check") {
        argv.remove(1);
    }
    i18n::set(&pick_lang(&argv));
    let cli = match localized_command().try_get_matches_from(argv) {
        Ok(m) => Cli::from_arg_matches(&m).unwrap_or_else(|e| e.exit()),
        Err(e) => e.exit(),
    };
    if cli.list_proxy_presets {
        for p in veriga_core::proxy::PRESETS {
            println!(
                "{}\t{}\t{}\n  {}\n  {}",
                p.id,
                p.url,
                tr(p.label, p.label_en),
                tr(p.requires_ro, p.requires_en),
                tr(p.caveat_ro, p.caveat_en)
            );
        }
        println!(
            "\n{}",
            tr(
                "Listele de proxy-uri publice deschise nu sunt incluse: sunt adesea mașini compromise sau interceptează traficul.",
                "Open public proxy lists are not included: they are often compromised machines or intercept traffic."
            )
        );
        return Ok(());
    }
    if cli.recheck.is_none() && cli.start.is_empty() {
        bail!(tr(
            "indică cel puțin un URL, fișier sau director de start (vezi --help)",
            "give at least one start URL, file or directory (see --help)"
        ));
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
            bail!(trf!(
                "extensie de ieșire necunoscută „.{ext}” pentru {} (acceptate: .html .csv .tsv .json .txt .xml .tree .veriga)",
                "unknown output extension “.{ext}” for {} (accepted: .html .csv .tsv .json .txt .xml .tree .veriga)",
                p.display()
            ));
        }
        if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty())
            && !dir.is_dir()
        {
            bail!(trf!(
                "directorul de ieșire {} nu există",
                "the output directory {} does not exist",
                dir.display()
            ));
        }
    }
    let quiet = cli.quiet;
    let tty = std::io::stderr().is_terminal();

    let ctl = Control::new();
    {
        let c = ctl.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                eprintln!(
                    "\n{}",
                    tr(
                        "Oprire cerută — se salvează rezultatele parțiale…",
                        "Stop requested — saving partial results…"
                    )
                );
                c.cancel();
            }
        });
    }
    let (tx, mut rx) = unbounded_channel();
    let lang = i18n::code();
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
                    let line = trf!(
                        "  verificate: {} · în coadă: {} · active: {} · rupte: {} · {:.0}s",
                        "  checked: {} · queued: {} · active: {} · broken: {} · {:.0}s",
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
        let mut s = Session::load(p).with_context(|| {
            trf!(
                "nu pot încărca sesiunea {}",
                "cannot load the session {}",
                p.display()
            )
        })?;
        // setările de securitate vin din linia de comandă, nu din fișierul sesiunii
        let cur = build_config(&cli)?;
        adopt_security(&mut s.config, &cur);
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
            "{}",
            trf!(
                "Gata: {} URL-uri · {} OK · {} rupte · {} redirecționări · {} orfane",
                "Done: {} URLs · {} OK · {} broken · {} redirects · {} orphans",
                sm.total,
                sm.ok,
                sm.broken,
                sm.redirected,
                sm.orphans
            )
        );
    }
    write_outputs(&session, &cli.out, cli.only_broken)?;

    let broken = session.broken_count();
    let _ = Arc::strong_count(&ctl);
    // verificare incompletă: niciodată „succes”, nici cu --no-fail; are prioritate față de 2
    if session.cancelled || session.truncated {
        if !cli.quiet {
            eprintln!(
                "{}",
                trf!(
                    "Atenție: verificare incompletă ({}) — cod de ieșire 3{}",
                    "Warning: incomplete check ({}) — exit code 3{}",
                    if session.cancelled {
                        tr("oprită", "cancelled")
                    } else {
                        tr(
                            "trunchiată la --max-urls / --max-per-pattern (posibilă capcană)",
                            "truncated at --max-urls / --max-per-pattern (possible trap)",
                        )
                    },
                    if broken > 0 {
                        trf!(
                            " ({broken} rupte până la oprire)",
                            " ({broken} broken before stopping)"
                        )
                    } else {
                        String::new()
                    }
                )
            );
        }
        std::process::exit(3);
    }
    if broken > 0 && !cli.no_fail {
        std::process::exit(2);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_has_an_english_help_text() {
        for a in Cli::command().get_arguments() {
            let id = a.get_id().as_str();
            if matches!(id, "help" | "version") {
                continue;
            }
            assert!(
                HELP_EN.iter().any(|(k, _, _)| *k == id),
                "lipsește ajutorul în engleză pentru --{id}"
            );
        }
        for (k, _, _) in HELP_EN {
            assert!(
                Cli::command().get_arguments().any(|a| a.get_id() == *k),
                "intrare învechită: {k}"
            );
        }
    }

    #[test]
    fn lang_flag_beats_environment() {
        let v = |a: &[&str]| a.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
        assert_eq!(pick_lang(&v(&["veriga", "--lang", "en", "x"])), "en");
        assert_eq!(pick_lang(&v(&["veriga", "--lang=ro"])), "ro");
        assert_eq!(
            pick_lang(&v(&["veriga", "--", "--lang", "en"])),
            i18n::detect()
        );
    }
    #[test]
    fn proxy_preset_conflicts_with_manual_proxy() {
        assert!(
            Cli::try_parse_from([
                "veriga",
                "--proxy-preset",
                "tor",
                "--proxy",
                "http://x:1",
                "u"
            ])
            .is_err()
        );
        let cli = Cli::try_parse_from(["veriga", "--proxy-preset", "tor", "https://example.org"])
            .unwrap();
        assert_eq!(cli.proxy_preset.as_deref(), Some("tor"));
    }

    use clap::CommandFactory;

    #[test]
    fn help_lists_subcommands() {
        let h = Cli::command().render_long_help().to_string();
        for n in guardian::NAMES {
            assert!(h.contains(n), "{n}");
        }
        assert!(h.contains("-- lint"));
    }

    #[test]
    fn double_dash_treats_subcommand_name_as_start() {
        let c = Cli::try_parse_from(["veriga", "--", "lint"]).unwrap();
        assert_eq!(c.start, vec!["lint".to_string()]);
    }

    #[test]
    fn recheck_ignores_security_settings_from_session_file() {
        let mut loaded = Config::with_start("https://example.com/");
        loaded.proxy = Some("http://evil:8080".into());
        loaded.ignore_tls_errors = true;
        loaded.block_private = false;
        loaded.user_agent = "evil".into();
        loaded.headers = vec![("X-Evil".into(), "1".into())];
        loaded.retries = u32::MAX;
        let cli = Cli::try_parse_from(["veriga", "--recheck", "x.veriga"]).unwrap();
        let cur = build_config(&cli).unwrap();
        adopt_security(&mut loaded, &cur);
        assert_eq!(loaded.proxy, None);
        assert!(!loaded.ignore_tls_errors);
        assert!(loaded.block_private);
        assert_eq!(loaded.user_agent, Config::default().user_agent);
        assert!(loaded.headers.is_empty());
        assert_eq!(loaded.retries, veriga_core::check::MAX_RETRIES);
    }

    #[test]
    fn retries_are_clamped() {
        let cli = Cli::try_parse_from(["veriga", "--retries", "4000000000", "x"]).unwrap();
        assert_eq!(
            build_config(&cli).unwrap().retries,
            veriga_core::check::MAX_RETRIES
        );
    }
}
