//! Subcomenzi „Guardian”: lint, fix, undo, log404, guardian, compliance, history,
//! watch, wizard. Se activează când primul argument este numele unei subcomenzi;
//! altfel rămâne interfața clasică (`veriga URL …`).

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use veriga_core::fixer::{self, ApplyOpts, FixLog, FixStatus, ProposeOpts};
use veriga_core::i18n::{self, tr};
use veriga_core::insights::{self, Period};
use veriga_core::rules::{self, Rule};
use veriga_core::trf;
use veriga_core::{Config, Control, Session, srcscan};

pub const NAMES: &[&str] = &[
    "lint",
    "fix",
    "undo",
    "log404",
    "guardian",
    "compliance",
    "history",
    "watch",
    "wizard",
];

#[derive(Parser, Debug)]
#[command(
    name = "veriga",
    about = "VERIGA Guardian — reparare, istoric, conformitate"
)]
struct G {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Debug, Clone)]
struct Common {
    /// Director de stare (istoric, jurnal reparații, copii de siguranță)
    #[arg(long, default_value = ".veriga")]
    state: PathBuf,
    /// Limba: ro sau en (implicit: din LANG / VERIGA_LANG)
    #[arg(long)]
    lang: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Analiză offline a surselor (HTML/Markdown/WXR/sitemap): URL-uri malformate, urmărire, http/www
    Lint {
        /// Fișiere sau directoare
        paths: Vec<PathBuf>,
        /// Ieșire: .csv sau .json
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[command(flatten)]
        c: Common,
    },
    /// Propune (și, cu --apply, aplică) reparații verificate live
    Fix {
        paths: Vec<PathBuf>,
        /// Sesiune .veriga (pentru reparații de redirecționări permanente)
        #[arg(long)]
        session: Option<PathBuf>,
        /// Doar aceste reguli: scheme_typo,unmangle,missing_scheme,tracking,protocol,www,redirect
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// Nu propune https:// pentru fiecare http:// (doar când site-ul amestecă schemele)
        #[arg(long)]
        no_upgrade_http: bool,
        /// Permite verificarea adreselor private/locale (implicit blocate: anti-SSRF)
        #[arg(long)]
        allow_private: bool,
        /// Nu verifica live (propunerile rămân „neverificate” și nu se aplică automat)
        #[arg(long)]
        no_verify: bool,
        /// Scrie modificările (implicit: doar propune)
        #[arg(long)]
        apply: bool,
        /// Aplică și propunerile neverificate (nerecomandat)
        #[arg(long)]
        allow_unverified: bool,
        /// Înlocuire manuală VECHI=NOU (repetabil)
        #[arg(long, value_name = "VECHI=NOU")]
        replace: Vec<String>,
        /// Dezleagă linkul (păstrează textul), repetabil
        #[arg(long, value_name = "URL")]
        unlink: Vec<String>,
        /// Caută în Wayback Machine și propune arhiva pentru acest URL (opt-in, repetabil)
        #[arg(long, value_name = "URL")]
        wayback: Vec<String>,
        /// Raport propuneri (.csv)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[command(flatten)]
        c: Common,
    },
    /// Anulează reparații din jurnal
    Undo {
        /// ID-uri de anulat (repetabil)
        #[arg(long)]
        id: Vec<String>,
        /// Anulează o rulare întreagă
        #[arg(long)]
        run: Option<String>,
        /// Doar listează jurnalul
        #[arg(long)]
        list: bool,
        #[command(flatten)]
        c: Common,
    },
    /// Importă 404-uri din jurnale de acces Apache/nginx
    Log404 {
        logfile: PathBuf,
        /// Sesiune .veriga sau fișier text (un path pe linie) cu pagini cunoscute, pentru sugestii
        #[arg(long)]
        known: Option<PathBuf>,
        /// Include și boții
        #[arg(long)]
        include_bots: bool,
        /// Ieșire: .csv sau .htaccess
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[command(flatten)]
        c: Common,
    },
    /// Pagină HTML: tablou de bord, domenii, hartă de sănătate, carte, redirecționări, tendințe
    Guardian {
        session: PathBuf,
        /// Sesiunea anterioară (nou/rezolvat/derivă)
        #[arg(long)]
        prev: Option<PathBuf>,
        /// Adaugă rularea în istoric
        #[arg(long)]
        record: bool,
        #[arg(short = 'o', long, default_value = "guardian.html")]
        out: PathBuf,
        #[command(flatten)]
        c: Common,
    },
    /// Raport de conformitate pe perioadă
    Compliance {
        session: PathBuf,
        /// Ultimele N zile (implicit 30)
        #[arg(long, default_value_t = 30, conflicts_with = "all")]
        days: u32,
        #[arg(long)]
        all: bool,
        #[arg(short = 'o', long, default_value = "conformitate.html")]
        out: PathBuf,
        #[command(flatten)]
        c: Common,
    },
    /// Afișează istoricul rulărilor
    History {
        #[command(flatten)]
        c: Common,
    },
    /// Rulări programate (din fișier de configurare creat de `wizard`)
    Watch {
        #[arg(long, default_value = "veriga-guardian.json")]
        config: PathBuf,
        /// O singură rulare, apoi ieșire (pentru cron / Task Scheduler)
        #[arg(long)]
        once: bool,
    },
    /// Asistent interactiv de configurare în 4 pași
    Wizard {
        #[arg(long, default_value = "veriga-guardian.json")]
        config: PathBuf,
    },
}

