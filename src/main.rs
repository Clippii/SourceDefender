#![cfg_attr(windows, windows_subsystem = "windows")]
mod backend;
mod editor;
mod workspace;

use backend::{Action, Run, Snapshot};
use editor::Editor;
use eframe::egui::{self, Color32, RichText, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::{Path, PathBuf}, sync::mpsc, time::{Duration, Instant}};

const BG: Color32 = Color32::from_rgb(15, 19, 26);
const PANEL: Color32 = Color32::from_rgb(23, 29, 39);
const BORDER: Color32 = Color32::from_rgb(43, 53, 68);
const MUTED: Color32 = Color32::from_rgb(145, 160, 179);
const ACCENT: Color32 = Color32::from_rgb(124, 222, 183);
const AMBER: Color32 = Color32::from_rgb(245, 190, 110);

fn main() -> eframe::Result {
    #[cfg(windows)]
    let renderer = eframe::Renderer::Wgpu;
    #[cfg(not(windows))]
    let renderer = eframe::Renderer::Glow;
    eframe::run_native("SourceDefender", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("SourceDefender")
            .with_inner_size([1360.0, 880.0]).with_min_inner_size([1080.0, 720.0])
            .with_icon(std::sync::Arc::new(app_icon())),
        renderer, ..Default::default()
    }, Box::new(|cc| Ok(Box::new(Defender::new(&cc.egui_ctx)))))
}

fn app_icon() -> egui::IconData {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 { for x in 0..32 {
        let stroke = (8..24).contains(&x) && ((7..11).contains(&y) || (14..18).contains(&y) || (21..25).contains(&y))
            || (8..12).contains(&x) && (10..16).contains(&y) || (20..24).contains(&x) && (17..23).contains(&y);
        rgba.extend(if stroke { [124, 222, 183, 255] } else { [15, 19, 26, 255] });
    }}
    egui::IconData { rgba, width: 32, height: 32 }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen { Home, Setup, Workspace, Archive, Agent, Manual, Settings }
enum Navigation { Screen(Screen), Open(Run), End, Quit }

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Preferences { font_size: f32 }
impl Default for Preferences { fn default() -> Self { Self { font_size: 14.0 } } }
fn preferences_path() -> PathBuf {
    let root = if let Some(configured) = std::env::var_os("SOURCEDEFENDER_DATA_DIR") {
        PathBuf::from(configured)
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(backend::directory).join("SourceDefender")
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(backend::directory).join("Library/Application Support/SourceDefender")
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(backend::directory).join(".local/share")
        }).join("SourceDefender")
    };
    root.join("ui").join("preferences.json")
}

