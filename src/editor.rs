//! Native investigation editor. Source is inspected and saved, never executed.

use crate::workspace::{Repository, SearchHit};
use eframe::egui::{self, Color32, FontId, RichText};
use std::{collections::BTreeMap, ops::Range, path::Path, sync::mpsc};

const BACKGROUND: Color32 = Color32::from_rgb(18, 22, 29);
const PANEL: Color32 = Color32::from_rgb(23, 28, 36);
const MUTED: Color32 = Color32::from_rgb(139, 153, 173);
const TEXT: Color32 = Color32::from_rgb(220, 227, 238);
const ACCENT: Color32 = Color32::from_rgb(127, 184, 214);

struct Document {
    path: String,
    text: String,
    saved: String,
}

impl Document {
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Sidebar {
    Files,
    Search,
    Evidence,
}

enum Pending {
    Close(usize),
    Reload(usize),
}

struct Evidence {
    title: String,
    text: String,
}

pub struct Editor {
    repository: Repository,
    documents: Vec<Document>,
    active: Option<usize>,
    sidebar: Sidebar,
    filter: String,
    query: String,
    hits: Vec<SearchHit>,
    searched: bool,
    show_find: bool,
    focus_find: bool,
    focus_search: bool,
    find: String,
    find_after: usize,
    selection: Option<Range<usize>>,
    reveal_line: Option<usize>,
    evidence: Option<Evidence>,
    evidence_task: Option<mpsc::Receiver<Result<String, String>>>,
    pending: Option<Pending>,
    error: Option<String>,
    status: String,
    cursor: (usize, usize),
    font_size: f32,
    read_only: bool,
}

impl Editor {
    pub fn open(root: &Path) -> Result<Self, String> {
        let repository = Repository::open(root)?;
        Ok(Self {
            repository,
            documents: Vec::new(),
            active: None,
            sidebar: Sidebar::Files,
            filter: String::new(),
            query: String::new(),
            hits: Vec::new(),
            searched: false,
            show_find: false,
            focus_find: false,
            focus_search: false,
            find: String::new(),
            find_after: 0,
            selection: None,
            reveal_line: None,
            evidence: None,
            evidence_task: None,
            pending: None,
            error: None,
            status: "Local files · UTF-8".into(),
            cursor: (1, 1),
            font_size: 14.0,
            read_only: false,
        })
    }

    pub fn root(&self) -> &Path {
        self.repository.root()
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size.clamp(12.0, 24.0);
    }

    pub fn dirty(&self) -> bool {
        self.documents.iter().any(Document::dirty)
    }

    pub fn open_file(&mut self, relative: &str) -> Result<(), String> {
        let relative = relative.replace('\\', "/");
        // Resolve through Repository even if the file has an existing tab. A
        // changed symlink or junction must not turn a stale tab into a new path.
        let text = self.repository.read(&relative)?;
        let index = self.documents.iter().position(|doc| doc.path == relative);
        self.active = Some(if let Some(index) = index {
            if !self.documents[index].dirty() {
                self.documents[index].text = text.clone();
                self.documents[index].saved = text;
            }
            index
        } else {
            self.documents.push(Document {
                path: relative,
                text: text.clone(),
                saved: text,
            });
            self.documents.len() - 1
        });
        self.evidence = None;
        self.selection = None;
        self.find_after = 0;
        self.cursor = (1, 1);
        Ok(())
    }

    pub fn save_active(&mut self) -> Result<(), String> {
        if self.read_only {
            return Err("This operation is open for observation only.".into());
        }
        if let Some(index) = self.active {
            self.save_document(index)?;
        }
        self.error = None;
        Ok(())
    }

    pub fn save_all(&mut self) -> Result<(), String> {
        if self.read_only {
            return Err("This operation is open for observation only.".into());
        }
        for index in 0..self.documents.len() {
            self.save_document(index)?;
        }
        self.status = "All repairs saved".into();
        self.error = None;
        Ok(())
    }