// ───────────── configurare programată ─────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GuardianConfig {
    pub start: Vec<String>,
    /// Surse locale (fișiere/directoare) pentru reparații
    pub sources: Vec<PathBuf>,
    pub state: PathBuf,
    pub every_hours: u64,
    pub max_depth: Option<u32>,
    pub check_external: bool,
    /// Reguli aplicate automat (verificate live). Gol = doar propuneri.
    pub auto_fix: Vec<String>,
    pub webhook: Option<String>,
    /// Comandă compatibilă sendmail (ex. /usr/sbin/sendmail)
    pub sendmail: Option<String>,
    pub email_to: Option<String>,
    /// Notifică doar dacă apar linkuri noi rupte
    pub notify_only_new: bool,
    pub lang: String,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            start: vec![],
            sources: vec![],
            state: PathBuf::from(".veriga"),
            every_hours: 24,
            max_depth: None,
            check_external: true,
            auto_fix: vec![],
            webhook: None,
            sendmail: None,
            email_to: None,
            notify_only_new: true,
            lang: "ro".into(),
        }
    }
}

fn parse_rules(v: &[String]) -> Result<Option<BTreeSet<Rule>>> {
    if v.is_empty() {
        return Ok(None);
    }
    let mut s = BTreeSet::new();
    for x in v {
        s.insert(match x.trim() {
            "scheme_typo" => Rule::SchemeTypo,
            "unmangle" => Rule::Unmangle,
            "missing_scheme" => Rule::MissingScheme,
            "tracking" => Rule::Tracking,
            "protocol" => Rule::Protocol,
            "www" => Rule::WwwConsistency,
            "redirect" => Rule::Redirect,
            o => bail!(trf!("regulă necunoscută „{o}”", "unknown rule “{o}”")),
        });
    }
    Ok(Some(s))
}

fn lang(l: &str) -> &'static str {
    if l == "en" { "en" } else { "ro" }
}

fn paths_state(state: &Path) -> (PathBuf, PathBuf, PathBuf) {
    insights::default_paths(state)
}

fn print_proposals(ps: &[fixer::Proposal], lg: &str) {
    for p in ps {
        println!(
            "{} {} [{}]{}{}\n    {}\n  → {}\n    {} · {}",
            if p.verified { "✔" } else { "?" },
            p.id,
            p.rule.id(),
            if p.impact() { " ★impact" } else { "" },
            if p.nav { tr(" (meniu)", " (menu)") } else { "" },
            p.old,
            p.new,
            p.page,
            p.note
        );
    }
    let ver = ps.iter().filter(|p| p.verified).count();
    println!(
        "\n{} {} · {} {}",
        ps.len(),
        if lg == "en" { "proposals" } else { "propuneri" },
        ver,
        if lg == "en" {
            "verified live"
        } else {
            "verificate live"
        }
    );
}