struct Defender {
    snapshot: Snapshot,
    current: Option<Run>,
    editor: Option<Editor>,
    screen: Screen,
    scenario: usize,
    language: usize,
    level: usize,
    seed: String,
    folder: String,
    report: String,
    error: Option<String>,
    refresh_error: Option<String>,
    busy: bool,
    refreshing: bool,
    last_refresh: Instant,
    pending_request: Option<Value>,
    pending_navigation: Option<Navigation>,
    confirm_end: bool,
    show_debrief: bool,
    allow_close: bool,
    preferences: Preferences,
    settings_notice: String,
    receiver: mpsc::Receiver<(String, Result<Value, String>)>,
    sender: mpsc::Sender<(String, Result<Value, String>)>,
}
impl Defender {
    fn new(ctx: &egui::Context) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG; visuals.window_fill = PANEL; visuals.extreme_bg_color = BG;
        visuals.override_text_color = Some(Color32::from_rgb(226, 232, 241));
        visuals.selection.bg_fill = Color32::from_rgb(37, 73, 70);
        visuals.widgets.active.bg_fill = Color32::from_rgb(44, 75, 73);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(31, 39, 51);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.button_padding = Vec2::new(13.0, 9.0);
            s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
            s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        });
        let mut preferences: Preferences = std::fs::read(preferences_path()).ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        if !preferences.font_size.is_finite() { preferences.font_size = 14.0; }
        preferences.font_size = preferences.font_size.clamp(12.0, 24.0);
        let (sender, receiver) = mpsc::channel();
        let mut app = Self {
            snapshot: Snapshot::default(), current: None, editor: None, screen: Screen::Home,
            scenario: 0, language: 0, level: 0, seed: String::new(),
            folder: backend::default_folder().to_string_lossy().into(), report: String::new(),
            error: None, refresh_error: None, busy: false, refreshing: false,
            last_refresh: Instant::now(), pending_request: None, pending_navigation: None,
            confirm_end: false, show_debrief: false, allow_close: false, preferences,
            settings_notice: String::new(), receiver, sender,
        };
        app.request(json!({"command":"snapshot"})); app
    }
    fn request(&mut self, mut request: Value) {
        if self.busy { return; }
        let command = request["command"].as_str().unwrap_or("").to_owned();
        let refresh = command == "refresh";
        if self.refreshing {
            if !refresh { self.pending_request = Some(request); self.busy = true; self.error = None; if let Some(editor)=&mut self.editor{editor.set_read_only(true);} }
            return;
        }
        if refresh { self.refreshing = true; request["command"] = json!("snapshot"); }
        else { self.busy = true; self.error = None; if let Some(editor)=&mut self.editor{editor.set_read_only(true);} }
        let sender = self.sender.clone();
        std::thread::spawn(move || { let result = backend::call(request); let _ = sender.send((command, result)); });
    }
    fn receive(&mut self) {
        while let Ok((command, result)) = self.receiver.try_recv() {
            let refresh = command == "refresh";
            if refresh { self.refreshing = false; } else { self.busy = false; }
            self.last_refresh = Instant::now();
            match result {
                Err(error) => if refresh { self.refresh_error = Some(error); } else { self.error = Some(error); },
                Ok(data) if command == "snapshot" || refresh => match serde_json::from_value::<Snapshot>(data) {
                    Err(error) => self.error = Some(format!("Could not read local operations: {error}")),
                    Ok(snapshot) => {
                        self.snapshot = snapshot; self.refresh_error = None;
                        if let Some(current) = &self.current {
                            if let Some(updated) = self.snapshot.runs.iter().find(|r| r.id == current.id).cloned() {
                                self.report = updated.events.last().cloned().unwrap_or_default();
                                self.current = Some(updated);
                            }
                        }
                        if let Some(editor) = &mut self.editor {
                            editor.set_read_only(self.busy || self.current.as_ref().is_none_or(|r| r.is_agent() || r.status != "Active"));
                            if self.screen == Screen::Workspace { editor.refresh(); }
                        }
                    }
                },
                Ok(data) => match serde_json::from_value::<Action>(data) {
                    Err(error) => self.error = Some(format!("Could not read operation result: {error}")),
                    Ok(action) => {
                        self.snapshot.runs.retain(|r| r.id != action.run.id);
                        self.snapshot.runs.insert(0, action.run.clone());
                        self.report = action.report;
                        if command == "generate" {
                            self.open_run(action.run);
                        } else {
                            self.current = Some(action.run);
                            if command=="end" && self.editor.is_none() {
                                if let Some(run)=self.current.clone(){self.open_run(run);}
                            }
                            let read_only = !self.active();
                            if let Some(editor) = &mut self.editor { editor.set_read_only(read_only); }
                            if command == "end" || self.current.as_ref().is_some_and(|r| r.status == "Contained") {
                                self.show_debrief = true;
                            }
                        }
                    }
                },
            }
        }
        if !self.refreshing {
            if let Some(request) = self.pending_request.take() { self.busy = false; self.request(request); }
        }
        let read_only=self.busy || !self.active();
        if let Some(editor)=&mut self.editor{editor.set_read_only(read_only);}
    }
    fn active(&self) -> bool {
        self.current.as_ref().is_some_and(|r| !r.is_agent() && r.status == "Active")
    }
    fn active_human(&self) -> Option<Run> {
        self.snapshot.runs.iter().find(|r| !r.is_agent() && r.status == "Active").cloned()
    }
    fn dirty(&self) -> bool { self.editor.as_ref().is_some_and(Editor::dirty) }
    fn navigate(&mut self, navigation: Navigation, ctx: &egui::Context) {
        if self.busy { return; }
        if self.dirty() { self.pending_navigation = Some(navigation); }
        else { self.perform_navigation(navigation, ctx); }
    }
    fn perform_navigation(&mut self, navigation: Navigation, ctx: &egui::Context) {
        self.error = None; self.show_debrief = false;
        match navigation {
            Navigation::Screen(screen) => self.screen = screen,
            Navigation::Open(run) => self.open_run(run),
            Navigation::End => self.action("end"),
            Navigation::Quit => { self.allow_close = true; ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
    }
    fn open_run(&mut self, run: Run) {
        // Every caller must resolve unsaved edits before replacing a workspace.
        if self.dirty() {
            self.error = Some("Save or discard your open repairs before changing operations.".into()); return;
        }
        let reuse = self.current.as_ref().is_some_and(|old| old.id == run.id) && self.editor.is_some();
        if !reuse {
            self.editor = match Editor::open(Path::new(&run.folder)) {
                Ok(mut editor) => {
                    editor.set_font_size(self.preferences.font_size);
                    editor.set_read_only(run.is_agent() || run.status != "Active");
                    if let Err(error) = editor.open_file("README.md") { self.error = Some(error); }
                    Some(editor)
                }
                Err(error) => { self.error = Some(error); None }
            };
        }
        self.report = run.events.last().cloned().unwrap_or_default();
        self.current = Some(run); self.screen = Screen::Workspace;
        let read_only = !self.active();
        if let Some(editor) = &mut self.editor { editor.set_read_only(read_only); }
    }
    fn save_all(&mut self) -> Result<(), String> {
        self.save_open_buffers(false)
    }
    fn save_open_buffers(&mut self, recover_ended: bool) -> Result<(), String> {
        if self.busy { return Err("Wait for the current operation to finish before saving.".into()); }
        let run=self.current.as_ref().ok_or("No operation selected.")?;
        // Recovery only writes already-dirty buffers after explicit user action.
        // It does not reopen an ended operation or allow new edits or scoring.
        if run.is_agent() || (run.status!="Active" && !(recover_ended && self.dirty())) {
            return Err("Only an active human operation can save new repairs.".into());
        }
        let read_only=run.status!="Active";
        let folder=std::fs::canonicalize(&self.current.as_ref().ok_or("No operation selected.")?.folder).map_err(|e|format!("The incident folder is unavailable: {e}"))?;
        let editor=self.editor.as_mut().ok_or("The operation workspace is unavailable.")?;
        if editor.root()!=folder.as_path(){return Err("The open workspace belongs to another operation.".into());}
        if recover_ended {editor.set_read_only(false);}
        let result=editor.save_all();
        editor.set_read_only(read_only);
        result
    }
    fn action(&mut self, command: &str) {
        if !self.active() { self.error = Some("Choose an active human operation first.".into()); return; }
        if command == "validate" {
            if let Err(error) = self.save_all() { self.error = Some(format!("Validation cancelled: {error}")); return; }
        }
        if let Some(run) = &self.current { self.request(json!({"command":command,"id":run.id})); }
    }
    fn caption(ui: &mut egui::Ui, text: &str) { ui.label(RichText::new(text).size(11.0).strong().color(MUTED)); }
    fn card() -> egui::Frame {
        egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0_f32, BORDER)).corner_radius(9.0).inner_margin(20.0)
    }
    fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(RichText::new(label).color(BG).strong()).fill(ACCENT).min_size(Vec2::new(160.0, 42.0)))
    }
    fn page_title(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
        Self::caption(ui, eyebrow); ui.add_space(6.0);
        ui.label(RichText::new(title).size(32.0).strong());
        ui.label(RichText::new(subtitle).color(MUTED)); ui.add_space(18.0);
    }
    fn menu(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("main_navigation").exact_width(205.0).resizable(false)
            .frame(egui::Frame::new().fill(Color32::from_rgb(11, 15, 21)).inner_margin(20.0))
            .show(ctx, |ui| {
                ui.add_space(12.0); ui.label(RichText::new("S / D").size(24.0).monospace().strong().color(ACCENT));
                ui.label(RichText::new("SOURCEDEFENDER").size(12.0).strong()); Self::caption(ui, "LOCAL DEFENSE OPERATIONS");
                ui.add_space(32.0);
                for (label, screen) in [("Main menu",Screen::Home),("New operation",Screen::Setup),("Operation archive",Screen::Archive),("Agent watch",Screen::Agent),("Field manual",Screen::Manual),("Settings",Screen::Settings)] {
                    if ui.add_enabled(!self.busy, egui::Button::new(label).selected(self.screen == screen).min_size(Vec2::new(165.0,38.0))).clicked() {
                        self.navigate(Navigation::Screen(screen),ctx);
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(RichText::new(format!("v{}",env!("CARGO_PKG_VERSION"))).small().color(MUTED));
                    ui.label(RichText::new("Rust native · Fully local").small().color(MUTED));
                    ui.separator();
                    if let Some(run) = self.active_human() {
                        ui.label(RichText::new(format!("LIVE  {}",backend::time(run.seconds()))).monospace().color(AMBER));
                        ui.label(RichText::new("Your operation clock is running.").small().color(MUTED));
                    }
                });
            });
    }
    fn home(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::page_title(ui,"BLUE TEAM / SOURCE INTEGRITY","Every change leaves evidence.","Investigate compromised repositories. Repair trusted behavior. Contain the incident.");
        ui.columns(2, |columns| {
            Self::card().show(&mut columns[0], |ui| {
                Self::caption(ui,"01 / NEW DEPLOYMENT"); ui.add_space(12.0);
                ui.label(RichText::new("Defend the source.").size(28.0).strong());
                ui.label("Choose your service, language, and threat level. A local repository becomes your investigation scene.");
                ui.add_space(24.0);
                if Self::primary(ui,"New operation").clicked() { self.navigate(Navigation::Screen(Screen::Setup),ctx); }
                ui.add_space(10.0); ui.label(RichText::new("6 scenarios  /  4 languages  /  16 attack types").small().color(MUTED));
            });
            Self::card().show(&mut columns[1], |ui| {
                Self::caption(ui,"02 / ACTIVE OPERATION"); ui.add_space(12.0);
                if let Some(run) = self.active_human() {
                    ui.label(RichText::new(&run.scenario).size(24.0).strong());
                    ui.label(format!("{} · {}",run.language,self.difficulty_name(run.level)));
                    ui.label(RichText::new(backend::time(run.seconds())).size(32.0).monospace().color(AMBER));
                    ui.label(RichText::new("Clock includes penalties and continues outside the workspace.").small().color(MUTED));
                    if ui.button("Resume operation").clicked() { self.navigate(Navigation::Open(run),ctx); }
                } else {
                    ui.label(RichText::new("No active operation").size(24.0).strong());
                    ui.label("Your next deployment starts when you begin an operation.");
                    ui.add_space(23.0); ui.label(RichText::new("READY FOR DEPLOYMENT").monospace().color(ACCENT));
                }
            });
        });
        ui.add_space(22.0);
        Self::card().show(ui, |ui| {
            Self::caption(ui,"YOUR SERVICE RECORD");
            let contained = self.snapshot.runs.iter().filter(|r| !r.is_agent() && r.status == "Contained").count();
            let best = self.snapshot.runs.iter().filter(|r| !r.is_agent() && r.status == "Contained").map(|r|r.score).max().unwrap_or(0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{contained} operations contained")).strong()); ui.separator();
                ui.label(format!("Best score  {best}")); ui.separator();
                if ui.button("Operation archive").clicked() { self.navigate(Navigation::Screen(Screen::Archive),ctx); }
                if ui.button("Field manual").clicked() { self.navigate(Navigation::Screen(Screen::Manual),ctx); }
            });
        });
        ui.add_space(20.0);
        ui.label(RichText::new("THE BRIEF").small().strong().color(ACCENT));
        ui.label("A trusted maintenance account has been compromised. Follow the application logic, investigate the source, remove attacker changes, and preserve legitimate behavior.");
        ui.label(RichText::new("Real source files. No project code is executed by the game.").small().color(MUTED));
    }
    fn difficulty_name(&self, level: usize) -> &str {
        self.snapshot.difficulties.get(level).map(|d|d.name.as_str()).unwrap_or("Unknown difficulty")
    }
    fn setup(&mut self, ui: &mut egui::Ui) {
        Self::page_title(ui,"DEPLOYMENT CONFIGURATION","Build your operation.","Choose what you maintain and how hard the attacker worked to hide their changes.");
        if self.snapshot.scenarios.is_empty() || self.snapshot.languages.is_empty() || self.snapshot.difficulties.is_empty() {
            ui.label("Loading the local scenario library…"); return;
        }
        self.scenario = self.scenario.min(self.snapshot.scenarios.len()-1);
        self.language = self.language.min(self.snapshot.languages.len()-1);
        self.level = self.level.min(self.snapshot.difficulties.len()-1);
        ui.add_enabled_ui(!self.busy, |ui| {
            Self::card().show(ui, |ui| {
                Self::caption(ui,"01 / TARGET SERVICE");
                ui.columns(2, |cols| {
                    egui::ComboBox::from_id_salt("scenario").width(cols[0].available_width())
                        .selected_text(&self.snapshot.scenarios[self.scenario].name).show_ui(&mut cols[0],|ui|{
                            for (index,scenario) in self.snapshot.scenarios.iter().enumerate() { ui.selectable_value(&mut self.scenario,index,&scenario.name); }
                        });
                    cols[0].label(RichText::new(&self.snapshot.scenarios[self.scenario].description).color(MUTED));
                    Self::caption(&mut cols[1],"MISSION");
                    cols[1].label(&self.snapshot.scenarios[self.scenario].mission);
                });
                ui.add_space(10.0); Self::caption(ui,"02 / SOURCE LANGUAGE");
                ui.horizontal_wrapped(|ui| { for (index,language) in self.snapshot.languages.iter().enumerate() {
                    ui.selectable_value(&mut self.language,index,language);
                }});
            });
            ui.add_space(12.0); Self::caption(ui,"03 / THREAT LEVEL");
            let count = self.snapshot.difficulties.len();
            ui.columns(count, |cols| { for (index,column) in cols.iter_mut().enumerate() {
                let difficulty = &self.snapshot.difficulties[index];
                egui::Frame::new().fill(if self.level == index { Color32::from_rgb(29,51,49) } else {PANEL})
                    .stroke(egui::Stroke::new(if self.level == index {2.0_f32}else{1.0_f32},if self.level == index {ACCENT}else{BORDER}))
                    .corner_radius(7.0).inner_margin(14.0).show(column,|ui|{
                        if ui.selectable_label(self.level==index,RichText::new(&difficulty.name).strong()).clicked() {self.level=index;}
                        ui.label(format!("{} attacks",difficulty.bugs)); ui.label(format!("{index} encoding layers"));
                        ui.label(RichText::new(format!("+{}s failed scan",difficulty.penalty)).color(AMBER));
                        ui.label(RichText::new(format!("+{}s intel",difficulty.hint_cost)).small().color(MUTED));
                    });
            }});
            ui.label(RichText::new(["Titles and file paths. Plain source values.","Titles without paths. One layer of encoded values.","Category counts. Two encoding layers; no Git history.","Remaining count only. Three encoding layers; no Git history."][self.level.min(3)]).color(MUTED));
            ui.add_space(12.0);
            Self::card().show(ui,|ui|{
                Self::caption(ui,"04 / REPOSITORY");
                ui.horizontal(|ui|{
                    ui.label("Seed");
                    ui.add(egui::TextEdit::singleline(&mut self.seed).hint_text("Optional reproducible seed").desired_width(300.0).char_limit(80));
                });
                ui.horizontal(|ui|{
                    ui.add(egui::TextEdit::singleline(&mut self.folder).desired_width((ui.available_width()-105.0).max(200.0)));
                    if ui.button("Browse…").clicked() { match backend::choose_folder() {Ok(Some(folder))=>self.folder=folder,Ok(None)=>{},Err(error)=>self.error=Some(error)} }
                });
                ui.label(RichText::new("Creates a new subfolder. The clock begins when you start; setup time is free.").small().color(MUTED));
            });
            if self.active_human().is_some() {ui.colored_label(AMBER,"An operation is already active. Resume it from the main menu, or end it before starting another.");}
            ui.add_space(12.0);
            ui.add_enabled_ui(self.active_human().is_none() && !self.folder.trim().is_empty(),|ui|{
                if Self::primary(ui,"Begin operation").clicked() {
                    if self.dirty() {self.error=Some("Return to your workspace and save or discard repairs before starting.".into());return;}
                    self.request(json!({"command":"generate","folder":self.folder.trim(),"level":self.level,"scenario":self.scenario,"language":self.snapshot.languages[self.language],"seed":self.seed.trim()}));
                }
            });
        });
    }
    fn archive(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::page_title(ui,"LOCAL SERVICE RECORD","Operation archive","Revisit an investigation and inspect its saved repository. Ended operations open read-only.");
        if self.snapshot.runs.is_empty() {ui.label("Your first operation will appear here.");}
        for run in self.snapshot.runs.clone() {
            Self::card().show(ui,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.label(RichText::new(&run.scenario).size(18.0).strong());ui.separator();
                    ui.colored_label(if run.status=="Contained"{ACCENT}else if run.status=="Active"{AMBER}else{MUTED},&run.status);
                    ui.label(format!("{} · {} · {}",run.mode_label(),run.language,self.difficulty_name(run.level)));
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label(format!("{}  /  {} scans  /  +{}s penalties",backend::time(run.seconds()),run.attempts,run.penalty_seconds));
                    if run.status=="Contained" {ui.label(RichText::new(format!("Score {}",run.score)).color(ACCENT));}
                    if ui.button(if run.status=="Active" && !run.is_agent() {"Resume operation"}else{"Review operation"}).clicked() {
                        self.navigate(Navigation::Open(run.clone()),ctx);
                    }
                });
            });ui.add_space(8.0);
        }
    }
    fn agent(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::page_title(ui,"MCP / LIVE OBSERVATION","Agent watch","Observe the investigation without spending the agent's time or changing its run.");
        let run = self.snapshot.runs.iter().find(|r|r.is_agent()&&r.status=="Active")
            .or_else(||self.snapshot.runs.iter().find(|r|r.is_agent())).cloned();
        Self::card().show(ui,|ui|{
            if let Some(run)=run {
                ui.label(RichText::new(&run.scenario).size(24.0).strong());
                ui.label(format!("{} · {} · {}",run.language,self.difficulty_name(run.level),run.status));
                ui.label(RichText::new(backend::time(run.seconds())).monospace().size(30.0).color(AMBER));
                ui.label(format!("{} game tool calls · {} validation attempts",run.tool_calls,run.attempts));
                if Self::primary(ui,"Watch operation").clicked(){self.navigate(Navigation::Open(run),ctx);}
            } else {ui.label(RichText::new("No agent operation yet").size(24.0).strong());}
            ui.add_space(14.0);ui.label("Connect an MCP client to the bundled engine with the --mcp argument. Ask the agent to start a practice run and investigate the generated folder with its own file tools.");
            ui.label(RichText::new("The observer workspace is read-only. Human operations and agent practice remain separate.").color(MUTED));
        });
    }
    fn manual(ui:&mut egui::Ui) {
        Self::page_title(ui,"FIELD MANUAL","Restore trust. Preserve behavior.","Your mission is to remove attacker changes while keeping the service working.");
        for (number,title,text) in [
            ("01","Read the scene","Start with README.md, docs/architecture.md, and HANDBOOK.md. Understand the expected behavior before changing code."),
            ("02","Follow the evidence","Explore application source, configuration, and automation. New operations on the two harder difficulties have no Git history. Existing incidents retain their original history. Hidden files are included in the explorer."),
            ("03","Repair the real files","Use the built-in editor, or your preferred external tools. Save all writes your open repairs. New operations on the two harder difficulties have no Git history."),
            ("04","Validate repairs","Validation saves edited incident files first. A failed scan adds the difficulty's time penalty. Intel reveals a new control and also costs time."),
            ("05","Contain the incident","A clean scan stops the clock and records your final score. The repository stays on disk. The main menu and closing the game do not pause the clock."),
        ] {
            Self::card().show(ui,|ui|{
                Self::caption(ui,number);ui.label(RichText::new(title).size(19.0).strong());ui.label(text);
            });ui.add_space(8.0);
        }
        ui.label(RichText::new("The game never executes generated source, installation hooks, or project tasks. Validation checks the known incident controls.").small().color(MUTED));
    }
    fn settings(&mut self, ui:&mut egui::Ui) {
        Self::page_title(ui,"DISPLAY / LOCAL PREFERENCES","Settings","Make the investigation workspace comfortable to read.");
        Self::card().show(ui,|ui|{
            Self::caption(ui,"EDITOR FONT SIZE");
            if ui.add(egui::Slider::new(&mut self.preferences.font_size,12.0..=24.0).suffix(" px")).changed() {
                if let Some(editor)=&mut self.editor {editor.set_font_size(self.preferences.font_size);}
                self.settings_notice="Preview updated. Save preferences to keep this setting.".into();
            }
            ui.label(RichText::new("def verify_source(change):").monospace().size(self.preferences.font_size).color(ACCENT));
            ui.add_space(14.0);
            if ui.button("Save preferences").clicked() {
                let path=preferences_path();
                let result=(||->Result<(),String>{
                    std::fs::create_dir_all(path.parent().ok_or("Preferences folder unavailable")?).map_err(|e|e.to_string())?;
                    let data=serde_json::to_vec_pretty(&self.preferences).map_err(|e|e.to_string())?;
                    std::fs::write(&path,data).map_err(|e|e.to_string())
                })();
                match result {Ok(())=>self.settings_notice="Preferences saved locally.".into(),Err(error)=>self.error=Some(format!("Could not save preferences: {error}"))}
            }
            ui.label(RichText::new(&self.settings_notice).color(MUTED));
        });
        ui.add_space(18.0);ui.label("Display preferences do not change the difficulty or timing rules of an operation.");
    }
    fn workspace(&mut self, ctx: &egui::Context) {
        let Some(run)=self.current.clone() else {self.screen=Screen::Home;return;};
        let playable=self.active();
        egui::TopBottomPanel::top("operation_hud").frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(18,12))).show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new("S / D").monospace().strong().color(ACCENT));ui.separator();
                ui.label(RichText::new(&run.scenario).strong());
                ui.label(RichText::new(format!("{} / {}",run.language,self.difficulty_name(run.level))).small().color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    if ui.add_enabled(!self.busy,egui::Button::new("Main menu")).clicked(){self.navigate(Navigation::Screen(Screen::Home),ctx);}
                    ui.label(RichText::new(backend::time(run.seconds())).monospace().size(26.0).color(if run.status=="Contained"{ACCENT}else{AMBER}));
                    Self::caption(ui,if run.status=="Active"{"LIVE / INCLUDING PENALTIES"}else{&run.status});
                });
            });
            ui.horizontal_wrapped(|ui|{
                ui.add_enabled_ui(!self.busy && playable && self.editor.is_some(),|ui|{
                    if Self::primary(ui,"Validate repairs").clicked(){self.action("validate");}
                    if ui.button("Save all").clicked(){if let Err(error)=self.save_all(){self.error=Some(error);}}
                });
                let hint_cost=self.snapshot.difficulties.get(run.level).map(|d|d.hint_cost).unwrap_or(0);
                if ui.add_enabled(!self.busy&&playable,egui::Button::new(format!("Request intel (+{hint_cost}s)"))).clicked(){self.action("hint");}
                if ui.add_enabled(!self.busy&&playable,egui::Button::new("End operation")).clicked(){self.confirm_end=true;}
                ui.separator();ui.label(RichText::new(format!("{} scans  ·  +{}s penalties  ·  {} intel",run.attempts,run.penalty_seconds,run.hints)).small().color(MUTED));
                if run.is_agent(){ui.label(RichText::new(format!("{} MCP calls / OBSERVER",run.tool_calls)).small().color(ACCENT));}
                else if !playable {ui.label(RichText::new("ARCHIVED / READ ONLY").small().color(ACCENT));}
            });
        });
        egui::SidePanel::right("incident_intel").default_width(295.0).width_range(245.0..=420.0).frame(egui::Frame::new().fill(PANEL).inner_margin(16.0)).show(ctx,|ui|{
            Self::caption(ui,"OPERATION INTEL");
            egui::ScrollArea::vertical().id_salt("intel_scroll").show(ui,|ui|{
                if run.status=="Contained" {
                    ui.label(RichText::new("INCIDENT CONTAINED").strong().color(ACCENT));ui.label(RichText::new(format!("Score {}",run.score)).size(25.0).strong());
                    if ui.button("View debrief").clicked(){self.show_debrief=true;}
                }
                if !playable && !run.is_agent() && self.dirty() {
                    ui.colored_label(AMBER,"This operation ended elsewhere. Your unsaved repairs are preserved.");
                    ui.label(RichText::new("Saving these existing edits changes the repository files, not the recorded result.").small().color(MUTED));
                    if ui.add_enabled(!self.busy,egui::Button::new("Save preserved repairs")).clicked(){
                        if let Err(error)=self.save_open_buffers(true){self.error=Some(error);}
                    }
                    ui.separator();
                }
                ui.label(RichText::new(format!("{} attacker changes at deployment",run.attack_count)).small().color(MUTED));
                ui.label(RichText::new(format!("{} encoding layers",run.level)).small().color(MUTED));
                ui.separator();
                if run.findings.is_empty() {
                    ui.label("Read the briefing and trace the repository. Validation results appear here.");
                }
                for (index,finding) in run.findings.iter().enumerate() {
                    ui.push_id(index,|ui|{
                        ui.label(RichText::new(&finding.title).strong());
                        ui.label(RichText::new(if finding.count>1{format!("{} · {}",finding.category,finding.count)}else{finding.category.clone()}).small().color(MUTED));
                        if let Some(path)=&finding.path {
                            if ui.link(path).clicked() {if let Some(editor)=&mut self.editor {if let Err(error)=editor.open_file(path){self.error=Some(error);}}}
                        }
                        ui.add_space(8.0);
                    });
                }
                ui.separator();egui::CollapsingHeader::new("Latest report").default_open(true).show(ui,|ui|{
                    ui.add(egui::Label::new(&self.report).wrap().selectable(true));
                });
                egui::CollapsingHeader::new("Activity & previous intel").show(ui,|ui|{
                    for event in run.events.iter().rev(){ui.add(egui::Label::new(event).wrap().selectable(true));ui.separator();}
                });
                ui.add_space(12.0);Self::caption(ui,"INCIDENT ID");ui.label(RichText::new(&run.id).small().monospace().color(MUTED));
                ui.label(RichText::new(format!("Seed {}",run.seed)).small().color(MUTED));
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(0.0)).show(ctx,|ui|{
            if let Some(editor)=&mut self.editor {editor.ui(ui,!playable || self.busy);}
            else {
                ui.add_space(30.0);ui.label("The repository workspace could not be opened.");
                if ui.button("Retry opening repository").clicked(){self.open_run(run.clone());}
                if ui.button("Open folder externally").clicked(){if let Err(error)=backend::open_folder(Path::new(&run.folder)){self.error=Some(error);}}
            }
        });
    }
    fn dialogs(&mut self, ctx:&egui::Context) {
        if self.confirm_end {
            egui::Modal::new(egui::Id::new("end_operation")).show(ctx,|ui|{
                ui.set_width(420.0);ui.heading("End this operation?");ui.label("It will be recorded as abandoned. Your generated repository stays on disk.");
                ui.horizontal(|ui|{
                    if ui.button("End operation").clicked(){self.confirm_end=false;self.navigate(Navigation::End,ctx);}
                    if ui.button("Keep investigating").clicked(){self.confirm_end=false;}
                });
            });
        }
        if self.pending_navigation.is_some() {
            egui::Modal::new(egui::Id::new("unsaved_repairs")).show(ctx,|ui|{
                ui.set_width(440.0);ui.heading("Unsaved repairs");ui.label("Save your open repairs before leaving, or explicitly discard the unsaved changes.");
                if self.active(){ui.label("The operation clock keeps running.");}
                if !self.active(){ui.colored_label(AMBER,"The operation has ended. Saving preserves your existing edits on disk; the recorded result stays unchanged.");}
                ui.horizontal_wrapped(|ui|{
                    if ui.button("Save and continue").clicked() {
                        match self.save_open_buffers(true) {
                            Ok(())=>if let Some(nav)=self.pending_navigation.take(){self.perform_navigation(nav,ctx);},
                            Err(error)=>self.error=Some(error),
                        }
                    }
                    if ui.button("Discard and continue").clicked(){
                        self.editor=None;
                        if let Some(nav)=self.pending_navigation.take(){self.perform_navigation(nav,ctx);}
                    }
                    if ui.button("Keep investigating").clicked(){self.pending_navigation=None;}
                });
                if let Some(error)=&self.error{ui.colored_label(Color32::LIGHT_RED,error);}
            });
        }
        if self.show_debrief && self.pending_navigation.is_none() {
            if let Some(run)=self.current.clone() {
                egui::Modal::new(egui::Id::new("operation_debrief")).show(ctx,|ui|{
                    ui.set_width(530.0);Self::caption(ui,"OPERATION DEBRIEF");
                    ui.label(RichText::new(if run.status=="Contained"{"Incident contained."}else{"Operation ended."}).size(34.0).strong().color(if run.status=="Contained"{ACCENT}else{AMBER}));
                    ui.label(&run.scenario);ui.add_space(14.0);
                    ui.columns(3,|cols|{
                        Self::caption(&mut cols[0],"FINAL TIME");cols[0].label(RichText::new(backend::time(run.seconds())).size(25.0).monospace());
                        Self::caption(&mut cols[1],"PENALTIES");cols[1].label(RichText::new(format!("+{}s",run.penalty_seconds)).size(25.0).monospace());
                        Self::caption(&mut cols[2],"SCORE");cols[2].label(RichText::new(if run.status=="Contained"{run.score.to_string()}else{"—".into()}).size(25.0).monospace());
                    });
                    ui.add_space(14.0);ui.label(format!("{} validations · {} intel requests · {}",run.attempts,run.hints,self.difficulty_name(run.level)));
                    ui.label("Your repository and investigation record stay on disk for review.");
                    ui.add_space(14.0);ui.horizontal(|ui|{
                        if Self::primary(ui,"Review repository").clicked(){self.show_debrief=false;}
                        if ui.button("Main menu").clicked(){self.navigate(Navigation::Screen(Screen::Home),ctx);}
                    });
                });
            }
        }
    }
}
impl eframe::App for Defender {
    fn update(&mut self, ctx:&egui::Context, _frame:&mut eframe::Frame) {
        self.receive();
        if ctx.input(|i|i.viewport().close_requested()) && !self.allow_close {
            if self.busy {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.error=Some("Wait for the current operation to finish before closing.".into());
            } else if self.dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending_navigation=Some(Navigation::Quit);
            }
        }
        if !self.busy && !self.refreshing && !self.snapshot.scenarios.is_empty() && self.last_refresh.elapsed()>=Duration::from_millis(1800) {
            self.request(json!({"command":"refresh"}));
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        egui::TopBottomPanel::top("system_status").frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(14,5))).show(ctx,|ui|{
            if self.busy {ui.horizontal(|ui|{ui.spinner();ui.label("Working on the local operation…");});}
            if let Some(error)=self.error.clone(){ui.horizontal_wrapped(|ui|{
                ui.colored_label(Color32::LIGHT_RED,error);
                if ui.small_button("Dismiss").clicked(){self.error=None;}
                if self.snapshot.scenarios.is_empty() && !self.busy && ui.button("Retry loading").clicked(){self.request(json!({"command":"snapshot"}));}
            });}
            if let Some(error)=&self.refresh_error{ui.label(RichText::new(format!("Live update delayed: {error}")).small().color(AMBER));}
        });
        if self.screen==Screen::Workspace {self.workspace(ctx);}
        else {
            self.menu(ctx);
            egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(30.0)).show(ctx,|ui|{
                egui::ScrollArea::vertical().id_salt("game_page").show(ui,|ui|{
                    ui.add_enabled_ui(!self.busy,|ui|match self.screen {
                        Screen::Home=>self.home(ui,ctx),Screen::Setup=>self.setup(ui),
                        Screen::Archive=>self.archive(ui,ctx),Screen::Agent=>self.agent(ui,ctx),
                        Screen::Manual=>Self::manual(ui),Screen::Settings=>self.settings(ui),Screen::Workspace=>{},
                    });
                });
            });
        }
        self.dialogs(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Defender {
        let (sender,receiver)=mpsc::channel();
        Defender {
            snapshot:Snapshot::default(),current:None,editor:None,screen:Screen::Home,
            scenario:0,language:0,level:0,seed:String::new(),folder:String::new(),report:String::new(),
            error:None,refresh_error:None,busy:false,refreshing:false,last_refresh:Instant::now(),
            pending_request:None,pending_navigation:None,confirm_end:false,show_debrief:false,
            allow_close:false,preferences:Preferences::default(),settings_notice:String::new(),receiver,sender,
        }
    }
    fn record(id:&str,mode:&str,status:&str,folder:&Path)->Value {
        json!({"Id":id,"Folder":folder,"Scenario":"Test incident","Language":"Python","Seed":"test",
            "StartedUtc":"2026-10-28T12:00:00Z","FinishedUtc":null,"Status":status,"Level":0,
            "PenaltySeconds":0,"Attempts":0,"Hints":0,"AttackCount":3,"Mode":mode,"ToolCalls":0,"Events":[],"Score":0})
    }
    #[test]
    fn shell_save_requires_active_human_even_before_editor_first_render() {
        let root=std::env::temp_dir().join(format!("sd-shell-gate-{}-{}",std::process::id(),chrono::Utc::now().timestamp_nanos_opt().unwrap()));
        std::fs::create_dir_all(&root).unwrap();std::fs::write(root.join("README.md"),"Original source").unwrap();
        let mut app=app();app.current=Some(serde_json::from_value(record("human","human","Active",&root)).unwrap());
        app.editor=Some(Editor::open(&root).unwrap());
        assert!(app.save_all().is_ok());
        app.current.as_mut().unwrap().mode="agent_practice".into();
        assert!(app.save_all().unwrap_err().contains("active human"));
        assert!(app.save_open_buffers(true).is_err(),"Agent mode must not use ended-buffer recovery");
        app.current.as_mut().unwrap().mode="human".into();
        app.current.as_mut().unwrap().status="Contained".into();
        assert!(app.save_all().is_err(),"Ended operations must be read-only");
        app.current.as_mut().unwrap().status="Active".into();app.busy=true;
        assert!(app.save_all().unwrap_err().contains("Wait"));
        assert_eq!(std::fs::read_to_string(root.join("README.md")).unwrap(),"Original source");
        drop(app);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn validation_save_failure_never_starts_or_charges_an_engine_scan() {
        let mut app=app();
        app.current=Some(serde_json::from_value(record("human","human","Active",Path::new("missing-incident-folder"))).unwrap());
        app.action("validate");
        assert!(!app.busy);
        assert!(app.pending_request.is_none());
        assert!(app.error.as_ref().unwrap().starts_with("Validation cancelled"));
        assert_eq!(app.current.as_ref().unwrap().attempts,0);
        assert_eq!(app.current.as_ref().unwrap().penalty_seconds,0);
    }
    #[test]
    fn refresh_keeps_selected_archived_agent_instead_of_another_active_agent() {
        let mut app=app();
        let archived=record("selected","agent_practice","Contained",Path::new("archived-agent"));
        app.current=Some(serde_json::from_value(archived.clone()).unwrap());
        app.refreshing=true;
        app.sender.send(("refresh".into(),Ok(json!({"Runs":[
            record("other","agent_practice","Active",Path::new("other-agent")),archived
        ],"Scenarios":[],"Difficulties":[],"Languages":[]})))).unwrap();
        app.receive();
        assert_eq!(app.current.as_ref().unwrap().id,"selected");
        assert!(!app.active());
        assert!(!app.refreshing);
    }
}