    fn save_document(&mut self, index: usize) -> Result<(), String> {
        let document = &mut self.documents[index];
        if document.dirty() {
            self.repository
                .save(&document.path, &document.text, &document.saved)?;
            document.saved.clone_from(&document.text);
            self.status = format!("Saved {}", document.path);
        }
        Ok(())
    }

    pub fn discard_all(&mut self) -> Result<(), String> {
        // Read all replacements before discarding anything. A missing file
        // should leave the other unsaved repairs available to the investigator.
        let replacements = self
            .documents
            .iter()
            .map(|document| self.repository.read(&document.path))
            .collect::<Result<Vec<_>, _>>()?;
        for (document, text) in self.documents.iter_mut().zip(replacements) {
            document.saved = text.clone();
            document.text = text;
        }
        self.error = None;
        Ok(())
    }

    pub fn refresh(&mut self) {
        if let Err(error) = self.repository.refresh() {
            self.error = Some(error);
            return;
        }
        for document in &mut self.documents {
            if !document.dirty() {
                if let Ok(text) = self.repository.read(&document.path) {
                    document.saved = text.clone();
                    document.text = text;
                }
            }
        }
    }

    fn close_document(&mut self, index: usize) {
        self.documents.remove(index);
        self.active = match self.active {
            _ if self.documents.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.documents.len() - 1)),
            active => active,
        };
    }

    fn request_close(&mut self, index: usize) {
        if self.documents[index].dirty() {
            self.pending = Some(Pending::Close(index));
        } else {
            self.close_document(index);
        }
    }

    fn reload_document(&mut self, index: usize) -> Result<(), String> {
        let text = self.repository.read(&self.documents[index].path)?;
        self.documents[index].text = text.clone();
        self.documents[index].saved = text;
        self.error = None;
        self.status = "Reloaded saved file".into();
        Ok(())
    }

    fn request_reload(&mut self) {
        if let Some(index) = self.active {
            if self.documents[index].dirty() {
                self.pending = Some(Pending::Reload(index));
            } else if let Err(error) = self.reload_document(index) {
                self.error = Some(error);
            }
        }
    }

    fn reveal(&mut self, path: &str, line: usize) {
        match self.open_file(path) {
            Ok(()) => {
                let document = &self.documents[self.active.unwrap()];
                let position = line_start(&document.text, line);
                self.selection = Some(position..position);
                self.reveal_line = Some(line);
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn find_next(&mut self) {
        if let Some(index) = self.active {
            let text = &self.documents[index].text;
            if let Some(range) = find_range(text, &self.find, self.find_after) {
                self.find_after = range.end;
                self.reveal_line =
                    Some(text[..range.start].bytes().filter(|b| *b == b'\n').count() + 1);
                self.selection =
                    Some(text[..range.start].chars().count()..text[..range.end].chars().count());
                self.status = "Match selected".into();
            } else {
                self.status = "No matching text in this file".into();
            }
        }
    }

    fn load_evidence(&mut self, history: bool, relative: Option<String>) {
        if self.evidence_task.is_some() {
            return;
        }
        let root = self.repository.root().to_owned();
        let title = if history {
            "Git history".to_string()
        } else {
            relative
                .as_ref()
                .map(|path| format!("Working diff · {path}"))
                .unwrap_or("Working diff · all files".into())
        };
        self.evidence = Some(Evidence {
            title,
            text: String::new(),
        });
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = Repository::open(&root).and_then(|repository| {
                if history {
                    repository.git_log()
                } else {
                    repository.git_diff(relative.as_deref())
                }
            });
            let _ = sender.send(result);
        });
        self.evidence_task = Some(receiver);
    }

    fn receive_evidence(&mut self) {
        let result = self
            .evidence_task
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = result {
            self.evidence_task = None;
            match result {
                Ok(text) => {
                    if let Some(evidence) = &mut self.evidence {
                        evidence.text = if text.trim().is_empty() {
                            "No working-tree changes. Saved repairs will appear here.".into()
                        } else {
                            text
                        };
                    }
                }
                Err(error) => {
                    if let Some(evidence) = &mut self.evidence {
                        evidence.text = "Git evidence is unavailable. Source editing and validation still work.".into();
                    }
                    self.error = Some(error);
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, read_only: bool) {
        self.read_only = read_only;
        self.receive_evidence();
        self.shortcuts(ui.ctx());
        egui::TopBottomPanel::bottom("workspace_status")
            .exact_height(26.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 3)),
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if read_only {
                            "OBSERVATION ONLY"
                        } else {
                            "LOCAL WORKSPACE"
                        })
                        .size(10.0)
                        .color(ACCENT),
                    );
                    ui.separator();
                    ui.label(RichText::new(&self.status).size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Ln {}, Col {}  ·  UTF-8",
                                self.cursor.0, self.cursor.1
                            ))
                            .size(11.0)
                            .color(MUTED),
                        );
                        if self.dirty() {
                            ui.label(
                                RichText::new("Unsaved repairs")
                                    .size(11.0)
                                    .color(Color32::LIGHT_YELLOW),
                            );
                        }
                    });
                });
            });
        egui::SidePanel::left("workspace_explorer")
            .default_width(235.0)
            .width_range(175.0..=360.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(10.0))
            .show_inside(ui, |ui| self.sidebar_ui(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BACKGROUND).inner_margin(0.0))
            .show_inside(ui, |ui| {
                self.tabs_ui(ui);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if ui.add_enabled(self.active.is_some() && !read_only, egui::Button::new("Save file").small()).on_hover_text("Ctrl/Cmd+S").clicked() {
                        if let Err(error) = self.save_active() { self.error = Some(error); }
                    }
                    if ui.add_enabled(self.active.is_some(), egui::Button::new("Find").small()).on_hover_text("Ctrl/Cmd+F").clicked() { self.show_find = !self.show_find; self.focus_find = self.show_find; }
                    if ui.add_enabled(self.active.is_some(), egui::Button::new("Reload file").small()).clicked() { self.request_reload(); }
                    if let Some(index) = self.active {
                        if ui.add_enabled(self.evidence_task.is_none(), egui::Button::new("View diff").small()).clicked() {
                            self.load_evidence(false, Some(self.documents[index].path.clone()));
                        }
                    }
                });
                if self.show_find && self.active.is_some() { self.find_ui(ui); }
                if self.error.is_some() {
                    egui::Frame::new().fill(Color32::from_rgb(55, 31, 34)).inner_margin(8.0).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(self.error.as_deref().unwrap_or_default()).color(Color32::LIGHT_RED));
                            if ui.small_button("Dismiss").clicked() { self.error = None; }
                        });
                    });
                }
                ui.separator();
                if self.evidence.is_some() { self.evidence_ui(ui); }
                else if let Some(index) = self.active { self.document_ui(ui, index, read_only); }
                else {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() * 0.25).max(24.0));
                        ui.label(RichText::new("INVESTIGATION WORKSPACE").size(12.0).color(ACCENT));
                        ui.add_space(10.0); ui.heading("Follow the evidence.");
                        ui.label(RichText::new("Open the briefing or choose a source file.").color(MUTED));
                        ui.add_space(16.0);
                        if ui.button("Open briefing").clicked() {
                            if let Err(error) = self.open_file("README.md") { self.error = Some(error); }
                        }
                        ui.add_space(12.0);
                        ui.label(RichText::new("Ctrl/Cmd+S  Save    ·    Ctrl/Cmd+F  Find    ·    Ctrl/Cmd+Shift+F  Search").size(11.0).color(MUTED));
                    });
                }
            });
        self.pending_dialog(ui.ctx());
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || ctx.memory(|memory| memory.top_modal_layer().is_some()) {
            return;
        }
        let command = egui::Modifiers::COMMAND;
        let shifted = egui::Modifiers {
            shift: true,
            ..command
        };
        if ctx.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(shifted, egui::Key::F))
        }) {
            self.sidebar = Sidebar::Search;
            self.focus_search = true;
        } else if ctx.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::F))
        }) {
            self.show_find = true;
            self.focus_find = true;
        }
        if !self.read_only {
            let save_all = ctx.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(shifted, egui::Key::S))
            });
            let save = ctx.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::S))
            });
            let result = if save_all {
                self.save_all()
            } else if save {
                self.save_active()
            } else {
                Ok(())
            };
            if let Err(error) = result {
                self.error = Some(error);
            }
        }
        if ctx.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(command, egui::Key::W))
        }) {
            if self.evidence.is_some() {
                self.evidence = None;
            } else if let Some(index) = self.active {
                self.request_close(index);
            }
        }
    }

    fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.sidebar, Sidebar::Files, "Files");
            ui.selectable_value(&mut self.sidebar, Sidebar::Search, "Search");
            ui.selectable_value(&mut self.sidebar, Sidebar::Evidence, "Git");
        });
        ui.add_space(8.0);
        let root_name = self
            .repository
            .root()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        ui.label(
            RichText::new(root_name.to_uppercase())
                .size(10.0)
                .color(MUTED)
                .strong(),
        );
        ui.separator();
        match self.sidebar {
            Sidebar::Files => {
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("Filter files…")
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} files", self.repository.files().len()))
                            .small()
                            .color(MUTED),
                    );
                    if ui.small_button("Refresh").clicked() {
                        self.refresh();
                    }
                });
                let active_path = self.active.map(|index| self.documents[index].path.as_str());
                let mut selected = None;
                egui::ScrollArea::vertical()
                    .id_salt("file_tree")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.filter.is_empty() {
                            let files: Vec<&str> =
                                self.repository.files().iter().map(String::as_str).collect();
                            selected = file_tree(ui, "", &files, active_path);
                        } else {
                            let filter = self.filter.to_lowercase();
                            for path in self
                                .repository
                                .files()
                                .iter()
                                .filter(|path| path.to_lowercase().contains(&filter))
                            {
                                if ui
                                    .selectable_label(active_path == Some(path.as_str()), path)
                                    .clicked()
                                {
                                    selected = Some(path.clone());
                                }
                            }
                        }
                    });
                if let Some(path) = selected {
                    if let Err(error) = self.open_file(&path) {
                        self.error = Some(error);
                    }
                }
            }
            Sidebar::Search => {
                ui.label(RichText::new("Search saved source").small().color(MUTED));
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Text across all files…")
                        .desired_width(f32::INFINITY),
                );
                if self.focus_search {
                    input.request_focus();
                    self.focus_search = false;
                }
                let run =
                    input.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if ui
                    .add_enabled(
                        !self.query.is_empty(),
                        egui::Button::new("Search repository"),
                    )
                    .clicked()
                    || run
                {
                    self.hits = self.repository.search(&self.query);
                    self.searched = true;
                }
                if self.searched {
                    ui.label(
                        RichText::new(format!("{} matches (up to 200)", self.hits.len()))
                            .small()
                            .color(MUTED),
                    );
                }
                ui.label(
                    RichText::new("Search uses saved files. Save repairs to include them.")
                        .small()
                        .color(MUTED),
                );
                let mut chosen = None;
                egui::ScrollArea::vertical()
                    .id_salt("search_hits")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (index, hit) in self.hits.iter().enumerate() {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("{}:{}", hit.path, hit.line)).small(),
                                    )
                                    .wrap(),
                                )
                                .on_hover_text(&hit.text)
                                .clicked()
                            {
                                chosen = Some(index);
                            }
                            ui.add(
                                egui::Label::new(
                                    RichText::new(hit.text.trim()).size(11.0).color(MUTED),
                                )
                                .truncate(),
                            );
                            ui.add_space(4.0);
                        }
                    });
                if let Some(index) = chosen {
                    let hit = self.hits[index].clone();
                    self.reveal(&hit.path, hit.line);
                }
            }
            Sidebar::Evidence => {
                ui.label(RichText::new("Repository evidence").small().color(MUTED));
                ui.add_enabled_ui(self.evidence_task.is_none(), |ui| {
                    if ui.button("Working changes").clicked() {
                        self.load_evidence(false, None);
                    }
                    if ui.button("Recent commits").clicked() {
                        self.load_evidence(true, None);
                    }
                });
                ui.add_space(12.0);
                ui.label(RichText::new("Compare saved repairs with the current commit, or trace the repository's maintenance history.").small().color(MUTED));
                ui.add_space(8.0);
                ui.label(RichText::new("Git views only read evidence. No source, hooks, or project commands are run.").small().color(MUTED));
                if self.evidence_task.is_some() {
                    ui.spinner();
                }
            }
        }
    }

    fn tabs_ui(&mut self, ui: &mut egui::Ui) {
        let mut close = None;
        egui::Frame::new()
            .fill(PANEL)
            .inner_margin(egui::Margin::symmetric(5, 4))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("document_tabs")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, document) in self.documents.iter().enumerate() {
                                let name =
                                    document.path.rsplit('/').next().unwrap_or(&document.path);
                                let label = if document.dirty() {
                                    format!("{name}  •")
                                } else {
                                    name.to_owned()
                                };
                                if ui
                                    .selectable_label(
                                        self.active == Some(index) && self.evidence.is_none(),
                                        label,
                                    )
                                    .on_hover_text(&document.path)
                                    .clicked()
                                {
                                    self.active = Some(index);
                                    self.evidence = None;
                                    self.find_after = 0;
                                }
                                if ui
                                    .small_button("×")
                                    .on_hover_text("Close file · Ctrl/Cmd+W")
                                    .clicked()
                                {
                                    close = Some(index);
                                }
                                ui.separator();
                            }
                            if self.documents.is_empty() {
                                ui.label(RichText::new("No file open").color(MUTED));
                            }
                        });
                    });
            });
        if let Some(index) = close {
            self.request_close(index);
        }
    }

    fn find_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.find)
                    .hint_text("Find in file (case sensitive)…")
                    .desired_width(270.0),
            );
            if self.focus_find {
                input.request_focus();
                self.focus_find = false;
            }
            if input.changed() {
                self.find_after = 0;
            }
            if ui.small_button("Next").clicked()
                || (input.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
            {
                self.find_next();
            }
            if ui.small_button("×").clicked() {
                self.show_find = false;
            }
        });
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, index: usize, read_only: bool) {
        let font = FontId::monospace(self.font_size);
        let line_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let document = &mut self.documents[index];
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(&document.path).size(11.0).color(MUTED));
            if read_only {
                ui.label(RichText::new("READ ONLY").size(10.0).color(ACCENT));
            }
        });
        let available = ui.available_size();
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("source", &document.path))
            .auto_shrink([false, false]);
        if let Some(line) = self.reveal_line.take() {
            scroll = scroll.vertical_scroll_offset(line.saturating_sub(3) as f32 * line_height);
        }
        let mut selection = self.selection.take();
        let mut position = None;
        scroll.show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            ui.horizontal_top(|ui| {
                let line_count = document.text.bytes().filter(|byte| *byte == b'\n').count() + 1;
                let numbers = (1..=line_count)
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 12,
                        top: 4,
                        bottom: 4,
                    })
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(numbers)
                                    .font(font.clone())
                                    .color(Color32::from_rgb(85, 103, 126)),
                            )
                            .selectable(false),
                        );
                    });
                let font_size = self.font_size;
                let path = document.path.clone();
                let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
                    let job = highlight(buffer.as_str(), &path, font_size);
                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                };
                // Immutable TextBuffer keeps selection/copy available in agent
                // observation without allowing a text edit or a disk write.
                let copy = if read_only {
                    document.text.clone()
                } else {
                    String::new()
                };
                let mut immutable = copy.as_str();
                let buffer: &mut dyn egui::TextBuffer = if read_only {
                    &mut immutable
                } else {
                    &mut document.text
                };
                let mut output = egui::TextEdit::multiline(buffer)
                    .id_salt(("document", &path))
                    .code_editor()
                    .font(font.clone())
                    .frame(false)
                    .margin(egui::vec2(4.0, 4.0))
                    .desired_width((available.x - 65.0).max(180.0))
                    .desired_rows(1)
                    .min_size(egui::vec2(
                        (available.x - 65.0).max(180.0),
                        (available.y - 10.0).max(80.0),
                    ))
                    .layouter(&mut layouter)
                    .show(ui);
                if let Some(range) = selection.take() {
                    output
                        .state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(range.start),
                            egui::text::CCursor::new(range.end),
                        )));
                    output.state.store(ui.ctx(), output.response.id);
                    output.response.request_focus();
                }
                position = output.cursor_range.map(|range| range.primary.index);
            });
        });
        if let Some(position) = position {
            self.cursor = line_column(&document.text, position);
        }
    }

    fn evidence_ui(&mut self, ui: &mut egui::Ui) {
        let mut close = false;
        if let Some(evidence) = &self.evidence {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new(&evidence.title).strong());
                if self.evidence_task.is_some() {
                    ui.spinner();
                }
                if ui.small_button("Return to source").clicked() {
                    close = true;
                }
            });
            egui::ScrollArea::both()
                .id_salt("git_evidence")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if evidence.text.is_empty() {
                        ui.label(RichText::new("Reading local Git evidence…").color(MUTED));
                    }
                    for line in evidence.text.lines() {
                        let color = if line.starts_with('+') && !line.starts_with("+++") {
                            Color32::from_rgb(145, 199, 159)
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            Color32::from_rgb(226, 144, 144)
                        } else if line.starts_with("@@") {
                            ACCENT
                        } else {
                            TEXT
                        };
                        ui.add(
                            egui::Label::new(
                                RichText::new(line)
                                    .font(FontId::monospace(self.font_size))
                                    .color(color),
                            )
                            .selectable(true)
                            .extend(),
                        );
                    }
                });
        }
        if close {
            self.evidence = None;
        }
    }

    fn pending_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.pending else {
            return;
        };
        let (index, reload) = match *pending {
            Pending::Close(index) => (index, false),
            Pending::Reload(index) => (index, true),
        };
        let Some(document) = self.documents.get(index) else {
            self.pending = None;
            return;
        };
        let path = document.path.clone();
        let mut action = 0;
        egui::Modal::new(egui::Id::new("unsaved_source_file")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(if reload {
                "Discard unsaved repairs and reload?"
            } else {
                "Save repairs before closing?"
            });
            ui.add(egui::Label::new(RichText::new(path).monospace()).wrap());
            ui.label(if reload {
                "The saved file on disk will replace this editor buffer."
            } else {
                "This file has changes that have not been saved."
            });
            ui.horizontal(|ui| {
                if !reload
                    && ui
                        .add_enabled(!self.read_only, egui::Button::new("Save and close"))
                        .clicked()
                {
                    action = 1;
                }
                if ui
                    .button(if reload {
                        "Discard and reload"
                    } else {
                        "Discard changes"
                    })
                    .clicked()
                {
                    action = 2;
                }
                if ui.button("Keep editing").clicked() {
                    action = 3;
                }
            });
        });
        match action {
            1 => match self.save_document(index) {
                Ok(()) => {
                    self.close_document(index);
                    self.pending = None;
                }
                Err(error) => {
                    self.error = Some(error);
                    self.pending = None;
                }
            },
            2 => {
                if reload {
                    if let Err(error) = self.reload_document(index) {
                        self.error = Some(error);
                    }
                } else {
                    self.close_document(index);
                }
                self.pending = None;
            }
            3 => self.pending = None,
            _ => {}
        }
    }
}