#[allow(clippy::too_many_arguments)] // oglindește 1:1 opțiunile CLI
async fn do_fix(
    paths: &[PathBuf],
    session: Option<&Path>,
    only: &[String],
    no_upgrade_http: bool,
    allow_private: bool,
    no_verify: bool,
    apply: bool,
    allow_unverified: bool,
    replace: &[String],
    unlink: &[String],
    wayback: &[String],
    out: Option<&Path>,
    c: &Common,
) -> Result<(usize, usize)> {
    let lg = i18n::code();
    let occs = srcscan::scan_paths(paths)?;
    let sess = match session {
        Some(p) => {
            Some(Session::load(p).with_context(|| trf!("sesiune {}", "session {}", p.display()))?)
        }
        None => None,
    };
    let opts = ProposeOpts {
        verify: !no_verify,
        only: parse_rules(only)?,
        lang: lg.into(),
        upgrade_http: !no_upgrade_http,
        allow_private,
        ..Default::default()
    };
    let mut ps = fixer::propose(&occs, sess.as_ref(), &opts).await;
    for r in replace {
        let (a, b) = r
            .split_once('=')
            .ok_or_else(|| anyhow!("--replace cere VECHI=NOU"))?;
        ps.extend(fixer::manual_replace(&occs, a, b).map_err(|e| anyhow!("--replace {a}: {e}"))?);
    }
    for u in unlink {
        ps.extend(fixer::unlink(&occs, u));
    }
    if !wayback.is_empty() {
        let cl = fixer::client(20, &opts.user_agent)?;
        for u in wayback {
            match fixer::wayback_lookup(&cl, u).await {
                Some(a) => match fixer::manual_replace(&occs, u, &a) {
                    Ok(v) => ps.extend(v),
                    Err(e) => eprintln!(
                        "{}",
                        trf!(
                            "Wayback: arhiva pentru {u} refuzată ({e})",
                            "Wayback: the archive for {u} was refused ({e})"
                        )
                    ),
                },
                None => eprintln!(
                    "{}",
                    trf!(
                        "Wayback: nicio arhivă pentru {u}",
                        "Wayback: no archive for {u}"
                    )
                ),
            }
        }
    }
    // ID-uri unice
    for (i, p) in ps.iter_mut().enumerate() {
        p.id = format!("p{}", i + 1);
    }
    fixer::fill_old_raw(&mut ps);
    print_proposals(&ps, lg);
    if let Some(o) = out {
        std::fs::write(o, fixer::proposals_csv(&ps, lg))?;
        eprintln!("→ {}", o.display());
    }
    if !apply {
        if !ps.is_empty() {
            eprintln!(
                "{}",
                tr(
                    "(simulare — adaugă --apply pentru a scrie modificările)",
                    "(dry run — add --apply to write the changes)"
                )
            );
        }
        return Ok((ps.len(), 0));
    }
    let (_, logp, bk) = paths_state(&c.state);
    let _lock = FixLog::lock(&logp)?;
    let mut log = FixLog::load_checked(&logp)?;
    let run = fixer::run_id();
    let rep = fixer::apply(
        &ps,
        &mut log,
        &ApplyOpts {
            backup_dir: bk,
            run: run.clone(),
            allow_unverified,
        },
    );
    log.save(&logp)?;
    println!(
        "\n{}",
        trf!(
            "Aplicate: {} (rularea {run}) · omise: {}",
            "Applied: {} (run {run}) · skipped: {}",
            rep.applied.len(),
            rep.skipped.len()
        )
    );
    for (id, why) in &rep.skipped {
        println!("  {} {id}: {why}", tr("omis", "skipped"));
    }
    Ok((ps.len(), rep.applied.len()))
}

// ───────────── notificări ─────────────

