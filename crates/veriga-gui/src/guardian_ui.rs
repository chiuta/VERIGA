//! Fereastra „Guardian”: reparații cu propuneri verificate, jurnal + anulare,
//! domenii, hartă de sănătate, redirecționări, tendințe, 404 din jurnale,
//! conformitate și configurare programată.

use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use veriga_core::fixer::{self, ApplyOpts, FixLog, FixStatus, Proposal, ProposeOpts};
use veriga_core::insights::{self, Period};
use veriga_core::{Session, srcscan};

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Fix,
    Log,
    Domains,
    Health,
    Redirects,
    Trends,
    Log404,
    Compliance,
}

pub struct GuardianUi {
    pub open: bool,
    tab: Tab,
    sources: String,
    state: String,
    verify: bool,
    props: Vec<Proposal>,
    sel: Vec<bool>,
    rx: Option<Receiver<Vec<Proposal>>>,
    msg: String,
    manual_old: String,
    manual_new: String,
    log: FixLog,
    log_loaded: bool,
    logfile: String,
    hits: Vec<insights::Hit404>,
    days: u32,
    prev: Option<Session>,
}

impl Default for GuardianUi {
    fn default() -> Self {
        Self {
            open: false,
            tab: Tab::Fix,
            sources: String::new(),
            state: ".veriga".into(),
            verify: true,
            props: vec![],
            sel: vec![],
            rx: None,
            msg: String::new(),
            manual_old: String::new(),
            manual_new: String::new(),
            log: FixLog::default(),
            log_loaded: false,
            logfile: String::new(),
            hits: vec![],
            days: 30,
            prev: None,
        }
    }
}

impl GuardianUi {
    fn paths(&self) -> (PathBuf, PathBuf, PathBuf) {
        insights::default_paths(&PathBuf::from(self.state.trim()))
    }

    /// Operează pe jurnalul de pe disc (nu pe copia din memorie): blocare, citire verificată,
    /// modificare, salvare atomică. Astfel GUI-ul nu suprascrie modificările făcute între timp de CLI.
    fn with_log<R>(&mut self, f: impl FnOnce(&mut FixLog) -> R) -> Result<R, String> {
        let logp = self.paths().1;
        let _lock = FixLog::lock(&logp).map_err(|e| e.to_string())?;
        let mut log = FixLog::load_checked(&logp).map_err(|e| e.to_string())?;
        let r = f(&mut log);
        log.save(&logp).map_err(|e| e.to_string())?;
        self.log = log;
        Ok(r)
    }

