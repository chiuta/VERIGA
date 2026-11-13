//! VERIGA — interfață grafică nativă (egui/eframe).

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod guardian_ui;
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use veriga_core::report;
use veriga_core::{Config, Control, Event, LinkResult, OrphanConfig, Progress, Session, Status};

#[derive(PartialEq, Clone, Copy)]
enum Filter {
    All,
    Broken,
    Redirects,
    Ok,
    Orphans,
    Skipped,
}

#[derive(PartialEq, Clone, Copy)]
enum SortBy {
    Order,
    Url,
    Status,
    Kind,
    Size,
    Depth,
    Time,
}

enum Job {
    Idle,
    Running {
        rx: UnboundedReceiver<Event>,
        ctl: Arc<Control>,
        result: std::sync::mpsc::Receiver<anyhow::Result<Session>>,
    },
}

struct App {
    // intrări
    start: String,
    cfg: Config,
    // câmpuri text pentru liste
    exclude: String,
    no_follow: String,
    only: String,
    internal_hosts: String,
    ignore_status: String,
    headers: String,
    auth: String,
    orphan_dir: String,
    orphan_base: String,
    max_depth_on: bool,
    max_depth: u32,
    show_options: bool,
    // stare
    job: Job,
    results: Vec<LinkResult>,
    session: Option<Session>,
    progress: Progress,
    phase: String,
    status_line: String,
    status_err: bool,
    last_title: String,
    filter: Filter,
    search: String,
    sort: SortBy,
    sort_desc: bool,
    selected: Option<usize>,
    paused: bool,
    english: bool,
    /// None = zoom automat (potrivit ferestrei); Some(f) = factor ales de utilizator
    zoom: Option<f32>,
    /// ultimul factor aplicat de noi (ca să deosebim Ctrl +/- făcut de utilizator)
    zoom_applied: f32,
    /// cadre de așteptare după o schimbare de zoom: dimensiunea ferestrei raportată de egui rămâne în urmă
    zoom_wait: u8,
    /// plasă de siguranță: (început fereastră, număr de schimbări automate)
    zoom_flips: Option<(std::time::Instant, u8)>,
    gd: guardian_ui::GuardianUi,
}

const ZOOM_MIN: f32 = 0.6;
const ZOOM_MAX: f32 = 3.0;
const ZOOM_STEP: f32 = 0.1;

/// Ca `auto_zoom`, dar cu histerezis: se schimbă doar dacă diferența față de factorul curent e de cel puțin 0,1,
/// ca să nu pâlpâie la limita dintre două trepte.
fn auto_zoom_hysteresis(current: f32, w: f32, h: f32) -> f32 {
    let target = auto_zoom(w, h);
    if (target - current).abs() >= 0.095 {
        target
    } else {
        current
    }
}

/// Factorul de zoom automat pentru o fereastră de `w`×`h` puncte logice native:
/// textul crește ca să umple ecranul (interfața e gândită pentru ~1100×680).
fn auto_zoom(w: f32, h: f32) -> f32 {
    (((w / 1100.0).min(h / 680.0) * 20.0).round() / 20.0).clamp(1.0, 2.2)
}

impl App {
    /// Încarcă o sesiune `.veriga`: rezultate + start; setările de securitate rămân cele curente.
    fn open_session(&mut self, p: &std::path::Path) {
        match Session::load(p) {
            Ok(mut s) => {
                self.results = s.results.clone();
                self.start = s.config.start.join("\n");
                self.adopt_security(&mut s.config);
                self.cfg = s.config.clone();
                self.session = Some(s);
                self.status_line = format!("{} {}", self.t("Încărcat:", "Loaded:"), p.display());
            }
            Err(e) => self.status_line = format!("{}: {e}", self.t("Eroare", "Error")),
        }
    }

    fn new() -> Self {
        Self {
            start: String::new(),
            cfg: Config::default(),
            exclude: String::new(),
            no_follow: String::new(),
            only: String::new(),
            internal_hosts: String::new(),
            ignore_status: String::new(),
            headers: String::new(),
            auth: String::new(),
            orphan_dir: String::new(),
            orphan_base: String::new(),
            max_depth_on: false,
            max_depth: 5,
            show_options: false,
            job: Job::Idle,
            results: vec![],
            session: None,
            progress: Progress::default(),
            phase: String::new(),
            status_line: String::new(),
            status_err: false,
            last_title: String::new(),
            filter: Filter::All,
            search: String::new(),
            sort: SortBy::Order,
            sort_desc: false,
            selected: None,
            paused: false,
            english: veriga_core::i18n::detect() == "en",
            zoom: None,
            zoom_applied: 1.0,
            zoom_wait: 0,
            zoom_flips: None,
            gd: guardian_ui::GuardianUi::default(),
        }
    }