async fn notify(cfg: &GuardianConfig, subject: &str, body: &str) {
    if let Some(w) = &cfg.webhook {
        let payload = serde_json::json!({ "text": format!("{subject}\n{body}"), "content": format!("{subject}\n{body}"), "subject": subject, "body": body });
        match reqwest::Client::new()
            .post(w)
            .header("content-type", "application/json")
            .body(payload.to_string())
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => eprintln!("webhook: HTTP {}", r.status()),
            Err(e) => eprintln!("webhook: {e}"),
        }
    }
    if let (Some(cmd), Some(to)) = (&cfg.sendmail, &cfg.email_to) {
        // antete curățate: fără CR/LF în subiect sau destinatar
        let clean = |s: &str| s.replace(['\r', '\n'], " ");
        let msg = format!(
            "To: {}\r\nSubject: =?UTF-8?B?{}?=\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\n",
            clean(to),
            b64(subject.as_bytes()),
            body
        );
        let child = std::process::Command::new(cmd)
            .arg("-t")
            .stdin(std::process::Stdio::piped())
            .spawn();
        match child {
            Ok(mut ch) => {
                if let Some(mut si) = ch.stdin.take() {
                    let _ = si.write_all(msg.as_bytes());
                }
                let _ = ch.wait();
            }
            Err(e) => eprintln!("sendmail: {e}"),
        }
    }
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for ch in data.chunks(3) {
        let n = (ch[0] as u32) << 16
            | (*ch.get(1).unwrap_or(&0) as u32) << 8
            | *ch.get(2).unwrap_or(&0) as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if ch.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        o.push(if ch.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    o
}

// ───────────── o rulare completă ─────────────

#[allow(clippy::field_reassign_with_default)]
async fn one_run(g: &GuardianConfig) -> Result<()> {
    std::fs::create_dir_all(&g.state)?;
    // o singură rulare la un moment dat pe același director de stare
    let _run_lock = FixLog::lock(&g.state.join("watch.json"))?;
    let (hist, logp, bk) = paths_state(&g.state);
    let last = g.state.join("last.veriga");
    let prev = Session::load(&last).ok();
    let mut cfg = Config::default();
    cfg.start = g.start.clone();
    cfg.max_depth = g.max_depth;
    cfg.check_external = g.check_external;
    cfg.lang = lang(&g.lang).into();
    let ctl = Control::new();
    let s = veriga_core::run(cfg, ctl, None).await?;
    // Rulare inconclusivă (rețea căzută / oprită): nu poluăm istoricul, nu suprascriem
    // referința anterioară, nu reparăm nimic și nu inundăm cu „linkuri noi rupte”.
    let checked: Vec<_> = s
        .results
        .iter()
        .filter(|r| {
            !matches!(
                r.status,
                veriga_core::Status::Orphan | veriga_core::Status::Pending
            )
        })
        .collect();
    let net_fail = checked
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                veriga_core::Status::NoConnection
                    | veriga_core::Status::DnsFailure
                    | veriga_core::Status::Timeout
            )
        })
        .count();
    if s.cancelled || checked.is_empty() || (checked.len() >= 5 && net_fail * 2 > checked.len()) {
        let msg = trf!(
            "rulare inconclusivă: {net_fail}/{} verificări cu eroare de rețea{} — nu a fost înregistrată",
            "inconclusive run: {net_fail}/{} checks with a network error{} — not recorded",
            checked.len(),
            if s.cancelled {
                tr(" (întreruptă)", " (interrupted)")
            } else {
                ""
            }
        );
        println!("{msg}");
        notify(
            g,
            tr(
                "VERIGA: rulare inconclusivă (rețea?)",
                "VERIGA: inconclusive run (network?)",
            ),
            &msg,
        )
        .await;
        return Ok(());
    }
    s.save(&last)?;
    let snap = insights::history_append(&hist, &s)?;
    let hist_v = insights::history_load(&hist);
    std::fs::write(
        g.state.join("guardian.html"),
        insights::guardian_html(&s, prev.as_ref(), &hist_v),
    )?;
    let d = prev.as_ref().map(|p| insights::diff(p, &s));
    let mut fixed = 0;
    if !g.auto_fix.is_empty() && !g.sources.is_empty() {
        let occs = srcscan::scan_paths(&g.sources)?;
        let opts = ProposeOpts {
            verify: true,
            only: parse_rules(&g.auto_fix)?,
            lang: lang(&g.lang).into(),
            ..Default::default()
        };
        let mut ps = fixer::propose(&occs, Some(&s), &opts).await;
        fixer::fill_old_raw(&mut ps);
        let _lock = FixLog::lock(&logp)?;
        let mut log = FixLog::load_checked(&logp)?;
        let rep = fixer::apply(
            &ps,
            &mut log,
            &ApplyOpts {
                backup_dir: bk,
                run: fixer::run_id(),
                allow_unverified: false,
            },
        );
        log.save(&logp)?;
        fixed = rep.applied.len();
    }
    let new_broken = d
        .as_ref()
        .map(|d| d.new_broken.len())
        .unwrap_or(snap.broken);
    println!(
        "{}",
        trf!(
            "{} · {} URL-uri · {} rupte ({} noi) · {} de verificat · {} reparate automat",
            "{} · {} URLs · {} broken ({} new) · {} to review · {} fixed automatically",
            snap.time,
            snap.total,
            snap.broken,
            new_broken,
            snap.review,
            fixed
        )
    );
    let should = if g.notify_only_new {
        new_broken > 0 || d.as_ref().map(|d| !d.drift.is_empty()).unwrap_or(false)
    } else {
        snap.broken > 0
    };
    if should {
        let mut body = trf!(
            "{}\nURL-uri: {} · rupte: {} (noi: {}) · de verificat: {} · reparate automat: {}\n",
            "{}\nURLs: {} · broken: {} (new: {}) · to review: {} · fixed automatically: {}\n",
            g.start.join(", "),
            snap.total,
            snap.broken,
            new_broken,
            snap.review,
            fixed
        );
        if let Some(d) = &d {
            for u in d.new_broken.iter().take(30) {
                body.push_str(&format!("  {}: {u}\n", tr("NOU RUPT", "NEWLY BROKEN")));
            }
            for x in d.drift.iter().take(10) {
                body.push_str(&format!("  {}: {}\n", tr("DERIVĂ", "DRIFT"), x.url));
            }
        }
        notify(
            g,
            &trf!(
                "VERIGA: {new_broken} linkuri rupte noi",
                "VERIGA: {new_broken} new broken links"
            ),
            &body,
        )
        .await;
    }
    Ok(())
}

