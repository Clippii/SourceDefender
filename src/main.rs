#![cfg_attr(windows, windows_subsystem = "windows")]
mod backend;
use backend::{Action, Run, Snapshot};
use eframe::egui::{self, Color32, RichText, Vec2};
use serde_json::{Value, json};
use std::{sync::mpsc, time::Duration};

const ACCENT: Color32 = Color32::from_rgb(129, 166, 239);
const MUTED: Color32 = Color32::from_rgb(150, 158, 173);
const SURFACE: Color32 = Color32::from_rgb(28, 31, 37);
const BG: Color32 = Color32::from_rgb(20, 22, 27);
const BORDER: Color32 = Color32::from_rgb(47, 51, 60);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SourceDefender")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([1000.0, 700.0])
            .with_icon(std::sync::Arc::new(app_icon())),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "SourceDefender",
        options,
        Box::new(|cc| Ok(Box::new(Defender::new(&cc.egui_ctx)))),
    )
}

fn app_icon() -> egui::IconData {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let stroke = (8..24).contains(&x)
                && ((7..11).contains(&y) || (14..18).contains(&y) || (21..25).contains(&y))
                || (8..12).contains(&x) && (10..16).contains(&y)
                || (20..24).contains(&x) && (17..23).contains(&y);
            rgba.extend(if stroke {
                [129, 166, 239, 255]
            } else {
                [20, 22, 27, 255]
            });
        }
    }
    egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}