    fn lang(&self) -> &'static str {
        if self.english { "en" } else { "ro" }
    }

    fn t(&self, ro: &'static str, en: &'static str) -> &'static str {
        if self.english { en } else { ro }
    }

    fn lines(s: &str) -> Vec<String> {
        s.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }

    fn build_config(&self) -> Result<Config, String> {
        let mut c = self.cfg.clone();
        c.start = Self::lines(&self.start);
        if c.start.is_empty() {
            return Err(self
                .t(
                    "Introdu cel puțin un URL sau o cale.",
                    "Enter at least one URL or path.",
                )
                .to_string());
        }
        c.max_depth = if self.max_depth_on {
            Some(self.max_depth)
        } else {
            None
        };
        c.exclude = Self::lines(&self.exclude);
        c.no_follow = Self::lines(&self.no_follow);
        c.include_only = Self::lines(&self.only);
        c.internal_hosts = Self::lines(&self.internal_hosts);
        c.ignore_status = self
            .ignore_status
            .split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        c.headers.clear();
        for l in Self::lines(&self.headers) {
            if let Some((k, v)) = l.split_once(':') {
                c.headers.push((k.trim().into(), v.trim().into()));
            }
        }
        c.basic_auth = self
            .auth
            .split_once(':')
            .map(|(u, p)| (u.to_string(), p.to_string()));
        if !self.orphan_dir.trim().is_empty() {
            c.orphan = Some(OrphanConfig {
                dir: PathBuf::from(self.orphan_dir.trim()),
                base_url: if self.orphan_base.trim().is_empty() {
                    None
                } else {
                    Some(self.orphan_base.trim().to_string())
                },
                ..Default::default()
            });
        } else {
            c.orphan = None;
        }
        c.lang = self.lang().into();
        Ok(c)
    }

    fn spawn_job(
        &mut self,
        work: impl FnOnce(
            Arc<Control>,
            tokio::sync::mpsc::UnboundedSender<Event>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = anyhow::Result<Session>> + Send>,
        > + Send
        + 'static,
    ) {
        let (tx, rx) = unbounded_channel();
        let ctl = Control::new();
        let (rtx, rrx) = std::sync::mpsc::channel();
        let ctl2 = ctl.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("runtime");
            let r = rt.block_on(work(ctl2, tx));
            let _ = rtx.send(r);
        });
        self.job = Job::Running {
            rx,
            ctl,
            result: rrx,
        };
        self.paused = false;
    }

    fn start_check(&mut self) {
        match self.build_config() {
            Err(e) => {
                self.status_line = e;
                self.status_err = true;
            }
            Ok(cfg) => {
                self.status_err = false;
                self.filter = Filter::All;
                self.results.clear();
                self.session = None;
                self.selected = None;
                self.progress = Progress::default();
                self.status_line.clear();
                self.spawn_job(move |ctl, tx| Box::pin(veriga_core::run(cfg, ctl, Some(tx))));
            }
        }
    }

    /// Setările de securitate se iau mereu din interfață, niciodată dintr-un fișier de sesiune.
    fn adopt_security(&self, c: &mut Config) {
        c.proxy = self.cfg.proxy.clone();
        c.ignore_tls_errors = self.cfg.ignore_tls_errors;
        c.block_private = self.cfg.block_private;
        c.user_agent = self.cfg.user_agent.clone();
        c.headers = Self::lines(&self.headers)
            .iter()
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().into(), v.trim().into()))
            .collect();
        c.basic_auth = self
            .auth
            .split_once(':')
            .map(|(u, p)| (u.to_string(), p.to_string()));
        c.retries = c.retries.min(veriga_core::check::MAX_RETRIES);
    }

    fn start_recheck(&mut self) {
        let Some(mut s) = self.session.clone() else {
            return;
        };
        self.adopt_security(&mut s.config);
        self.progress = Progress::default();
        self.spawn_job(move |ctl, tx| Box::pin(veriga_core::recheck(s, ctl, Some(tx))));
    }

    fn poll(&mut self) {
        let mut finished: Option<anyhow::Result<Session>> = None;
        if let Job::Running { rx, result, .. } = &mut self.job {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    Event::Checked(r) => {
                        // rezultatele se actualizează la final (recheck) — în timpul rulării doar se adaugă
                        self.results.push(*r);
                    }
                    Event::Progress(p) => self.progress = p,
                    Event::Phase(p) => self.phase = p,
                    Event::Started | Event::Finished => {}
                }
            }
            if let Ok(r) = result.try_recv() {
                finished = Some(r);
            }
        }
        if let Some(r) = finished {
            self.job = Job::Idle;
            match r {
                Ok(s) => {
                    self.results = s.results.clone();
                    let sm = report::Summary::from(&s);
                    self.status_line = format!(
                        "{}: {} URL · {} {} · {} {}",
                        self.t("Terminat", "Finished"),
                        sm.total,
                        sm.broken,
                        self.t("rupte", "broken"),
                        sm.redirected,
                        self.t("redirecționări", "redirects")
                    );
                    // după verificare, cele rupte sunt primul lucru de văzut
                    self.filter = if sm.broken > 0 {
                        Filter::Broken
                    } else {
                        Filter::All
                    };
                    self.selected = None;
                    self.session = Some(s);
                }
                Err(e) => {
                    self.status_line = format!("{}: {e}", self.t("Eroare", "Error"));
                    self.status_err = true;
                }
            }
        }
    }

    /// Scurtături: Ctrl+Enter pornește, Ctrl+F caută, Esc golește căutarea, ↑/↓ aleg rândul.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let (go, find, esc, up, down) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::Enter),
                i.modifiers.command && i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if go && !self.running() {
            self.start_check();
        }
        if find {
            ctx.memory_mut(|m| m.request_focus(egui::Id::unique("veriga_search")));
        }
        if esc && !self.search.is_empty() {
            self.search.clear();
        }
        if (up || down) && !ctx.egui_wants_keyboard_input() {
            let vis = self.visible();
            if !vis.is_empty() {
                let pos = self.selected.and_then(|s| vis.iter().position(|&v| v == s));
                let next = match (pos, down) {
                    (None, true) => 0,
                    (None, false) => vis.len() - 1,
                    (Some(p), true) => (p + 1).min(vis.len() - 1),
                    (Some(p), false) => p.saturating_sub(1),
                };
                self.selected = Some(vis[next]);
            }
        }
    }

    /// Titlul ferestrei arată progresul / rezultatul (vizibil și când fereastra e în fundal).
    fn update_title(&mut self, ctx: &egui::Context) {
        let broken = if self.running() {
            self.progress.broken
        } else {
            self.results
                .iter()
                .filter(|r| r.status.is_broken(&self.cfg))
                .count()
        };
        let title = if self.running() {
            format!("{} {} · VERIGA", self.t("⏳ rupte:", "⏳ broken:"), broken)
        } else if self.results.is_empty() {
            "VERIGA".to_string()
        } else if broken > 0 {
            format!("✖ {broken} · VERIGA")
        } else {
            "✔ VERIGA".to_string()
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }

    fn summary_banner(&self, ui: &mut egui::Ui) {
        let bad = self
            .results
            .iter()
            .filter(|r| r.status.is_broken(&self.cfg))
            .count();
        let (color, text) = if self.running() {
            (
                ui.visuals().selection.bg_fill,
                format!(
                    "⏳ {} · {} {}",
                    self.t("Se verifică…", "Checking…"),
                    bad,
                    self.t("rupte până acum", "broken so far")
                ),
            )
        } else if bad == 0 {
            (
                Color32::from_rgb(70, 170, 110),
                format!("✔ {}", self.t("Niciun link rupt", "No broken links")),
            )
        } else {
            let pages = self
                .results
                .iter()
                .filter(|r| r.status.is_broken(&self.cfg))
                .flat_map(|r| r.parents.iter().map(|p| p.url.as_str()))
                .collect::<std::collections::HashSet<_>>()
                .len();
            (
                ui.visuals().error_fg_color,
                format!(
                    "✖ {bad} {}{}",
                    if bad == 1 {
                        self.t("link rupt", "broken link")
                    } else {
                        self.t("linkuri rupte", "broken links")
                    },
                    if pages > 0 {
                        format!(
                            " {} {pages} {}",
                            self.t("în", "on"),
                            if pages == 1 {
                                self.t("pagină", "page")
                            } else {
                                self.t("pagini", "pages")
                            }
                        )
                    } else {
                        String::new()
                    }
                ),
            )
        };
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, color))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(text).strong().color(color).size(16.0));
                    ui.label(
                        RichText::new(format!(
                            "{} URL · {}",
                            self.results.len(),
                            self.t("click pe un rând pentru detalii", "click a row for details")
                        ))
                        .weak(),
                    );
                });
            });
        ui.add_space(4.0);
    }

    fn empty_state(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() / 4.0).max(10.0));
            ui.heading(self.t("Verifică linkurile unui site", "Check a site's links"));
            ui.add_space(8.0);
            ui.label(self.t(
                "1. Scrie o adresă (https://…) sau alege un folder cu un site salvat",
                "1. Type an address (https://…) or choose a folder with a saved site",
            ));
            ui.label(self.t(
                "2. Apasă Pornește (Ctrl+Enter)",
                "2. Press Start (Ctrl+Enter)",
            ));
            ui.label(self.t(
                "3. Linkurile rupte apar primele, cu pagina în care se află",
                "3. Broken links appear first, with the page they are on",
            ));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - 330.0) / 2.0).max(0.0));
                if ui
                    .button(self.t("📁 Alege folder…", "📁 Choose folder…"))
                    .clicked()
                    && let Some(d) = rfd::FileDialog::new().pick_folder()
                {
                    self.start = d.display().to_string();
                    self.start_check();
                }
                if ui
                    .button(self.t("📂 Deschide o sesiune salvată", "📂 Open a saved session"))
                    .clicked()
                    && let Some(p) = rfd::FileDialog::new()
                        .add_filter("VERIGA", &["veriga", "json"])
                        .pick_file()
                {
                    self.open_session(&p);
                }
            });
            ui.add_space(10.0);
            ui.label(
                RichText::new(self.t(
                    "Ctrl+F caută · ↑/↓ aleg rândul · Ctrl +/- zoom",
                    "Ctrl+F search · ↑/↓ select row · Ctrl +/- zoom",
                ))
                .weak(),
            );
        });
    }

    fn running(&self) -> bool {
        matches!(self.job, Job::Running { .. })
    }

    fn matches(&self, r: &LinkResult) -> bool {
        let ok = match self.filter {
            Filter::All => true,
            Filter::Broken => r.status.is_broken(&self.cfg),
            Filter::Redirects => r.status == Status::Redirected,
            Filter::Ok => r.status == Status::Ok,
            Filter::Orphans => r.status == Status::Orphan,
            Filter::Skipped => matches!(r.status, Status::Skipped | Status::RobotsBlocked),
        };
        if !ok {
            return false;
        }
        if self.search.is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        r.url.to_lowercase().contains(&q)
            || r.title.as_deref().unwrap_or("").to_lowercase().contains(&q)
            || r.message.to_lowercase().contains(&q)
    }

    fn visible(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.results.len())
            .filter(|&i| self.matches(&self.results[i]))
            .collect();
        let lang = self.lang();
        match self.sort {
            SortBy::Order => {}
            SortBy::Url => v.sort_by(|&a, &b| self.results[a].url.cmp(&self.results[b].url)),
            SortBy::Status => v.sort_by(|&a, &b| {
                self.results[a]
                    .status
                    .label(lang)
                    .cmp(self.results[b].status.label(lang))
            }),
            SortBy::Kind => v.sort_by(|&a, &b| self.results[a].kind.cmp(&self.results[b].kind)),
            SortBy::Size => v.sort_by_key(|&i| self.results[i].size.unwrap_or(0)),
            SortBy::Depth => v.sort_by_key(|&i| self.results[i].depth),
            SortBy::Time => v.sort_by_key(|&i| self.results[i].elapsed_ms),
        }
        if self.sort_desc {
            v.reverse();
        }
        v
    }

    fn export(&mut self, kind: &str) {
        let Some(s) = &self.session else {
            self.status_line = self
                .t("Nu există rezultate de exportat.", "Nothing to export.")
                .into();
            return;
        };
        let en = self.english;
        let tr = |ro: &'static str, e: &'static str| if en { e } else { ro };
        let (name, ext, data): (&str, &str, Vec<u8>) = match kind {
            "html" => (
                tr("Raport HTML", "HTML report"),
                "html",
                report::to_html(s).into_bytes(),
            ),
            "csv" => ("CSV", "csv", report::to_csv(s, b',', false).into_bytes()),
            "csvb" => (
                tr("CSV — rupte", "CSV — broken"),
                "csv",
                report::to_csv(s, b',', true).into_bytes(),
            ),
            "tsv" => ("TSV", "tsv", report::to_csv(s, b'\t', false).into_bytes()),
            "json" => ("JSON", "json", report::to_json(s).into_bytes()),
            "txt" => ("Text", "txt", report::to_text(s).into_bytes()),
            "xml" => ("Sitemap XML", "xml", report::to_sitemap_xml(s).into_bytes()),
            "junit" => (
                "JUnit XML",
                "xml",
                veriga_core::junit::to_junit(s).into_bytes(),
            ),
            "sarif" => (
                "SARIF",
                "sarif",
                veriga_core::sarif::to_sarif(s).into_bytes(),
            ),
            "tree" => (
                tr("Arbore", "Tree"),
                "txt",
                report::to_tree_text(s, 12).into_bytes(),
            ),
            _ => return,
        };
        if let Some(p) = rfd::FileDialog::new()
            .set_title(name)
            .set_file_name(format!("{}.{ext}", tr("veriga-raport", "veriga-report")))
            .add_filter(name, &[ext])
            .save_file()
        {
            self.status_line = match std::fs::write(&p, data) {
                Ok(_) => format!("{} {}", self.t("Salvat:", "Saved:"), p.display()),
                Err(e) => format!("{}: {e}", self.t("Eroare la salvare", "Save error")),
            };
        }
    }
}