fn ask(prompt: &str, default: &str) -> String {
    print!("{prompt} [{default}]: ");
    let _ = std::io::stdout().flush();
    let mut l = String::new();
    let _ = std::io::stdin().lock().read_line(&mut l);
    let l = l.trim();
    if l.is_empty() {
        default.to_string()
    } else {
        l.to_string()
    }
}

/// Ajutorul subcomenzilor în engleză: (subcomandă, descriere, [(argument, ajutor, nume valoare)]).
type SubHelp = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str, Option<&'static str>)],
);
const SUB_HELP_EN: &[SubHelp] = &[
    (
        "lint",
        "Offline analysis of sources (HTML/Markdown/WXR/sitemap): malformed URLs, tracking, http/www",
        &[
            ("paths", "Files or directories", None),
            ("out", "Output: .csv or .json", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "fix",
        "Propose (and, with --apply, apply) fixes verified live",
        &[
            (
                "session",
                "A .veriga session (for permanent-redirect fixes)",
                None,
            ),
            (
                "only",
                "Only these rules: scheme_typo,unmangle,missing_scheme,tracking,protocol,www,redirect",
                None,
            ),
            (
                "no_upgrade_http",
                "Do not propose https:// for every http:// (only when the site mixes schemes)",
                None,
            ),
            (
                "allow_private",
                "Allow checking private/local addresses (blocked by default: anti-SSRF)",
                None,
            ),
            (
                "no_verify",
                "Do not verify live (proposals stay \"unverified\" and are not applied automatically)",
                None,
            ),
            ("apply", "Write the changes (default: only propose)", None),
            (
                "allow_unverified",
                "Also apply unverified proposals (not recommended)",
                None,
            ),
            (
                "replace",
                "Manual replacement OLD=NEW (repeatable)",
                Some("OLD=NEW"),
            ),
            (
                "unlink",
                "Unlink the link (keep the text), repeatable",
                Some("URL"),
            ),
            (
                "wayback",
                "Look in the Wayback Machine and propose the archive for this URL (opt-in, repeatable)",
                Some("URL"),
            ),
            ("out", "Proposals report (.csv)", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "undo",
        "Undo fixes from the log",
        &[
            ("id", "IDs to undo (repeatable)", None),
            ("run", "Undo a whole run", None),
            ("list", "Only list the log", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "log404",
        "Import 404s from Apache/nginx access logs",
        &[
            (
                "known",
                "A .veriga session or a text file (one path per line) with known pages, for suggestions",
                None,
            ),
            ("include_bots", "Include bots too", None),
            ("out", "Output: .csv or .htaccess", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "guardian",
        "HTML page: dashboard, domains, health map, book, redirects, trends",
        &[
            ("prev", "The previous session (new/resolved/drift)", None),
            ("record", "Add the run to the history", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "compliance",
        "Compliance report over a period",
        &[
            ("days", "Last N days (default 30)", None),
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "history",
        "Show the run history",
        &[
            ("state", "State directory (history, fix log, backups)", None),
            (
                "lang",
                "Language: ro or en (default: from LANG / VERIGA_LANG)",
                None,
            ),
        ],
    ),
    (
        "watch",
        "Scheduled runs (from a configuration file created by `wizard`)",
        &[(
            "once",
            "A single run, then exit (for cron / Task Scheduler)",
            None,
        )],
    ),
    ("wizard", "Interactive 4-step setup wizard", &[]),
];

fn localized_command() -> clap::Command {
    let mut cmd = <G as clap::CommandFactory>::command();
    if i18n::is_en() {
        cmd = cmd.about("VERIGA Guardian — fixes, history, compliance");
        for (name, about, args) in SUB_HELP_EN {
            let (name, about, args) = (*name, *about, *args);
            cmd = cmd.mut_subcommand(name, move |mut sc| {
                sc = sc.about(about);
                for (id, help, vn) in args {
                    let (help, vn) = (*help, *vn);
                    sc = sc.mut_arg(*id, move |a| {
                        let a = a.help(help);
                        match vn {
                            Some(v) => a.value_name(v),
                            None => a,
                        }
                    });
                }
                sc
            });
        }
    }
    cmd
}

pub async fn dispatch() -> Result<()> {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    i18n::set(&crate::pick_lang(&argv));
    let g = match localized_command().try_get_matches_from(argv) {
        Ok(m) => <G as clap::FromArgMatches>::from_arg_matches(&m).unwrap_or_else(|e| e.exit()),
        Err(e) => e.exit(),
    };
    match g.cmd {
        Cmd::Lint { paths, out, c: _ } => {
            if paths.is_empty() {
                bail!(tr(
                    "indică fișiere sau directoare-sursă",
                    "give source files or directories"
                ));
            }
            let lg = i18n::code();
            let occs = srcscan::scan_paths(&paths)?;
            let f = rules::analyze(&occs, lg);
            for x in &f {
                println!(
                    "[{}]{} {}\n    {}{}",
                    x.rule.id(),
                    if x.nav { " (meniu)" } else { "" },
                    x.page,
                    x.url,
                    x.suggestion
                        .as_ref()
                        .map(|s| format!("\n  → {s}"))
                        .unwrap_or_default()
                );
            }
            println!(
                "\n{}",
                trf!(
                    "{} linkuri scanate · {} constatări",
                    "{} links scanned · {} findings",
                    occs.len(),
                    f.len()
                )
            );
            if let Some(o) = out {
                let ext = o.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "json" {
                    std::fs::write(&o, serde_json::to_vec_pretty(&f)?)?;
                } else {
                    let mut w = csv_out();
                    w.write_record(["file", "page", "rule", "url", "suggestion", "menu", "note"])?;
                    for x in &f {
                        w.write_record(
                            [
                                x.file.as_str(),
                                &x.page,
                                x.rule.id(),
                                &x.url,
                                x.suggestion.as_deref().unwrap_or(""),
                                if x.nav { "da" } else { "nu" },
                                &x.note,
                            ]
                            .map(fixer::csv_safe),
                        )?;
                    }
                    std::fs::write(&o, w.into_inner()?)?;
                }
                eprintln!("→ {}", o.display());
            }
            if !f.is_empty() {
                std::process::exit(2);
            }
        }
        Cmd::Fix {
            paths,
            session,
            only,
            no_upgrade_http,
            allow_private,
            no_verify,
            apply,
            allow_unverified,
            replace,
            unlink,
            wayback,
            out,
            c,
        } => {
            if paths.is_empty() {
                bail!(tr(
                    "indică fișiere sau directoare-sursă",
                    "give source files or directories"
                ));
            }
            do_fix(
                &paths,
                session.as_deref(),
                &only,
                no_upgrade_http,
                allow_private,
                no_verify,
                apply,
                allow_unverified,
                &replace,
                &unlink,
                &wayback,
                out.as_deref(),
                &c,
            )
            .await?;
        }
        Cmd::Undo { id, run, list, c } => {
            let (_, logp, _) = paths_state(&c.state);
            let _lock = if list {
                None
            } else {
                Some(FixLog::lock(&logp)?)
            };
            let mut log = FixLog::load_checked(&logp)?;
            if list {
                for e in &log.entries {
                    println!(
                        "{} [{}] {:?} {} {} → {}",
                        e.id,
                        e.rule.id(),
                        e.status,
                        e.file.display(),
                        e.old,
                        e.new
                    );
                }
                return Ok(());
            }
            if id.is_empty() && run.is_none() {
                bail!(tr(
                    "indică --id ID, --run RULARE sau --list",
                    "give --id ID, --run RUN or --list"
                ));
            }
            let rep = fixer::undo(
                &mut log,
                if id.is_empty() { None } else { Some(&id) },
                run.as_deref(),
            );
            log.save(&logp)?;
            println!(
                "{}",
                trf!(
                    "Anulate: {} · conflicte: {}",
                    "Undone: {} · conflicts: {}",
                    rep.undone.len(),
                    rep.conflicts.len()
                )
            );
            for (i, w) in rep.conflicts {
                println!("  {i}: {w}");
            }
            let _ = FixStatus::Applied;
        }
        Cmd::Log404 {
            logfile,
            known,
            include_bots,
            out,
            c,
        } => {
            let _ = c;
            let text =
                std::fs::read_to_string(&logfile).with_context(|| logfile.display().to_string())?;
            let mut paths: Vec<String> = vec![];
            if let Some(k) = &known {
                match Session::load(k) {
                    Ok(s) => {
                        for r in &s.results {
                            if r.status.is_broken(&s.config) {
                                continue;
                            }
                            if let Ok(u) = url::Url::parse(&r.url) {
                                paths.push(u.path().to_string());
                            }
                        }
                    }
                    _ => {
                        if let Ok(t) = std::fs::read_to_string(k) {
                            paths = t
                                .lines()
                                .map(|l| l.trim().to_string())
                                .filter(|l| !l.is_empty())
                                .collect();
                        }
                    }
                }
            }
            paths.sort();
            paths.dedup();
            let h = insights::parse_404(&text, !include_bots, &paths);
            for x in h.iter().take(50) {
                println!(
                    "{:>6}×  {}{}",
                    x.count,
                    x.path,
                    x.suggestion
                        .as_ref()
                        .map(|s| format!("  → {s}"))
                        .unwrap_or_default()
                );
            }
            println!(
                "\n{}",
                trf!("{} URL-uri 404 distincte", "{} distinct 404 URLs", h.len())
            );
            if let Some(o) = out {
                let data = if o
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_ascii_lowercase().ends_with("htaccess"))
                {
                    insights::htaccess_rules(&h)
                } else {
                    insights::hits_csv(&h)
                };
                std::fs::write(&o, data)?;
                eprintln!("→ {}", o.display());
            }
        }
        Cmd::Guardian {
            session,
            prev,
            record,
            out,
            c,
        } => {
            let s = Session::load(&session)?;
            let p = prev.as_deref().map(Session::load).transpose()?;
            let (hist, _, _) = paths_state(&c.state);
            if record {
                insights::history_append(&hist, &s)?;
            }
            std::fs::write(
                &out,
                insights::guardian_html(&s, p.as_ref(), &insights::history_load(&hist)),
            )?;
            eprintln!("→ {}", out.display());
        }
        Cmd::Compliance {
            session,
            days,
            all,
            out,
            c,
        } => {
            let s = Session::load(&session)?;
            let (hist, logp, _) = paths_state(&c.state);
            let period = if all { Period::All } else { Period::Days(days) };
            std::fs::write(
                &out,
                insights::compliance_html(
                    &s,
                    &FixLog::load(&logp),
                    &insights::history_load(&hist),
                    period,
                ),
            )?;
            eprintln!("→ {}", out.display());
        }
        Cmd::History { c } => {
            let (hist, _, _) = paths_state(&c.state);
            let h = insights::history_load(&hist);
            if h.is_empty() {
                println!(
                    "{}",
                    trf!(
                        "Istoric gol ({}).",
                        "History is empty ({}).",
                        hist.display()
                    )
                );
            }
            for x in h {
                println!(
                    "{}",
                    trf!(
                        "{}  total {:>6}  rupte {:>5}  verificat {:>4}  redir {:>5}  sănătate {:>5.1}%",
                        "{}  total {:>6}  broken {:>5}  review {:>4}  redir {:>5}  health {:>5.1}%",
                        x.time,
                        x.total,
                        x.broken,
                        x.review,
                        x.redirects,
                        x.health
                    )
                );
            }
        }
        Cmd::Watch { config, once } => {
            let g: GuardianConfig =
                serde_json::from_slice(&std::fs::read(&config).with_context(|| {
                    trf!("configurare {}", "configuration {}", config.display())
                })?)?;
            // configurația fixează limba notificărilor programate
            i18n::set(&g.lang);
            if g.start.iter().all(|s| s.trim().is_empty()) {
                bail!(tr(
                    "configurarea nu conține niciun URL de start",
                    "the configuration contains no start URL"
                ));
            }
            if let Some(cmd) = &g.sendmail {
                // se execută direct, fără shell; cerem cale absolută către un fișier existent
                let p = Path::new(cmd);
                if !p.is_absolute() || !p.is_file() {
                    bail!(
                        "{}",
                        trf!(
                            "`sendmail` trebuie să fie o cale absolută către un executabil existent (primit: {cmd}). Configurările din surse necunoscute pot rula comenzi — verifică fișierul înainte de a-l folosi.",
                            "`sendmail` must be an absolute path to an existing executable (got: {cmd}). Configurations from unknown sources can run commands — check the file before using it."
                        )
                    );
                }
            }
            loop {
                if let Err(e) = one_run(&g).await {
                    eprintln!("{}: {e:#}", tr("rulare eșuată", "run failed"));
                    if once {
                        // cod ≠ 0 ca systemd, CronJob și cron să vadă eșecul
                        return Err(e.context(tr("rulare eșuată", "run failed")));
                    }
                }
                if once {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(g.every_hours.max(1) * 3600))
                    .await;
            }
        }
        Cmd::Wizard { config } => {
            println!(
                "{}\n",
                tr(
                    "VERIGA — asistent de configurare (4 pași)",
                    "VERIGA — setup wizard (4 steps)"
                )
            );
            println!(
                "{}",
                tr("Pasul 1/4 — ce verificăm", "Step 1/4 — what to check")
            );
            let start = ask(
                tr(
                    "URL-uri sau directoare de start (separate prin spațiu)",
                    "Start URLs or directories (separated by spaces)",
                ),
                "https://example.org/",
            );
            let sources = ask(
                tr(
                    "Directoare cu surse editabile (HTML/Markdown/export WordPress), gol = fără reparații",
                    "Directories with editable sources (HTML/Markdown/WordPress export), empty = no fixes",
                ),
                "",
            );
            println!(
                "\n{}",
                tr("Pasul 2/4 — domeniu de aplicare", "Step 2/4 — scope")
            );
            let ext = ask(
                tr(
                    "Verific și linkurile externe? (da/nu)",
                    "Check external links too? (yes/no)",
                ),
                tr("da", "yes"),
            );
            let depth = ask(
                tr(
                    "Adâncime maximă (gol = nelimitat)",
                    "Maximum depth (empty = unlimited)",
                ),
                "",
            );
            println!(
                "\n{}",
                tr(
                    "Pasul 3/4 — reparații automate (verificate live, cu jurnal și anulare)",
                    "Step 3/4 — automatic fixes (verified live, with a log and undo)"
                )
            );
            let auto = ask(
                tr(
                    "Reguli automate separate prin virgulă (scheme_typo,unmangle,missing_scheme,tracking,protocol,www,redirect) sau gol",
                    "Automatic rules separated by commas (scheme_typo,unmangle,missing_scheme,tracking,protocol,www,redirect) or empty",
                ),
                "",
            );
            println!(
                "\n{}",
                tr(
                    "Pasul 4/4 — programare și notificări",
                    "Step 4/4 — schedule and notifications"
                )
            );
            let every = ask(tr("Rulează la fiecare N ore", "Run every N hours"), "24");
            let hook = ask(
                tr(
                    "Webhook (Slack/Discord/generic), gol = niciunul",
                    "Webhook (Slack/Discord/generic), empty = none",
                ),
                "",
            );
            let to = ask(
                tr(
                    "E-mail destinatar (necesită sendmail), gol = niciunul",
                    "Recipient e-mail (needs sendmail), empty = none",
                ),
                "",
            );
            let sm = if to.is_empty() {
                String::new()
            } else {
                ask(
                    tr("Comandă sendmail", "sendmail command"),
                    "/usr/sbin/sendmail",
                )
            };
            let g = GuardianConfig {
                start: start.split_whitespace().map(String::from).collect(),
                sources: sources.split_whitespace().map(PathBuf::from).collect(),
                every_hours: every.parse().unwrap_or(24),
                max_depth: depth.parse().ok(),
                check_external: ext.starts_with('d') || ext.starts_with('y'),
                auto_fix: auto
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                webhook: if hook.is_empty() { None } else { Some(hook) },
                email_to: if to.is_empty() { None } else { Some(to) },
                sendmail: if sm.is_empty() { None } else { Some(sm) },
                lang: i18n::code().into(),
                ..Default::default()
            };
            std::fs::write(&config, serde_json::to_vec_pretty(&g)?)?;
            println!(
                "\n{}",
                trf!(
                    "Salvat în {}.\nRulare unică: veriga watch --config {} --once\nProgramat (la {} h): veriga watch --config {}",
                    "Saved to {}.\nOne-off run: veriga watch --config {} --once\nScheduled (every {} h): veriga watch --config {}",
                    config.display(),
                    config.display(),
                    g.every_hours,
                    config.display()
                )
            );
        }
    }
    Ok(())
}

fn csv_out() -> CsvW {
    CsvW(Vec::new())
}

/// Scriitor CSV minimal (fără dependență suplimentară în CLI).
struct CsvW(Vec<u8>);
impl CsvW {
    fn write_record<I, S>(&mut self, r: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let cells: Vec<String> = r
            .into_iter()
            .map(|c| {
                let c = c.as_ref();
                if c.contains([',', '"', '\n', '\r']) {
                    format!("\"{}\"", c.replace('"', "\"\""))
                } else {
                    c.to_string()
                }
            })
            .collect();
        self.0.extend_from_slice(cells.join(",").as_bytes());
        self.0.push(b'\n');
        Ok(())
    }
    fn into_inner(self) -> Result<Vec<u8>> {
        Ok(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_subcommand_option_has_an_english_help_text() {
        let root = <G as clap::CommandFactory>::command();
        for sc in root.get_subcommands() {
            let name = sc.get_name();
            let entry = SUB_HELP_EN
                .iter()
                .find(|(n, _, _)| *n == name)
                .unwrap_or_else(|| panic!("lipsește subcomanda {name}"));
            for a in sc.get_arguments() {
                let id = a.get_id().as_str();
                if matches!(id, "help" | "version") || a.get_help().is_none() {
                    continue;
                }
                assert!(
                    entry.2.iter().any(|(k, _, _)| *k == id),
                    "lipsește ajutorul în engleză pentru {name} --{id}"
                );
            }
        }
    }
}