struct Defender {
    snapshot: Snapshot,
    current: Option<Run>,
    setup: bool,
    history: bool,
    confirm_end: bool,
    scenario: usize,
    language: usize,
    level: usize,
    seed: String,
    folder: String,
    report: String,
    error: Option<String>,
    busy: bool,
    receiver: mpsc::Receiver<(String, Result<Value, String>)>,
    sender: mpsc::Sender<(String, Result<Value, String>)>,
}
impl Defender {
    fn new(ctx: &egui::Context) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE;
        visuals.extreme_bg_color = Color32::from_rgb(16, 18, 22);
        visuals.override_text_color = Some(Color32::from_rgb(231, 234, 240));
        visuals.selection.bg_fill = Color32::from_rgb(43, 63, 99);
        visuals.widgets.active.bg_fill = Color32::from_rgb(52, 71, 108);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 42, 50);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.button_padding = Vec2::new(13.0, 9.0);
            s.text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            s.text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        });
        let (sender, receiver) = mpsc::channel();
        let mut app = Self {
            snapshot: Snapshot::default(),
            current: None,
            setup: false,
            history: false,
            confirm_end: false,
            scenario: 0,
            language: 0,
            level: 0,
            seed: String::new(),
            folder: backend::default_folder().to_string_lossy().into(),
            report: "Loading local incident records…".into(),
            error: None,
            busy: false,
            receiver,
            sender,
        };
        app.request(json!({"command":"snapshot"}));
        app
    }
    fn request(&mut self, request: Value) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let command = request["command"].as_str().unwrap_or("").to_owned();
            let result = backend::call(request);
            let _ = sender.send((command, result));
        });
    }
    fn receive(&mut self) {
        while let Ok((command, result)) = self.receiver.try_recv() {
            self.busy = false;
            match result {
                Err(e) => self.error = Some(e),
                Ok(data) => {
                    if command == "snapshot" {
                        match serde_json::from_value::<Snapshot>(data) {
                            Err(e) => {
                                self.error = Some(format!("Could not read local records: {e}"))
                            }
                            Ok(snapshot) => {
                                self.current =
                                    snapshot.runs.iter().find(|r| r.status == "Active").cloned();
                                self.snapshot = snapshot;
                                self.setup = self.current.is_none();
                                self.report = if self.current.is_some() {
                                    "Active incident restored. The clock has continued while the app was closed.".into()
                                } else {
                                    "A trusted account has been compromised. Investigate the repository, remove attacker changes, and contain the incident.\n\nYour editor. Real files. Local Git evidence.\n\nChoose an incident to begin.".into()
                                };
                            }
                        }
                    } else {
                        match serde_json::from_value::<Action>(data) {
                            Err(e) => {
                                self.error = Some(format!("Could not read engine result: {e}"))
                            }
                            Ok(action) => {
                                self.report = action.report;
                                self.snapshot.runs.retain(|r| r.id != action.run.id);
                                self.snapshot.runs.insert(0, action.run.clone());
                                self.current = Some(action.run);
                                if command == "generate" {
                                    self.setup = false;
                                    self.history = false;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn active(&self) -> bool {
        self.current.as_ref().is_some_and(|r| r.status == "Active")
    }
    fn action(&mut self, command: &str) {
        if let Some(r) = &self.current {
            self.request(json!({"command":command,"id":r.id}));
        }
    }
    fn heading(ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).size(12.0).color(MUTED).strong());
    }
    fn summary(&self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(SURFACE)
            .stroke(egui::Stroke::new(1.0_f32, BORDER))
            .corner_radius(8.0)
            .inner_margin(20.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.columns(3, |cols| {
                    if let Some(r) = &self.current {
                        Self::heading(&mut cols[0], &r.status);
                        cols[0].label(RichText::new(&r.scenario).size(19.0).strong());
                        cols[0].label(
                            RichText::new(format!(
                                "{} / {}",
                                r.language, self.snapshot.difficulties[r.level].name
                            ))
                            .color(MUTED),
                        );
                        Self::heading(&mut cols[1], "Elapsed time");
                        cols[1].label(
                            RichText::new(backend::time(r.seconds()))
                                .monospace()
                                .size(31.0),
                        );
                        Self::heading(&mut cols[2], "Run statistics");
                        cols[2].label(format!(
                            "{} scans   ·   {} intel requests",
                            r.attempts, r.hints
                        ));
                        cols[2].label(
                            RichText::new(format!("+{}s penalties", r.penalty_seconds))
                                .color(MUTED),
                        );
                        if r.status == "Contained" {
                            cols[2]
                                .label(RichText::new(format!("Score: {}", r.score)).color(ACCENT));
                        }
                    } else {
                        Self::heading(&mut cols[0], "No active incident");
                        cols[0].label("Choose a repository to investigate.");
                        cols[1].label(RichText::new("00:00").monospace().size(31.0));
                        Self::heading(&mut cols[2], "Training library");
                        cols[2].label("16 attack types · 4 languages");
                    }
                });
            });
    }
    fn setup_dialog(&mut self, ctx: &egui::Context) {
        if !self.setup || self.snapshot.scenarios.is_empty() {
            return;
        }
        let height = (ctx.content_rect().height() - 100.0).max(320.0);
        let id = egui::Id::new("setup");
        egui::Modal::new(id)
            .area(egui::Modal::default_area(id).default_size([610.0,height+40.0]))
            .frame(egui::Frame::new().fill(SURFACE).stroke(egui::Stroke::new(1.0_f32,BORDER)).corner_radius(8.0).inner_margin(24.0))
            .show(ctx,|ui| {
            ui.set_width(570.0);
            egui::ScrollArea::vertical().max_height(height).auto_shrink([false,false]).show(ui,|ui| {
                ui.heading("New incident");
                ui.label(RichText::new("Investigate an attacked codebase. Restore trusted behavior.").color(MUTED));ui.separator();
                ui.add_enabled_ui(!self.busy,|ui| {
                    Self::heading(ui,"Scenario");
                    egui::ComboBox::from_id_salt("scenario").width(535.0).selected_text(&self.snapshot.scenarios[self.scenario].name).show_ui(ui,|ui| {
                        for (i,s) in self.snapshot.scenarios.iter().enumerate() {ui.selectable_value(&mut self.scenario,i,&s.name);}
                    });
                    ui.label(RichText::new(&self.snapshot.scenarios[self.scenario].description).color(MUTED));
                    Self::heading(ui,"Source language");
                    ui.horizontal(|ui| {for (i,s) in self.snapshot.languages.iter().enumerate() {ui.selectable_value(&mut self.language,i,s);}});
                    Self::heading(ui,"Difficulty");
                    egui::ComboBox::from_id_salt("difficulty").width(535.0).selected_text(&self.snapshot.difficulties[self.level].name).show_ui(ui,|ui| {
                        for (i,d) in self.snapshot.difficulties.iter().enumerate() {ui.selectable_value(&mut self.level,i,&d.name);}
                    });
                    let d=&self.snapshot.difficulties[self.level];
                    ui.label(format!("{} attacks · {} encoding layers · +{}s per failed scan",d.bugs,self.level,d.penalty));
                    ui.label(RichText::new(["Finding titles and file paths. Plain source.","Finding titles. Encoded source values.","Category counts. Misleading adapters and maintenance commits.","Remaining count only. Three encoding layers and scattered commits."][self.level]).color(MUTED));
                    Self::heading(ui,"Seed (optional)");ui.add(egui::TextEdit::singleline(&mut self.seed).desired_width(f32::INFINITY).hint_text("Leave blank for a fresh incident").char_limit(80));
                    Self::heading(ui,"Repository parent folder");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.folder).desired_width(420.0));
                        if ui.button("Browse…").clicked() {match backend::choose_folder() {Ok(Some(path))=>self.folder=path,Ok(None)=>{},Err(e)=>self.error=Some(e)}}
                    });
                    ui.label(RichText::new("A new incident subfolder is created here. The timer starts when you click Start.").small().color(MUTED));
                    if self.active() {ui.colored_label(Color32::LIGHT_YELLOW,"End the active run before starting another incident.");}
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.active()&&!self.folder.trim().is_empty(),egui::Button::new(RichText::new("Start incident").color(Color32::BLACK)).fill(ACCENT)).clicked() {
                            self.request(json!({"command":"generate","folder":self.folder.trim(),"level":self.level,"scenario":self.scenario,"language":self.snapshot.languages[self.language],"seed":self.seed.trim()}));
                        }
                        if ui.button("Cancel").clicked() {self.setup=false;}
                    });
                });
                if self.busy {ui.horizontal(|ui| {ui.spinner();ui.label("Preparing repository and Git evidence…");});}
                if let Some(error)=&self.error {ui.colored_label(Color32::LIGHT_RED,error);}
            });
        });
    }
    fn history_panel(&self, ui: &mut egui::Ui) {
        Self::heading(ui, "Your local incident record");
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("history")
                .striped(true)
                .spacing([20.0, 16.0])
                .show(ui, |ui| {
                    for h in [
                        "Incident",
                        "Language",
                        "Difficulty",
                        "Result",
                        "Time",
                        "Score",
                    ] {
                        Self::heading(ui, h);
                    }
                    ui.end_row();
                    for r in &self.snapshot.runs {
                        ui.label(&r.scenario);
                        ui.label(&r.language);
                        ui.label(&self.snapshot.difficulties[r.level].name);
                        ui.label(&r.status);
                        ui.label(backend::time(r.seconds()));
                        ui.label(if r.status == "Contained" {
                            r.score.to_string()
                        } else {
                            "—".into()
                        });
                        ui.end_row();
                    }
                });
            if self.snapshot.runs.is_empty() {
                ui.label("Your completed operations will appear here.");
            }
        });
    }
}
impl eframe::App for Defender {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive();
        ctx.request_repaint_after(Duration::from_millis(100));
        egui::SidePanel::left("navigation")
            .exact_width(195.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(16, 18, 22))
                    .inner_margin(20.0),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("SourceDefender").size(19.0).strong());
                ui.label(
                    RichText::new("Incident response training")
                        .size(11.0)
                        .color(MUTED),
                );
                ui.add_space(26.0);
                if ui
                    .add_sized(
                        [155.0, 38.0],
                        egui::Button::new("Investigation").selected(!self.history),
                    )
                    .clicked()
                {
                    self.history = false;
                }
                if ui
                    .add_sized(
                        [155.0, 38.0],
                        egui::Button::new("Run history").selected(self.history),
                    )
                    .clicked()
                {
                    self.history = true;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(RichText::new("Version 0.3.1").size(11.0).color(MUTED));
                    ui.label(
                        RichText::new("Local files · Your editor")
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.separator();
                });
            });
        egui::TopBottomPanel::bottom("footer").frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(28,16))).show(ctx,|ui| {
            ui.label(RichText::new("Review in your editor. Save your files. Return here to validate. The timer continues while closed.").small().color(MUTED));
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(28.0)).show(ctx,|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(if self.history {"Run history"}else{"Investigation"}).size(24.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.add_enabled(!self.busy,egui::Button::new("New incident")).clicked(){self.setup=true;}
                });
            });ui.add_space(16.0);
            if !self.history {
            self.summary(ui);ui.add_space(8.0);
            ui.add_enabled_ui(!self.busy&&!self.setup,|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(self.active(),egui::Button::new(RichText::new("Validate changes").color(Color32::from_rgb(12,22,42))).fill(ACCENT)).clicked() {self.action("validate");}
                    if ui.add_enabled(self.current.is_some(),egui::Button::new("Open folder")).clicked() {
                        if let Some(r)=&self.current {
                            let folder=std::path::Path::new(&r.folder);
                            if let Err(e)=backend::open_folder(folder){self.error=Some(e);}
                        }
                    }
                    let hint_label=self.current.as_ref().map(|r|format!("Request intel (+{}s)",self.snapshot.difficulties[r.level].hint_cost)).unwrap_or("Request intel".into());
                    if ui.add_enabled(self.active(),egui::Button::new(hint_label)).clicked() {self.action("hint");}
                    if ui.add_enabled(self.active(),egui::Button::new("End run")).clicked() {self.confirm_end=true;}
                });
            });
            }
            if self.busy&&!self.setup {ui.horizontal(|ui| {ui.spinner();ui.label("Working on the local repository…");});}
            if let Some(error)=&self.error {ui.colored_label(Color32::LIGHT_RED,error);if !self.busy&&self.snapshot.scenarios.is_empty()&&ui.button("Retry loading").clicked(){self.request(json!({"command":"snapshot"}));}}
            ui.separator();
            if self.history {self.history_panel(ui);} else {
                egui::ScrollArea::vertical().id_salt("report").show(ui,|ui| {
                    if let Some(r)=&self.current {
                        Self::heading(ui,&format!("Incident {} · Seed {}",r.id.chars().take(8).collect::<String>(),r.seed));
                        if let Some(s)=self.snapshot.scenarios.iter().find(|s|s.name==r.scenario) {ui.label(&s.mission);}
                        ui.label(RichText::new(format!("{} attacker changes at generation · {} source encoding layers",r.broken.len(),r.level)).color(MUTED));
                        ui.add(egui::Label::new(RichText::new(&r.folder).monospace().size(12.0).color(MUTED)).wrap().selectable(true));ui.separator();
                    }
                    egui::Frame::new().fill(SURFACE).stroke(egui::Stroke::new(1.0_f32,BORDER)).corner_radius(8.0).inner_margin(18.0).show(ui,|ui|{
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Investigation notes").strong());
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(&self.report).wrap().selectable(true));
                    });
                    if let Some(r)=&self.current {
                        ui.add_space(18.0);
                        egui::CollapsingHeader::new("Incident activity & previous intel").show(ui,|ui| {for event in r.events.iter().rev(){ui.label(event);ui.separator();}});
                    }
                });
            }
        });
        self.setup_dialog(ctx);
        if self.confirm_end {
            egui::Modal::new(egui::Id::new("end")).show(ctx,|ui| {
                ui.set_width(420.0);ui.heading("End this incident?");ui.label("The run will be recorded as abandoned. Your generated repository stays on disk.");
                ui.horizontal(|ui| {if ui.button("End run").clicked(){self.confirm_end=false;self.action("end");}if ui.button("Keep investigating").clicked(){self.confirm_end=false;}});
            });
        }
    }
}