/// Pagina-părinte într-o formă scurtă: fără gazda comună, iar pentru fișiere locale ultimele două segmente.
fn short_page(parent: &str, child: &str) -> String {
    if let Some(rest) = parent.strip_prefix("file://") {
        let segs: Vec<&str> = rest.trim_end_matches('/').rsplit('/').take(2).collect();
        return segs.into_iter().rev().collect::<Vec<_>>().join("/");
    }
    let host = |u: &str| {
        u.split_once("://")
            .map(|(_, r)| r.split('/').next().unwrap_or("").to_string())
    };
    match (host(parent), host(child)) {
        (Some(a), Some(b)) if a == b => {
            let path = parent
                .split_once("://")
                .map(|(_, r)| &r[a.len()..])
                .unwrap_or(parent);
            if path.is_empty() {
                "/".to_string()
            } else {
                path.to_string()
            }
        }
        _ => parent.to_string(),
    }
}

fn open_in_browser(url: &str) {
    // doar http/https/file; fără interpretor de comenzi (cmd /C start era injectabil)
    let Ok(u) = url::Url::parse(url) else { return };
    if !matches!(u.scheme(), "http" | "https" | "file") {
        return;
    }
    let url = u.as_str();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

fn status_color(r: &LinkResult, cfg: &Config) -> Color32 {
    if r.status.is_broken(cfg) {
        Color32::from_rgb(214, 69, 65)
    } else if r.status == Status::Ok {
        Color32::from_rgb(60, 160, 100)
    } else if matches!(r.status, Status::Redirected | Status::Orphan) {
        Color32::from_rgb(217, 150, 30)
    } else {
        Color32::GRAY
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        self.poll();
        veriga_core::i18n::set(self.lang()); // mesajele din nucleu urmează limba aleasă în interfață
        if self.running() {
            // 4 fps: suficient pentru progres; 10 fps ținea un nucleu ocupat la randare software
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        // zoom: Ctrl +/- făcut de utilizator trece pe manual; altfel automat după dimensiunea ferestrei
        let cz = ctx.zoom_factor();
        if (cz - self.zoom_applied).abs() > 0.001 {
            self.zoom = Some(cz.clamp(ZOOM_MIN, ZOOM_MAX));
        }
        let want = match self.zoom {
            Some(z) => z,
            None if self.zoom_wait > 0 => {
                // `inner_rect` (în puncte) rămâne în urmă cu 1–2 cadre după o schimbare de zoom: dacă l-am converti acum
                // cu factorul nou am obține o dimensiune greșită și zoomul ar oscila (reprodus sub scalare DPI).
                self.zoom_wait -= 1;
                ctx.request_repaint();
                cz
            }
            None => {
                let k = ctx.pixels_per_point() / ctx.native_pixels_per_point().unwrap_or(1.0);
                match ctx.input(|i| i.viewport().inner_rect) {
                    Some(r) => auto_zoom_hysteresis(cz, r.width() * k, r.height() * k),
                    None => cz, // fără dimensiune: păstrăm valoarea curentă, nu revenim la 100 %
                }
            }
        };
        let mut want = want;
        if self.zoom.is_none() && (want - cz).abs() > 0.001 {
            let now = std::time::Instant::now();
            let (t0, n) = match self.zoom_flips {
                Some((t0, n)) if now.duration_since(t0).as_secs() < 5 => (t0, n + 1),
                _ => (now, 1),
            };
            self.zoom_flips = Some((t0, n));
            if n > 4 {
                // oscilație: renunțăm la zoomul automat și păstrăm factorul curent
                self.zoom = Some(cz.clamp(ZOOM_MIN, ZOOM_MAX));
                want = cz;
            }
        }
        if (want - cz).abs() > 0.001 {
            if std::env::var_os("VERIGA_ZOOM_DEBUG").is_some() {
                eprintln!(
                    "zoom {cz:.2} -> {want:.2} (manual: {})",
                    self.zoom.is_some()
                );
            }
            ctx.set_zoom_factor(want);
            self.zoom_wait = 4;
        }
        self.zoom_applied = want;
        let lang = self.lang();
        self.shortcuts(ctx);
        self.update_title(ctx);
        {
            let eng = self.english;
            let sess = self.session.take();
            self.gd.show(ctx, &sess, eng);
            self.session = sess;
        }

        egui::Panel::top("top").show(root, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("VERIGA").strong().size(18.0));
                ui.separator();
                ui.toggle_value(&mut self.gd.open, "🛡 Guardian");
                ui.separator();
                let running = self.running();
                if !running {
                    let go =
                        egui::Button::new(RichText::new(self.t("▶ Pornește", "▶ Start")).strong())
                            .fill(ui.visuals().selection.bg_fill);
                    if ui
                        .add(go)
                        .on_hover_text(self.t(
                            "Pornește verificarea (Ctrl+Enter)",
                            "Start the check (Ctrl+Enter)",
                        ))
                        .clicked()
                    {
                        self.start_check();
                    }
                    if ui
                        .add_enabled(
                            self.session
                                .as_ref()
                                .map(|s| s.broken_count() > 0)
                                .unwrap_or(false),
                            egui::Button::new(self.t("↻ Reverifică rupte", "↻ Recheck broken")),
                        )
                        .clicked()
                    {
                        self.start_recheck();
                    }
                } else if let Job::Running { ctl, .. } = &self.job {
                    if self.paused {
                        if ui.button(self.t("⏵ Reia", "⏵ Resume")).clicked() {
                            ctl.resume();
                            self.paused = false;
                        }
                    } else if ui.button(self.t("⏸ Pauză", "⏸ Pause")).clicked() {
                        ctl.pause();
                        self.paused = true;
                    }
                    if ui.button(self.t("■ Oprește", "■ Stop")).clicked() {
                        ctl.cancel();
                    }
                }
                ui.separator();
                if ui
                    .button(self.t("📂 Deschide sesiune", "📂 Open session"))
                    .clicked()
                    && let Some(p) = rfd::FileDialog::new()
                        .add_filter("VERIGA", &["veriga", "json"])
                        .pick_file()
                {
                    self.open_session(&p);
                }
                if ui
                    .button(self.t("💾 Salvează sesiune", "💾 Save session"))
                    .clicked()
                    && let Some(s) = &self.session
                    && let Some(p) = rfd::FileDialog::new()
                        .set_file_name("sesiune.veriga")
                        .add_filter("VERIGA", &["veriga"])
                        .save_file()
                {
                    self.status_line = match s.save(&p) {
                        Ok(_) => format!("{} {}", self.t("Salvat:", "Saved:"), p.display()),
                        Err(e) => format!("{}: {e}", self.t("Eroare", "Error")),
                    };
                }
                ui.menu_button(self.t("📤 Exportă", "📤 Export"), |ui| {
                    for (k, l) in [
                        ("html", self.t("Raport HTML", "HTML report")),
                        ("csv", "CSV"),
                        ("csvb", self.t("CSV (doar rupte)", "CSV (broken only)")),
                        ("tsv", "TSV"),
                        ("json", "JSON"),
                        ("txt", "Text"),
                        ("xml", "Sitemap XML"),
                        ("sarif", "SARIF"),
                        ("junit", "JUnit XML"),
                        ("tree", self.t("Arbore site", "Site tree")),
                    ] {
                        if ui.button(l).clicked() {
                            self.export(k);
                            ui.close();
                        }
                    }
                });
                let opt_label = self.t("⚙ Opțiuni", "⚙ Options");
                ui.toggle_value(&mut self.show_options, opt_label);
                ui.checkbox(&mut self.english, "EN");
            });
            ui.horizontal(|ui| {
                ui.label(self.t(
                    "Start (un URL / cale pe linie):",
                    "Start (one URL / path per line):",
                ));
            });
            let start_hint = self.t(
                "https://exemplu.ro/   sau   /cale/catre/site",
                "https://example.com/   or   /path/to/site",
            );
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 150.0).max(120.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.start)
                        .desired_rows(1)
                        .desired_width(w)
                        .hint_text(start_hint),
                );
                if ui
                    .button(self.t("📁 Alege folder…", "📁 Choose folder…"))
                    .on_hover_text(self.t(
                        "Verifică un site salvat local (folder cu index.html)",
                        "Check a locally saved site (folder with index.html)",
                    ))
                    .clicked()
                    && let Some(d) = rfd::FileDialog::new().pick_folder()
                {
                    self.start = d.display().to_string();
                }
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("bottom").show(root, |ui| {
            ui.horizontal(|ui| {
                if self.running() {
                    ui.spinner();
                    ui.label(format!(
                        "{}: {} · {}: {} · {}: {} · {}: {} · {:.0}s {}",
                        self.t("verificate", "checked"),
                        self.progress.checked,
                        self.t("în coadă", "queued"),
                        self.progress.queued,
                        self.t("active", "active"),
                        self.progress.in_flight,
                        self.t("rupte", "broken"),
                        self.progress.broken,
                        self.progress.elapsed_ms as f64 / 1000.0,
                        if self.paused {
                            self.t("(pauză)", "(paused)")
                        } else {
                            ""
                        }
                    ));
                    if !self.phase.is_empty() {
                        ui.label(RichText::new(&self.phase).weak());
                    }
                } else if self.status_err {
                    ui.label(
                        RichText::new(format!("⚠ {}", self.status_line))
                            .color(ui.visuals().error_fg_color),
                    );
                } else {
                    ui.label(&self.status_line);
                }
            });
        });

        if self.show_options {
            egui::Panel::left("opts").min_size(320.0).show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(self.t("Opțiuni", "Options"));
                    let c = &mut self.cfg;
                    ui.add(
                        egui::Slider::new(&mut c.threads, 1..=200).text(if lang == "en" {
                            "threads"
                        } else {
                            "fire"
                        }),
                    );
                    ui.add(
                        egui::Slider::new(&mut c.per_host, 1..=50).text(if lang == "en" {
                            "per host"
                        } else {
                            "pe gazdă"
                        }),
                    );
                    ui.add(egui::Slider::new(&mut c.timeout_secs, 3..=120).text("timeout (s)"));
                    ui.add(
                        egui::Slider::new(&mut c.retries, 0..=6).text(if lang == "en" {
                            "retries"
                        } else {
                            "reîncercări"
                        }),
                    );
                    ui.add(
                        egui::Slider::new(&mut c.max_redirects, 1..=30).text(if lang == "en" {
                            "max redirects"
                        } else {
                            "redirecționări max."
                        }),
                    );
                    ui.add(
                        egui::Slider::new(&mut c.delay_ms, 0..=5000).text(if lang == "en" {
                            "delay (ms)"
                        } else {
                            "pauză (ms)"
                        }),
                    );
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.max_depth_on,
                            if lang == "en" {
                                "max depth"
                            } else {
                                "adâncime max."
                            },
                        );
                        ui.add_enabled(
                            self.max_depth_on,
                            egui::DragValue::new(&mut self.max_depth).range(0..=50),
                        );
                    });
                    ui.separator();
                    ui.checkbox(
                        &mut c.check_external,
                        if lang == "en" {
                            "Check external links"
                        } else {
                            "Verifică linkuri externe"
                        },
                    );
                    ui.checkbox(
                        &mut c.stay_in_dir,
                        if lang == "en" {
                            "Stay in start directory"
                        } else {
                            "Rămâi în directorul de start"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_resources,
                        if lang == "en" {
                            "Check images/CSS/JS/media"
                        } else {
                            "Verifică imagini/CSS/JS/media"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_forms,
                        if lang == "en" {
                            "Check form actions"
                        } else {
                            "Verifică acțiuni de formular"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_mailto,
                        if lang == "en" {
                            "Check mailto:"
                        } else {
                            "Verifică mailto:"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_ftp,
                        if lang == "en" {
                            "Check ftp://"
                        } else {
                            "Verifică ftp://"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_fragments,
                        if lang == "en" {
                            "Check #anchors"
                        } else {
                            "Verifică ancore #"
                        },
                    );
                    ui.checkbox(
                        &mut c.treat_redirects_as_errors,
                        if lang == "en" {
                            "Redirects are errors"
                        } else {
                            "Redirecționările sunt erori"
                        },
                    );
                    ui.checkbox(
                        &mut c.respect_robots,
                        if lang == "en" {
                            "Obey robots.txt"
                        } else {
                            "Respectă robots.txt"
                        },
                    );
                    ui.checkbox(
                        &mut c.seed_sitemaps,
                        if lang == "en" {
                            "Seed from sitemap.xml"
                        } else {
                            "Pornește și din sitemap.xml"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_cert_expiry,
                        if lang == "en" {
                            "Warn about expiring TLS certificates"
                        } else {
                            "Avertizează la certificate TLS care expiră"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_sri,
                        if lang == "en" {
                            "Verify subresource integrity"
                        } else {
                            "Verifică integritatea subresurselor"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_feeds,
                        if lang == "en" {
                            "Read RSS/Atom feeds"
                        } else {
                            "Citește fluxuri RSS/Atom"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_mixed_content,
                        if lang == "en" {
                            "Detect mixed content"
                        } else {
                            "Detectează conținut mixt"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_seo_links,
                        if lang == "en" {
                            "Check canonical and hreflang"
                        } else {
                            "Verifică canonic și hreflang"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_link_header,
                        if lang == "en" {
                            "Check HTTP Link header"
                        } else {
                            "Verifică antetul HTTP Link"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_link_names,
                        if lang == "en" {
                            "Check link names (accessibility)"
                        } else {
                            "Verifică numele legăturilor (accesibilitate)"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_ambiguous_names,
                        if lang == "en" {
                            "Flag ambiguous link names"
                        } else {
                            "Semnalează nume de legături ambigue"
                        },
                    );
                    ui.checkbox(
                        &mut c.check_nonweb,
                        if lang == "en" {
                            "Validate tel:/geo:/data: syntax"
                        } else {
                            "Validează sintaxa tel:/geo:/data:"
                        },
                    );
                    ui.checkbox(
                        &mut c.honor_nofollow,
                        if lang == "en" {
                            "Do not crawl rel=nofollow targets"
                        } else {
                            "Nu parcurge ținte rel=nofollow"
                        },
                    );
                    ui.checkbox(
                        &mut c.ignore_tls_errors,
                        if lang == "en" {
                            "Ignore TLS errors"
                        } else {
                            "Ignoră erori TLS"
                        },
                    );
                    ui.checkbox(
                        &mut c.block_private,
                        if lang == "en" {
                            "Block private/local addresses (anti-SSRF)"
                        } else {
                            "Blochează adrese private/locale (anti-SSRF)"
                        },
                    );
                    ui.checkbox(&mut c.cookies, "Cookies");
                    ui.checkbox(
                        &mut c.head_first,
                        if lang == "en" {
                            "HEAD before GET"
                        } else {
                            "HEAD înainte de GET"
                        },
                    );
                    ui.separator();
                    ui.label("User-Agent");
                    ui.text_edit_singleline(&mut c.user_agent);
                    ui.label("Proxy");
                    let mut p = c.proxy.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut p).changed() {
                        c.proxy = if p.trim().is_empty() { None } else { Some(p) };
                    }
                    let en = lang == "en";
                    let current = veriga_core::proxy::PRESETS
                        .iter()
                        .find(|x| c.proxy.as_deref() == Some(x.url));
                    egui::ComboBox::from_id_salt("proxy_preset")
                        .selected_text(match current {
                            Some(x) => (if en { x.label_en } else { x.label }).to_string(),
                            None => {
                                if en {
                                    "Free ethical proxy…".into()
                                } else {
                                    "Proxy gratuit etic…".into()
                                }
                            }
                        })
                        .show_ui(ui, |ui| {
                            for x in veriga_core::proxy::PRESETS.iter() {
                                if ui
                                    .selectable_label(
                                        current.map(|y| y.id) == Some(x.id),
                                        if en { x.label_en } else { x.label },
                                    )
                                    .clicked()
                                {
                                    veriga_core::proxy::apply(c, x);
                                }
                            }
                        });
                    if let Some(x) = current {
                        ui.small(if en { x.requires_en } else { x.requires_ro });
                        ui.small(if en { x.caveat_en } else { x.caveat_ro });
                    }
                    ui.label(if lang == "en" {
                        "Basic auth (user:password)"
                    } else {
                        "Autentificare Basic (utilizator:parolă)"
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.auth).password(true));
                    ui.label(if lang == "en" {
                        "Extra headers (Name: value, one per line)"
                    } else {
                        "Antete suplimentare (Nume: valoare, câte unul pe linie)"
                    });
                    ui.text_edit_multiline(&mut self.headers);
                    ui.separator();
                    ui.label(if lang == "en" {
                        "Exclude (regex, one per line)"
                    } else {
                        "Exclude (regex, câte unul pe linie)"
                    });
                    ui.text_edit_multiline(&mut self.exclude);
                    ui.label(if lang == "en" {
                        "Check but don't follow (regex)"
                    } else {
                        "Verifică, dar nu parcurge (regex)"
                    });
                    ui.text_edit_multiline(&mut self.no_follow);
                    ui.label(if lang == "en" {
                        "Follow only matching (regex)"
                    } else {
                        "Parcurge doar ce se potrivește (regex)"
                    });
                    ui.text_edit_multiline(&mut self.only);
                    ui.label(if lang == "en" {
                        "Extra internal hosts"
                    } else {
                        "Gazde interne suplimentare"
                    });
                    ui.text_edit_multiline(&mut self.internal_hosts);
                    ui.label(if lang == "en" {
                        "HTTP codes treated as OK (e.g. 999)"
                    } else {
                        "Coduri HTTP tratate ca OK (ex. 999)"
                    });
                    ui.text_edit_singleline(&mut self.ignore_status);
                    ui.separator();
                    ui.label(if lang == "en" {
                        "Orphan files — local directory"
                    } else {
                        "Fișiere orfane — director local"
                    });
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.orphan_dir);
                        if ui.button("…").clicked()
                            && let Some(d) = rfd::FileDialog::new().pick_folder()
                        {
                            self.orphan_dir = d.display().to_string();
                        }
                    });
                    ui.label(if lang == "en" {
                        "Base URL of that directory (optional)"
                    } else {
                        "URL-ul rădăcinii acelui director (opțional)"
                    });
                    ui.text_edit_singleline(&mut self.orphan_base);
                });
            });
        }

        // Detalii
        let sel = self.selected.and_then(|i| self.results.get(i)).cloned();
        if let Some(r) = &sel {
            egui::Panel::bottom("detail")
                .resizable(true)
                .min_size(90.0)
                .default_size(170.0)
                .show(root, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&r.url).strong());
                            if ui
                                .small_button("📋")
                                .on_hover_text(self.t("Copiază URL", "Copy URL"))
                                .clicked()
                            {
                                ui.ctx().copy_text(r.url.clone());
                            }
                            if ui
                                .small_button("↗")
                                .on_hover_text(self.t("Deschide în browser", "Open in browser"))
                                .clicked()
                            {
                                open_in_browser(&r.url);
                            }
                        });
                        ui.label(format!(
                            "{} · {}{}",
                            r.status.label(lang),
                            r.http_status
                                .map(|c| format!("HTTP {c} · "))
                                .unwrap_or_default(),
                            r.message
                        ));
                        if let Some(f) = &r.final_url {
                            ui.label(format!("→ {f}"));
                        }
                        if !r.chain.is_empty() {
                            ui.label(
                                r.chain
                                    .iter()
                                    .map(|h| format!("{} {}", h.code, h.url))
                                    .collect::<Vec<_>>()
                                    .join("  →  "),
                            );
                        }
                        ui.separator();
                        ui.label(RichText::new(self.t("Găsit pe:", "Found on:")).strong());
                        for p in &r.parents {
                            ui.horizontal(|ui| {
                                if ui.link(&p.url).clicked() {
                                    open_in_browser(&p.url);
                                }
                                if !p.text.is_empty() {
                                    ui.label(RichText::new(format!("„{}”", p.text)).weak());
                                }
                            });
                        }
                    });
                });
        }

        egui::Panel::bottom("zoom").show(root, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - 300.0) / 2.0).max(0.0));
                let cur = self.zoom_applied;
                let out = egui::Button::new("🔍➖");
                if ui
                    .add_enabled(cur > ZOOM_MIN + 0.001, out)
                    .on_hover_text(self.t("Micșorează (zoom out)", "Zoom out"))
                    .clicked()
                {
                    self.zoom = Some((((cur - ZOOM_STEP) * 10.0).round() / 10.0).max(ZOOM_MIN));
                }
                ui.label(
                    RichText::new(format!("{:>3.0}%", cur * 100.0))
                        .strong()
                        .monospace(),
                );
                let inn = egui::Button::new("🔍➕");
                if ui
                    .add_enabled(cur < ZOOM_MAX - 0.001, inn)
                    .on_hover_text(self.t("Mărește (zoom in)", "Zoom in"))
                    .clicked()
                {
                    self.zoom = Some((((cur + ZOOM_STEP) * 10.0).round() / 10.0).min(ZOOM_MAX));
                }
                if ui
                    .selectable_label(self.zoom.is_none(), "Auto")
                    .on_hover_text(self.t(
                        "Zoom automat: potrivește ecranul",
                        "Automatic zoom: fit the screen",
                    ))
                    .clicked()
                {
                    self.zoom = None;
                    self.zoom_wait = 4;
                }
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(root, |ui| {
            if self.results.is_empty() && !self.running() {
                self.empty_state(ui);
                return;
            }
            self.summary_banner(ui);
            ui.horizontal(|ui| {
                let all = self.results.len();
                let broken = self
                    .results
                    .iter()
                    .filter(|r| r.status.is_broken(&self.cfg))
                    .count();
                let red = self
                    .results
                    .iter()
                    .filter(|r| r.status == Status::Redirected)
                    .count();
                let orph = self
                    .results
                    .iter()
                    .filter(|r| r.status == Status::Orphan)
                    .count();
                let l_all = format!("{} ({all})", self.t("Toate", "All"));
                let l_broken = format!("{} ({broken})", self.t("Rupte", "Broken"));
                let l_red = format!("{} ({red})", self.t("Redirecționări", "Redirects"));
                let l_orph = format!("{} ({orph})", self.t("Orfane", "Orphans"));
                let okc = self
                    .results
                    .iter()
                    .filter(|r| r.status == Status::Ok)
                    .count();
                let skc = self
                    .results
                    .iter()
                    .filter(|r| matches!(r.status, Status::Skipped | Status::RobotsBlocked))
                    .count();
                let l_ok = format!("OK ({okc})");
                let l_skip = format!("{} ({skc})", self.t("Omise", "Skipped"));
                let l_search = self.t("Caută…", "Search…");
                ui.selectable_value(&mut self.filter, Filter::All, l_all);
                ui.selectable_value(&mut self.filter, Filter::Broken, l_broken);
                ui.selectable_value(&mut self.filter, Filter::Redirects, l_red);
                ui.selectable_value(&mut self.filter, Filter::Ok, l_ok);
                ui.selectable_value(&mut self.filter, Filter::Orphans, l_orph);
                ui.selectable_value(&mut self.filter, Filter::Skipped, l_skip);
                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .id(egui::Id::unique("veriga_search"))
                        .hint_text(l_search)
                        .desired_width(200.0),
                );
            });
            ui.separator();

            let vis = self.visible();
            let cfg = self.cfg.clone();
            let mut clicked: Option<usize> = None;
            let mut new_sort: Option<SortBy> = None;
            let mut menu_open: Option<String> = None;

            // etichetele selectabile ar intercepta clicurile pe rând (selecția / meniul contextual)
            ui.style_mut().interaction.selectable_labels = false;
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .column(Column::initial(150.0).at_least(80.0))
                .column(Column::initial(50.0))
                .column(Column::initial(380.0).at_least(150.0).clip(true))
                .column(Column::remainder().at_least(160.0).clip(true))
                .column(Column::initial(90.0))
                .column(Column::initial(70.0))
                .column(Column::initial(200.0).clip(true))
                .column(Column::initial(45.0))
                .column(Column::initial(60.0))
                .header(22.0, |mut h| {
                    let heads: [(&str, &str, SortBy); 9] = [
                        ("Stare", "Status", SortBy::Status),
                        ("Cod", "Code", SortBy::Order),
                        ("URL", "URL", SortBy::Url),
                        ("Găsit pe", "Found on", SortBy::Order),
                        ("Tip", "Type", SortBy::Kind),
                        ("Mărime", "Size", SortBy::Size),
                        ("Titlu", "Title", SortBy::Order),
                        ("Niv.", "Lvl", SortBy::Depth),
                        ("ms", "ms", SortBy::Time),
                    ];
                    for (ro, en, s) in heads {
                        h.col(|ui| {
                            let label = if self.english { en } else { ro };
                            if ui.button(RichText::new(label).strong()).clicked() {
                                new_sort = Some(s);
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(20.0, vis.len(), |mut row| {
                        let i = vis[row.index()];
                        let r = &self.results[i];
                        row.set_selected(self.selected == Some(i));
                        row.col(|ui| {
                            ui.label(
                                RichText::new(r.status.label(lang)).color(status_color(r, &cfg)),
                            );
                        });
                        row.col(|ui| {
                            ui.label(r.http_status.map(|c| c.to_string()).unwrap_or_default());
                        });
                        row.col(|ui| {
                            ui.label(&r.url);
                        });
                        row.col(|ui| {
                            if let Some(p) = r.parents.first() {
                                let n = r
                                    .parents
                                    .iter()
                                    .map(|x| x.url.as_str())
                                    .collect::<std::collections::HashSet<_>>()
                                    .len();
                                let short = short_page(&p.url, &r.url);
                                let t = if n > 1 {
                                    format!("{short} (+{})", n - 1)
                                } else {
                                    short
                                };
                                ui.label(RichText::new(t).weak());
                            }
                        });
                        row.col(|ui| {
                            ui.label(r.kind.label(lang));
                        });
                        row.col(|ui| {
                            ui.label(r.size.map(report::human_size).unwrap_or_default());
                        });
                        row.col(|ui| {
                            ui.label(r.title.as_deref().unwrap_or(""));
                        });
                        row.col(|ui| {
                            ui.label(r.depth.to_string());
                        });
                        row.col(|ui| {
                            ui.label(r.elapsed_ms.to_string());
                        });
                        let resp = row.response();
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                        resp.context_menu(|ui| {
                            if ui
                                .button(if lang == "en" {
                                    "Copy URL"
                                } else {
                                    "Copiază URL"
                                })
                                .clicked()
                            {
                                ui.ctx().copy_text(r.url.clone());
                                ui.close();
                            }
                            if ui
                                .button(if lang == "en" {
                                    "Open in browser"
                                } else {
                                    "Deschide în browser"
                                })
                                .clicked()
                            {
                                menu_open = Some(r.url.clone());
                                ui.close();
                            }
                        });
                    });
                });

            if let Some(i) = clicked {
                self.selected = Some(i);
            }
            if let Some(u) = menu_open {
                open_in_browser(&u);
            }
            if let Some(s) = new_sort {
                if self.sort == s {
                    self.sort_desc = !self.sort_desc;
                } else {
                    self.sort = s;
                    self.sort_desc = false;
                }
            }
        });
    }
}

fn main() -> std::process::ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("veriga-gui {}", env!("CARGO_PKG_VERSION"));
            return std::process::ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            let ver = env!("CARGO_PKG_VERSION");
            if veriga_core::i18n::detect() == "en" {
                println!(
                    "VERIGA GUI {ver}\nUsage: veriga-gui [session.veriga | folder | https://address] [--start]\n  --start    start checking right away (with a folder or address)\n  --version  print the version\n  --help     this message\nThe graphical interface needs a desktop (display). For a terminal/server use `veriga` (CLI)."
                );
            } else {
                println!(
                    "VERIGA GUI {ver}\nUtilizare: veriga-gui [sesiune.veriga | folder | https://adresă] [--start]\n  --start    pornește verificarea imediat (cu un folder sau o adresă)\n  --version  afișează versiunea\n  --help     acest mesaj\nInterfața grafică are nevoie de un mediu desktop (afișaj). Pentru terminal/server folosiți `veriga` (CLI)."
                );
            }
            return std::process::ExitCode::SUCCESS;
        }
        _ => {}
    }
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            let e = e.to_string();
            let e = e.split(" at /").next().unwrap_or(&e); // fără căi din mediul de compilare
            let e = if e.contains("DISPLAY") {
                "nu există afișaj (DISPLAY/WAYLAND_DISPLAY)"
            } else {
                e
            };
            eprintln!(
                "veriga-gui: nu s-a putut porni interfața grafică ({e}).\nDacă nu există afișaj (server, SSH), folosiți CLI-ul: veriga --help"
            );
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_maximized(true)
            .with_min_inner_size([760.0, 480.0])
            .with_title("VERIGA — verificator de legături"),
        ..Default::default()
    };
    eframe::run_native(
        "VERIGA",
        opts,
        Box::new(|cc| {
            // Ubuntu-Light nu are săgeți (→ ↶ ↳); Hack le are, deci devine rezervă și pentru textul proporțional
            let mut fonts = egui::FontDefinitions::default();
            if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                f.push("Hack".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            let mut app = App::new();
            // asocierea de fișiere (PortableApps.com / dublu clic pe .veriga): primul argument = sesiune
            let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
            let auto = args.iter().any(|a| a == "--start");
            if let Some(a) = args.iter().find(|a| !a.to_string_lossy().starts_with("--")) {
                let p = std::path::PathBuf::from(a);
                if p.is_file() {
                    app.open_session(&p);
                } else {
                    // un folder sau o adresă: completează câmpul de start (`--start` pornește imediat)
                    app.start = a.to_string_lossy().into_owned();
                    if auto {
                        app.start_check();
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod zoom_tests {
    use super::{auto_zoom, auto_zoom_hysteresis, short_page};

    #[test]
    fn parent_pages_are_shortened() {
        assert_eq!(
            short_page("file:///tmp/site/blog/a.html", "file:///x"),
            "blog/a.html"
        );
        assert_eq!(short_page("https://a.ro/x/y", "https://a.ro/z"), "/x/y");
        assert_eq!(short_page("https://a.ro", "https://a.ro/z"), "/");
        assert_eq!(
            short_page("https://a.ro/x", "https://b.ro/z"),
            "https://a.ro/x"
        );
    }

    #[test]
    fn auto_zoom_fills_the_screen_but_stays_sane() {
        assert_eq!(auto_zoom(1920.0, 1080.0), 1.6); // Full HD: text mărit
        assert_eq!(auto_zoom(1180.0, 760.0), 1.05); // fereastra implicită
        assert_eq!(auto_zoom(800.0, 500.0), 1.0); // ecran mic: nu micșorăm sub 100%
        assert_eq!(auto_zoom(7680.0, 4320.0), 2.2); // 8K: plafon
        assert_eq!(auto_zoom(0.0, 0.0), 1.0);
    }

    #[test]
    fn hysteresis_prevents_flicker_near_a_step_boundary() {
        let mut z = 1.0;
        for w in [1165.0, 1180.0, 1172.0, 1176.0, 1168.0] {
            z = auto_zoom_hysteresis(z, w, 760.0);
            assert_eq!(z, 1.0, "diferență sub 0,1 → rămâne");
        }
        assert_eq!(auto_zoom_hysteresis(1.0, 1920.0, 1080.0), 1.6); // fereastră mărită: trece
        assert_eq!(auto_zoom_hysteresis(1.6, 1180.0, 760.0), 1.05); // și înapoi
    }

    /// Simulează egui: `inner_rect` e raportat în puncte la factorul de cu 2 cadre în urmă, dar convertit cu factorul
    /// curent (greșit, ca pe Windows) ar cicla. Cu perioada de așteptare bucla converge și rămâne stabilă.
    fn simulate(nw: f32, nh: f32, wait_frames: u8) -> (Vec<f32>, f32) {
        let mut zoom = 1.0_f32;
        let mut wait = 0u8;
        let mut hist = vec![zoom, zoom];
        let mut changes = Vec::new();
        for _ in 0..80 {
            let stale = hist[hist.len() - 2];
            let (rw, rh) = (nw / stale * zoom, nh / stale * zoom);
            let want = if wait > 0 {
                wait -= 1;
                zoom
            } else {
                auto_zoom_hysteresis(zoom, rw, rh)
            };
            if (want - zoom).abs() > 0.001 {
                changes.push(want);
                wait = wait_frames;
            }
            zoom = want;
            hist.push(zoom);
        }
        (changes, zoom)
    }

    #[test]
    fn lagging_window_size_converges_without_oscillation() {
        // fără așteptare, modelul reproduce ciclarea raportată pe Windows 11
        let (bad, _) = simulate(1920.0, 1080.0, 0);
        assert!(
            bad.len() > 5,
            "modelul trebuie să reproducă defectul: {bad:?}"
        );
        // cu așteptarea din aplicație: o singură schimbare, stabil
        for (w, h, expect) in [
            (1920.0, 1080.0, 1.6),
            (2560.0, 1440.0, 2.1),
            (1180.0, 760.0, 1.0),
        ] {
            let (ch, z) = simulate(w, h, 4);
            assert!(ch.len() <= 1, "{w}x{h}: prea multe schimbări {ch:?}");
            assert_eq!(z, expect, "{w}x{h}");
        }
    }
}