fn file_tree(
    ui: &mut egui::Ui,
    prefix: &str,
    files: &[&str],
    active: Option<&str>,
) -> Option<String> {
    let mut directories: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut chosen = None;
    for file in files {
        let remainder = file.strip_prefix(prefix).unwrap_or(file);
        if let Some((directory, _)) = remainder.split_once('/') {
            directories.entry(directory).or_default().push(file);
        } else if ui
            .selectable_label(active == Some(*file), remainder)
            .on_hover_text(*file)
            .clicked()
        {
            chosen = Some((*file).to_owned());
        }
    }
    for (directory, files) in directories {
        let path = format!("{prefix}{directory}/");
        let open = path == "src/" || active.is_some_and(|active| active.starts_with(&path));
        egui::CollapsingHeader::new(directory)
            .id_salt(&path)
            .default_open(open)
            .show(ui, |ui| {
                if let Some(file) = file_tree(ui, &path, &files, active) {
                    chosen = Some(file);
                }
            });
    }
    chosen
}

fn line_start(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut number = 1;
    for (index, character) in text.chars().enumerate() {
        if character == '\n' {
            number += 1;
            if number == line {
                return index + 1;
            }
        }
    }
    text.chars().count()
}

fn line_column(text: &str, position: usize) -> (usize, usize) {
    text.chars()
        .take(position)
        .fold((1, 1), |(line, column), character| {
            if character == '\n' {
                (line + 1, 1)
            } else {
                (line, column + 1)
            }
        })
}

