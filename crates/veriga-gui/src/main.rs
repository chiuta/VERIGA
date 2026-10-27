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
    filter: Filter,
    search: String,
    sort: SortBy,
    sort_desc: bool,
    selected: Option<usize>,
    paused: bool,
    english: bool,
    gd: guardian_ui::GuardianUi,
}

impl App {
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
            filter: Filter::All,
            search: String::new(),
            sort: SortBy::Order,
            sort_desc: false,
            selected: None,
            paused: false,
            english: false,
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
            Err(e) => self.status_line = e,
            Ok(cfg) => {
                self.results.clear();
                self.session = None;
                self.selected = None;
                self.progress = Progress::default();
                self.status_line.clear();
                self.spawn_job(move |ctl, tx| Box::pin(veriga_core::run(cfg, ctl, Some(tx))));
            }
        }
    }

    fn start_recheck(&mut self) {
        let Some(s) = self.session.clone() else {
            return;
        };
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
                    self.session = Some(s);
                }
                Err(e) => self.status_line = format!("{}: {e}", self.t("Eroare", "Error")),
            }
        }
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
        let (name, ext, data): (&str, &str, Vec<u8>) = match kind {
            "html" => ("Raport HTML", "html", report::to_html(s).into_bytes()),
            "csv" => ("CSV", "csv", report::to_csv(s, b',', false).into_bytes()),
            "csvb" => (
                "CSV — rupte",
                "csv",
                report::to_csv(s, b',', true).into_bytes(),
            ),
            "tsv" => ("TSV", "tsv", report::to_csv(s, b'\t', false).into_bytes()),
            "json" => ("JSON", "json", report::to_json(s).into_bytes()),
            "txt" => ("Text", "txt", report::to_text(s).into_bytes()),
            "xml" => ("Sitemap XML", "xml", report::to_sitemap_xml(s).into_bytes()),
            "tree" => ("Arbore", "txt", report::to_tree_text(s, 12).into_bytes()),
            _ => return,
        };
        if let Some(p) = rfd::FileDialog::new()
            .set_title(name)
            .set_file_name(format!("veriga-raport.{ext}"))
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

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
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
        if self.running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let lang = self.lang();
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
                    if ui.button(self.t("▶ Pornește", "▶ Start")).clicked() {
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
                    match Session::load(&p) {
                        Ok(s) => {
                            self.results = s.results.clone();
                            self.start = s.config.start.join("\n");
                            self.cfg = s.config.clone();
                            self.session = Some(s);
                            self.status_line =
                                format!("{} {}", self.t("Încărcat:", "Loaded:"), p.display());
                        }
                        Err(e) => self.status_line = format!("{}: {e}", self.t("Eroare", "Error")),
                    }
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
                        ("html", "Raport HTML"),
                        ("csv", "CSV"),
                        ("csvb", "CSV (doar rupte)"),
                        ("tsv", "TSV"),
                        ("json", "JSON"),
                        ("txt", "Text"),
                        ("xml", "Sitemap XML"),
                        ("tree", "Arbore site"),
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
            ui.add(
                egui::TextEdit::multiline(&mut self.start)
                    .desired_rows(1)
                    .desired_width(f32::INFINITY)
                    .hint_text("https://exemplu.ro/   sau   /cale/catre/site"),
            );
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
                        if self.paused { "(pauză)" } else { "" }
                    ));
                    if !self.phase.is_empty() {
                        ui.label(RichText::new(&self.phase).weak());
                    }
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

        egui::CentralPanel::default().show(root, |ui| {
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
                let l_skip = self.t("Omise", "Skipped");
                let l_search = self.t("Caută…", "Search…");
                ui.selectable_value(&mut self.filter, Filter::All, l_all);
                ui.selectable_value(&mut self.filter, Filter::Broken, l_broken);
                ui.selectable_value(&mut self.filter, Filter::Redirects, l_red);
                ui.selectable_value(&mut self.filter, Filter::Ok, "OK");
                ui.selectable_value(&mut self.filter, Filter::Orphans, l_orph);
                ui.selectable_value(&mut self.filter, Filter::Skipped, l_skip);
                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
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
                .column(Column::remainder().at_least(250.0).clip(true))
                .column(Column::initial(90.0))
                .column(Column::initial(70.0))
                .column(Column::initial(200.0).clip(true))
                .column(Column::initial(45.0))
                .column(Column::initial(60.0))
                .header(22.0, |mut h| {
                    let heads: [(&str, &str, SortBy); 8] = [
                        ("Stare", "Status", SortBy::Status),
                        ("Cod", "Code", SortBy::Order),
                        ("URL", "URL", SortBy::Url),
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

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
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
            Ok(Box::new(App::new()))
        }),
    )
}