    fn scan(&mut self, session: Option<Session>, lang: &str) {
        let paths: Vec<PathBuf> = self
            .sources
            .lines()
            .map(|l| PathBuf::from(l.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        if paths.is_empty() {
            self.msg = if lang == "en" {
                "Add source files or folders.".into()
            } else {
                "Adaugă fișiere sau directoare-sursă.".into()
            };
            return;
        }
        let verify = self.verify;
        let lang = lang.to_string();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.msg = if lang == "en" {
            "Scanning and verifying live…".into()
        } else {
            "Scanez și verific live…".into()
        };
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let ps = rt.block_on(async move {
                let occs = srcscan::scan_paths(&paths).unwrap_or_default();
                let mut ps = fixer::propose(
                    &occs,
                    session.as_ref(),
                    &ProposeOpts {
                        verify,
                        lang,
                        ..Default::default()
                    },
                )
                .await;
                fixer::fill_old_raw(&mut ps);
                ps
            });
            let _ = tx.send(ps);
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, session: &Option<Session>, english: bool) {
        if !self.open {
            return;
        }
        let lang = if english { "en" } else { "ro" };
        let t = |ro: &'static str, en: &'static str| if english { en } else { ro };
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(ps) => {
                    self.sel = ps.iter().map(|p| p.verified).collect();
                    self.msg = format!("{} {}", ps.len(), t("propuneri", "proposals"));
                    self.props = ps;
                    self.rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // firul de analiză s-a oprit (panică / eroare): nu rămânem blocați pe „Scanez…”
                    self.msg = t(
                        "Analiza a eșuat (firul s-a oprit). Reîncearcă.",
                        "Analysis failed (worker stopped). Try again.",
                    )
                    .to_string();
                    self.rx = None;
                }
            }
        }
        if !self.log_loaded {
            match FixLog::load_checked(&self.paths().1) {
                Ok(l) => self.log = l,
                Err(e) => {
                    self.log = FixLog::default();
                    self.msg = e.to_string();
                }
            }
            self.log_loaded = true;
        }
        let mut open = self.open;
        egui::Window::new("VERIGA Guardian").open(&mut open).default_size([920.0, 560.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tab, ro, en) in [
                    (Tab::Fix, "Reparații", "Fixes"),
                    (Tab::Log, "Jurnal / anulare", "Log / undo"),
                    (Tab::Domains, "Domenii", "Domains"),
                    (Tab::Health, "Hartă / carte", "Health / book"),
                    (Tab::Redirects, "Redirecționări", "Redirects"),
                    (Tab::Trends, "Tendințe", "Trends"),
                    (Tab::Log404, "404 din jurnal", "404 from logs"),
                    (Tab::Compliance, "Conformitate", "Compliance"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, t(ro, en));
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(t("Director de stare:", "State folder:"));
                if ui.text_edit_singleline(&mut self.state).changed() {
                    self.log_loaded = false;
                }
            });
            match self.tab {
                Tab::Fix => {
                    ui.label(t("Surse (un fișier/director pe linie; HTML, Markdown, export WordPress .xml, sitemap):", "Sources (one file/folder per line; HTML, Markdown, WordPress export .xml, sitemap):"));
                    ui.add(egui::TextEdit::multiline(&mut self.sources).desired_rows(3).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        if ui.button(t("📂 Adaugă director…", "📂 Add folder…")).clicked()
                            && let Some(d) = rfd::FileDialog::new().pick_folder() {
                                self.sources.push_str(&format!("{}\n", d.display()));
                            }
                        if ui.button(t("📄 Adaugă fișier…", "📄 Add file…")).clicked()
                            && let Some(d) = rfd::FileDialog::new().pick_file() {
                                self.sources.push_str(&format!("{}\n", d.display()));
                            }
                        ui.checkbox(&mut self.verify, t("Verifică live URL-ul nou", "Verify new URL live"));
                        if ui.add_enabled(self.rx.is_none(), egui::Button::new(t("🔍 Analizează", "🔍 Analyze"))).clicked() {
                            self.scan(session.clone(), lang);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(t("Manual: înlocuiește", "Manual: replace"));
                        ui.add(egui::TextEdit::singleline(&mut self.manual_old).hint_text("URL").desired_width(220.0));
                        ui.label("→");
                        ui.add(egui::TextEdit::singleline(&mut self.manual_new).hint_text(t("URL nou (gol = dezleagă)", "new URL (empty = unlink)")).desired_width(220.0));
                        if ui.button(t("Adaugă propunere", "Add proposal")).clicked() && !self.manual_old.trim().is_empty() {
                            let paths: Vec<PathBuf> = self.sources.lines().map(|l| PathBuf::from(l.trim())).filter(|p| !p.as_os_str().is_empty()).collect();
                            let occs = srcscan::scan_paths(&paths).unwrap_or_default();
                            let old = self.manual_old.trim().to_string();
                            let mut add = if self.manual_new.trim().is_empty() { fixer::unlink(&occs, &old) } else { match fixer::manual_replace(&occs, &old, self.manual_new.trim()) { Ok(v) => v, Err(e) => { self.msg = e; vec![] } } };
                            fixer::fill_old_raw(&mut add);
                            for p in add {
                                self.props.push(p);
                                self.sel.push(true);
                            }
                        }
                    });
                    ui.label(&self.msg);
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        egui::Grid::new("props").striped(true).show(ui, |ui| {
                            for (i, p) in self.props.iter().enumerate() {
                                ui.checkbox(&mut self.sel[i], "");
                                ui.label(if p.verified { "✔" } else { "?" });
                                ui.label(p.rule.label(lang));
                                if p.impact() {
                                    ui.label(RichText::new("★").color(egui::Color32::from_rgb(220, 150, 20))).on_hover_text(t("Impact: în meniu sau pe ≥ 3 pagini", "Impact: in a menu or on ≥ 3 pages"));
                                } else {
                                    ui.label("");
                                }
                                ui.label(RichText::new(&p.old).monospace());
                                ui.label(RichText::new(format!("→ {}", p.new)).monospace());
                                ui.end_row();
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        if ui.button(t("Selectează verificate", "Select verified")).clicked() {
                            for (i, p) in self.props.iter().enumerate() {
                                self.sel[i] = p.verified;
                            }
                        }
                        if ui.button(t("CSV propuneri…", "Proposals CSV…")).clicked()
                            && let Some(f) = rfd::FileDialog::new().set_file_name("propuneri.csv").save_file() {
                                let _ = std::fs::write(f, fixer::proposals_csv(&self.props, lang));
                            }
                        let n = self.sel.iter().filter(|b| **b).count();
                        if ui.add_enabled(n > 0, egui::Button::new(format!("{} ({n})", t("✓ Aplică selecția (cu copie de siguranță)", "✓ Apply selection (with backup)")))).clicked() {
                            let chosen: Vec<Proposal> = self.props.iter().zip(&self.sel).filter(|(_, s)| **s).map(|(p, _)| p.clone()).collect();
                            let (_, _, bk) = self.paths();
                            match self.with_log(|log| fixer::apply(&chosen, log, &ApplyOpts { backup_dir: bk, run: fixer::run_id(), allow_unverified: false })) {
                                Ok(rep) => {
                                    self.msg = format!("{}: {} · {}: {}", t("Aplicate", "Applied"), rep.applied.len(), t("omise", "skipped"), rep.skipped.len());
                                    self.props.clear();
                                    self.sel.clear();
                                }
                                Err(e) => self.msg = e,
                            }
                        }
                    });
                }
                Tab::Log => {
                    ui.horizontal(|ui| {
                        if ui.button(t("↶ Anulează ultima rulare", "↶ Undo last run")).clicked()
                            && let Some(run) = self.log.entries.iter().rev().find(|e| e.status == FixStatus::Applied).map(|e| e.run.clone()) {
                                match self.with_log(|log| fixer::undo(log, None, Some(&run))) {
                                    Ok(rep) => self.msg = format!("{}: {} · {}: {}", t("Anulate", "Undone"), rep.undone.len(), t("conflicte", "conflicts"), rep.conflicts.len()),
                                    Err(e) => self.msg = e,
                                }
                            }
                        if ui.button(t("CSV jurnal…", "Log CSV…")).clicked()
                            && let Some(f) = rfd::FileDialog::new().set_file_name("jurnal-reparatii.csv").save_file() {
                                let _ = std::fs::write(f, fixer::log_csv(&self.log));
                            }
                        ui.label(&self.msg);
                    });
                    let mut undo_id: Option<String> = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("log").striped(true).show(ui, |ui| {
                            for e in self.log.entries.iter().rev() {
                                ui.label(&e.run);
                                ui.label(e.rule.label(lang));
                                ui.label(format!("{:?}", e.status));
                                ui.label(RichText::new(format!("{} → {}", e.old, e.new)).monospace());
                                if e.status == FixStatus::Applied && ui.small_button("↶").clicked() {
                                    undo_id = Some(e.id.clone());
                                }
                                ui.end_row();
                            }
                        });
                    });
                    if let Some(id) = undo_id {
                        match self.with_log(|log| fixer::undo(log, Some(&[id]), None)) {
                            Ok(rep) => self.msg = format!("{}: {} · {}: {}", t("Anulate", "Undone"), rep.undone.len(), t("conflicte", "conflicts"), rep.conflicts.len()),
                            Err(e) => self.msg = e,
                        }
                    }
                }
                Tab::Domains => match session {
                    Some(s) => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("dom").striped(true).show(ui, |ui| {
                                for h in [t("Gazdă", "Host"), t("Total", "Total"), t("Rupte", "Broken"), "403", t("Redir.", "Redir."), "ms", t("Sănătate", "Health")] {
                                    ui.label(RichText::new(h).strong());
                                }
                                ui.end_row();
                                for d in insights::domain_stats(s) {
                                    ui.label(&d.host);
                                    ui.label(d.total.to_string());
                                    ui.label(d.broken.to_string());
                                    ui.label(d.review.to_string());
                                    ui.label(d.redirects.to_string());
                                    ui.label(d.avg_ms.to_string());
                                    ui.add(egui::ProgressBar::new((d.health / 100.0) as f32).text(format!("{:.0}%", d.health)).desired_width(120.0));
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    None => {
                        ui.label(t("Rulează întâi o verificare.", "Run a check first."));
                    }
                },
                Tab::Health => match session {
                    Some(s) => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for p in insights::health_map(s) {
                                let head = format!("{:.0}% · {} / {} — {}", p.score, p.broken, p.links, p.title.as_deref().unwrap_or(&p.page));
                                if p.items.is_empty() {
                                    ui.label(head);
                                } else {
                                    egui::CollapsingHeader::new(head).id_salt(&p.page).show(ui, |ui| {
                                        for i in &p.items {
                                            ui.label(RichText::new(format!("{} [{}] {}", if i.review { "⚠" } else { "✖" }, i.status.label(lang), i.url)).monospace());
                                        }
                                    });
                                }
                            }
                        });
                    }
                    None => {
                        ui.label(t("Rulează întâi o verificare.", "Run a check first."));
                    }
                },
                Tab::Redirects => match session {
                    Some(s) => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (a, b, c, perm, n) in insights::redirects(s) {
                                ui.label(RichText::new(format!("{a}\n  → {b}   {} {} · {n} {}", c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("→"), if perm { t("permanent", "permanent") } else { t("temporar", "temporary") }, t("pagini", "pages"))).monospace());
                            }
                        });
                    }
                    None => {
                        ui.label(t("Rulează întâi o verificare.", "Run a check first."));
                    }
                },
                Tab::Trends => {
                    let h = insights::history_load(&self.paths().0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(session.is_some(), egui::Button::new(t("＋ Înregistrează rularea curentă", "＋ Record current run"))).clicked()
                            && let Some(s) = session {
                                let _ = insights::history_append(&self.paths().0, s);
                            }
                        if ui.button(t("Încarcă sesiune anterioară…", "Load previous session…")).clicked()
                            && let Some(f) = rfd::FileDialog::new().add_filter("VERIGA", &["veriga"]).pick_file() {
                                self.prev = Session::load(&f).ok();
                            }
                        if ui.add_enabled(session.is_some(), egui::Button::new(t("Raport Guardian (HTML)…", "Guardian report (HTML)…"))).clicked()
                            && let (Some(s), Some(f)) = (session, rfd::FileDialog::new().set_file_name("guardian.html").save_file()) {
                                let _ = std::fs::write(f, insights::guardian_html(s, self.prev.as_ref(), &h));
                            }
                    });
                    if let (Some(p), Some(s)) = (&self.prev, session) {
                        let d = insights::diff(p, s);
                        ui.label(format!("{}: {} · {}: {} · {}: {}", t("noi rupte", "newly broken"), d.new_broken.len(), t("rezolvate", "resolved"), d.resolved.len(), t("derivă conținut", "content drift"), d.drift.len()));
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for x in h.iter().rev() {
                            ui.label(RichText::new(format!("{}  total {:>6}  rupte {:>5}  {:.1}%", x.time, x.total, x.broken, x.health)).monospace());
                        }
                    });
                }
                Tab::Log404 => {
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.logfile).hint_text(t("jurnal de acces Apache/nginx", "Apache/nginx access log")));
                        if ui.button("…").clicked()
                            && let Some(f) = rfd::FileDialog::new().pick_file() {
                                self.logfile = f.display().to_string();
                            }
                        if ui.button(t("Importă 404", "Import 404s")).clicked() {
                            match std::fs::read_to_string(&self.logfile) {
                                Ok(txt) => {
                                    let mut known: Vec<String> = vec![];
                                    if let Some(s) = session {
                                        for r in &s.results {
                                            if !r.status.is_broken(&s.config)
                                                && let Ok(u) = url::Url::parse(&r.url) {
                                                    known.push(u.path().to_string());
                                                }
                                        }
                                        known.sort();
                                        known.dedup();
                                    }
                                    self.hits = insights::parse_404(&txt, true, &known);
                                    self.msg = format!("{} 404", self.hits.len());
                                }
                                Err(e) => self.msg = e.to_string(),
                            }
                        }
                        if ui.add_enabled(!self.hits.is_empty(), egui::Button::new(".htaccess…")).clicked()
                            && let Some(f) = rfd::FileDialog::new().set_file_name("redirect.htaccess").save_file() {
                                let _ = std::fs::write(f, insights::htaccess_rules(&self.hits));
                            }
                        if ui.add_enabled(!self.hits.is_empty(), egui::Button::new("CSV…")).clicked()
                            && let Some(f) = rfd::FileDialog::new().set_file_name("404.csv").save_file() {
                                let _ = std::fs::write(f, insights::hits_csv(&self.hits));
                            }
                    });
                    ui.label(&self.msg);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for h in &self.hits {
                            ui.label(RichText::new(format!("{:>5}×  {}{}", h.count, h.path, h.suggestion.as_ref().map(|s| format!("   → {s}")).unwrap_or_default())).monospace());
                        }
                    });
                }
                Tab::Compliance => {
                    ui.horizontal(|ui| {
                        ui.label(t("Perioadă (zile):", "Period (days):"));
                        ui.add(egui::DragValue::new(&mut self.days).range(1..=3650));
                        let go = |all: bool, me: &Self| {
                            if let (Some(s), Some(f)) = (session, rfd::FileDialog::new().set_file_name("conformitate.html").save_file()) {
                                let h = insights::history_load(&me.paths().0);
                                let p = if all { Period::All } else { Period::Days(me.days) };
                                let _ = std::fs::write(f, insights::compliance_html(s, &me.log, &h, p));
                            }
                        };
                        if ui.add_enabled(session.is_some(), egui::Button::new(t("Raport…", "Report…"))).clicked() {
                            go(false, self);
                        }
                        if ui.add_enabled(session.is_some(), egui::Button::new(t("Tot istoricul…", "All time…"))).clicked() {
                            go(true, self);
                        }
                    });
                    ui.label(t("Pentru rulări programate folosește CLI: `veriga wizard` și `veriga watch`.", "For scheduled runs use the CLI: `veriga wizard` and `veriga watch`."));
                }
            }
        });
        self.open = open;
    }
}