fn find_range(text: &str, query: &str, after: usize) -> Option<Range<usize>> {
    if query.is_empty() {
        return None;
    }
    let mut after = after.min(text.len());
    while !text.is_char_boundary(after) {
        after -= 1;
    }
    let start = text[after..]
        .find(query)
        .map(|start| after + start)
        .or_else(|| text.find(query))?;
    Some(start..start + query.len())
}

fn highlight(text: &str, path: &str, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let python = path.ends_with(".py")
        || path.ends_with(".sh")
        || path.ends_with(".yml")
        || path.ends_with(".yaml")
        || path.ends_with(".toml");
    let mut index = 0;
    while index < text.len() {
        let start = index;
        let character = text[index..].chars().next().unwrap();
        let rest = &text[index..];
        let mut color = TEXT;
        if rest.starts_with("//") || (python && character == '#') {
            index += rest.find('\n').unwrap_or(rest.len());
            color = MUTED;
        } else if rest.starts_with("/*") {
            index += rest[2..]
                .find("*/")
                .map(|end| end + 4)
                .unwrap_or(rest.len());
            color = MUTED;
        } else if character == '"' || character == '\'' || character == '`' {
            index += character.len_utf8();
            let mut escaped = false;
            for next in text[index..].chars() {
                index += next.len_utf8();
                if !escaped && next == character {
                    break;
                }
                escaped = !escaped && next == '\\';
            }
            color = Color32::from_rgb(157, 199, 154);
        } else if character.is_ascii_alphabetic() || character == '_' {
            index += character.len_utf8();
            while index < text.len() {
                let next = text[index..].chars().next().unwrap();
                if !next.is_ascii_alphanumeric() && next != '_' {
                    break;
                }
                index += next.len_utf8();
            }
            if matches!(
                &text[start..index],
                "import"
                    | "from"
                    | "class"
                    | "def"
                    | "return"
                    | "if"
                    | "else"
                    | "elif"
                    | "for"
                    | "while"
                    | "in"
                    | "not"
                    | "and"
                    | "or"
                    | "try"
                    | "except"
                    | "raise"
                    | "with"
                    | "as"
                    | "pass"
                    | "async"
                    | "await"
                    | "const"
                    | "let"
                    | "var"
                    | "function"
                    | "export"
                    | "default"
                    | "new"
                    | "throw"
                    | "catch"
                    | "finally"
                    | "switch"
                    | "case"
                    | "break"
                    | "continue"
                    | "package"
                    | "func"
                    | "type"
                    | "struct"
                    | "interface"
                    | "defer"
                    | "go"
                    | "range"
                    | "map"
                    | "using"
                    | "namespace"
                    | "public"
                    | "private"
                    | "protected"
                    | "internal"
                    | "static"
                    | "void"
                    | "string"
                    | "bool"
                    | "int"
                    | "record"
                    | "sealed"
                    | "override"
                    | "true"
                    | "false"
                    | "null"
                    | "True"
                    | "False"
                    | "None"
                    | "nil"
            ) {
                color = Color32::from_rgb(192, 169, 225);
            }
        } else if character.is_ascii_digit() {
            index += 1;
            while index < text.len()
                && (text.as_bytes()[index].is_ascii_alphanumeric()
                    || text.as_bytes()[index] == b'.'
                    || text.as_bytes()[index] == b'_')
            {
                index += 1;
            }
            color = Color32::from_rgb(224, 188, 134);
        } else {
            index += character.len_utf8();
        }
        // Coalesce equal adjacent styles. A large plain file should not create
        // one layout allocation for every space or punctuation character.
        if let Some(section) = job.sections.last_mut()
            && section.format.color == color
        {
            job.text.push_str(&text[start..index]);
            section.byte_range.end = job.text.len();
        } else {
            job.append(
                &text[start..index],
                0.0,
                egui::TextFormat {
                    font_id: FontId::monospace(size),
                    color,
                    ..Default::default()
                },
            );
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_preserves_repairs_on_external_conflict_and_guards_read_only_saves() {
        let root = std::env::temp_dir().join(format!(
            "sourcedefender-editor-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("source.py"), "original\n").unwrap();
        let mut editor = Editor::open(&root).unwrap();
        editor.open_file("source.py").unwrap();
        editor.documents[0].text = "repair\n".into();
        std::fs::write(root.join("source.py"), "external change\n").unwrap();
        editor.refresh();
        assert_eq!(editor.documents[0].text, "repair\n");
        assert!(editor.save_all().is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("source.py")).unwrap(),
            "external change\n"
        );
        assert!(editor.dirty());
        editor.request_close(0);
        assert!(matches!(editor.pending, Some(Pending::Close(0))));
        assert_eq!(editor.documents.len(), 1);
        editor.discard_all().unwrap();
        assert_eq!(editor.documents[0].text, "external change\n");
        assert!(!editor.dirty());
        editor.documents[0].text = "attempted edit\n".into();
        editor.set_read_only(true);
        assert!(editor.save_active().is_err());
        assert!(editor.save_all().is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("source.py")).unwrap(),
            "external change\n"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_highlighting_preserves_unicode_and_unclosed_tokens() {
        for source in [
            "const café = '東京'; // hello\n",
            "/* unfinished λ",
            "x = `\\` still open",
            "# λ\nprint(42)",
            "\"trailing\\",
        ] {
            let job = highlight(source, "sample.py", 14.0);
            assert_eq!(job.text, source);
            for section in job.sections {
                assert!(source.get(section.byte_range).is_some());
            }
        }
        assert_eq!(
            highlight(&" ".repeat(5000), "source.py", 14.0)
                .sections
                .len(),
            1
        );
    }

    #[test]
    fn find_wraps_and_recovers_from_an_edited_unicode_boundary() {
        let text = "écho α écho";
        assert_eq!(find_range(text, "écho", 0), Some(0..5));
        assert_eq!(find_range(text, "écho", 5), Some(9..14));
        assert_eq!(find_range(text, "écho", 14), Some(0..5));
        assert_eq!(find_range(text, "écho", 1), Some(0..5));
        assert_eq!(find_range(text, "", 0), None);
        assert_eq!(find_range(text, "missing", 0), None);
    }

    #[test]
    fn navigation_uses_character_positions_and_one_based_lines() {
        let text = "λ first\nsecond 東京\nlast";
        let position = line_start(text, 2);
        assert_eq!(position, 8);
        assert_eq!(line_column(text, position), (2, 1));
        assert_eq!(line_column(text, line_start(text, 3)), (3, 1));
        assert_eq!(line_start(text, 100), text.chars().count());
    }
}
