mod folders;
mod graph;
mod links;
mod markdown;
mod storage;

use eframe::egui;
use links::{LinkIndex, LinkResolution};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use storage::{AppData, AppSettings, Note, NoteSort, StoragePaths, ThemeChoice};
use uuid::Uuid;

const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
const EXTERNAL_SYNC_INTERVAL: Duration = Duration::from_secs(2);

struct WidgetApp {
    data: AppData,
    settings: AppSettings,
    storage_paths: StoragePaths,
    pending_delete_id: Option<Uuid>,
    dirty_note_ids: HashSet<Uuid>,
    pending_title_rename_ids: HashSet<Uuid>,
    last_edit_at: Option<Instant>,
    storage_message: Option<String>,
    link_index: LinkIndex,
    folder_paths: Vec<PathBuf>,
    graph_state: graph::GraphState,

    view: AppView,
    search_query: String,
    normalized_search_query: String,
    focus_search: bool,
    focus_editor: bool,
    show_new_folder_input: bool,
    new_folder_name: String,
    editing_folder: Option<PathBuf>,
    folder_name_buffer: String,
    graph_overlay_open: bool,
    vault_path_buffer: String,
    vault_snapshot: HashSet<(PathBuf, u128)>,
    last_external_sync: Instant,
    external_conflict: bool,
    window_settings_applied: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppView {
    Editor,
    NotesList,
    Graph,
    Trash,
    Settings,
}

fn show_note_dates(ui: &mut egui::Ui, note: &Note) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    let created_text = note
        .created_at
        .format("Created: %d/%m/%Y %H:%M")
        .to_string();
    let updated_text = note
        .updated_at
        .format("Updated: %d/%m/%Y %H:%M")
        .to_string();

    ui.small(created_text);
    ui.small(updated_text);
}

#[derive(Default)]
struct NotesListActions {
    selected_note_id: Option<Uuid>,
    requested_delete_id: Option<Uuid>,
    selected_folder: Option<PathBuf>,
    toggled_folder: Option<PathBuf>,
    toggled_pin_id: Option<Uuid>,
    rename_folder: Option<PathBuf>,
    delete_folder: Option<PathBuf>,
}

fn folder_has_visible_notes(
    folder: &folders::FolderNode,
    notes: &HashMap<Uuid, &Note>,
    normalized_query: &str,
) -> bool {
    normalized_query.is_empty()
        || folder.note_ids.iter().any(|id| {
            notes
                .get(id)
                .is_some_and(|note| note.search_text.contains(normalized_query))
        })
        || folder
            .folders
            .iter()
            .any(|child| folder_has_visible_notes(child, notes, normalized_query))
}

fn show_note_row(
    ui: &mut egui::Ui,
    note: &Note,
    selected_note_id: Option<Uuid>,
    actions: &mut NotesListActions,
) {
    let display_title = if note.title.trim().is_empty() {
        note.content
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("Untitled")
    } else {
        note.title.as_str()
    };
    let updated_text = note.updated_at.format("%d/%m %H:%M").to_string();

    ui.horizontal(|ui| {
        if note.pinned {
            ui.label("*").on_hover_text("Pinned");
        }
        let response = ui
            .selectable_label(selected_note_id == Some(note.id), display_title)
            .on_hover_text(note.file_path.display().to_string());
        if response.clicked() {
            actions.selected_note_id = Some(note.id);
        }
        response.context_menu(|ui| {
            if ui
                .button(if note.pinned { "Unpin" } else { "Pin" })
                .clicked()
            {
                actions.toggled_pin_id = Some(note.id);
                ui.close();
            }
            if ui.button("Move to Trash").clicked() {
                actions.requested_delete_id = Some(note.id);
                ui.close();
            }
        });
        ui.small(updated_text);
    });
    ui.add_space(2.0);
}

#[allow(clippy::too_many_arguments)]
fn show_folder_node(
    ui: &mut egui::Ui,
    folder: &folders::FolderNode,
    notes: &HashMap<Uuid, &Note>,
    normalized_query: &str,
    selected_note_id: Option<Uuid>,
    selected_folder: &Path,
    collapsed_folders: &[PathBuf],
    note_sort: NoteSort,
    actions: &mut NotesListActions,
) {
    if !folder_has_visible_notes(folder, notes, normalized_query) {
        return;
    }

    let collapsed = collapsed_folders.contains(&folder.relative_path);
    ui.horizontal(|ui| {
        if ui.small_button(if collapsed { ">" } else { "v" }).clicked() {
            actions.toggled_folder = Some(folder.relative_path.clone());
        }
        let response = ui
            .selectable_label(selected_folder == folder.relative_path, &folder.name)
            .on_hover_text(folder.relative_path.display().to_string());
        if response.clicked() {
            actions.selected_folder = Some(folder.relative_path.clone());
        }
        if !folder.relative_path.as_os_str().is_empty() {
            response.context_menu(|ui| {
                if ui.button("Rename folder").clicked() {
                    actions.rename_folder = Some(folder.relative_path.clone());
                    ui.close();
                }
                if ui.button("Delete empty folder").clicked() {
                    actions.delete_folder = Some(folder.relative_path.clone());
                    ui.close();
                }
            });
        }
    });

    if collapsed {
        return;
    }

    ui.indent(("folder", &folder.relative_path), |ui| {
        for child in &folder.folders {
            show_folder_node(
                ui,
                child,
                notes,
                normalized_query,
                selected_note_id,
                selected_folder,
                collapsed_folders,
                note_sort,
                actions,
            );
        }

        let mut note_ids = folder.note_ids.clone();
        note_ids.sort_by(|left, right| {
            let left = notes.get(left).expect("folder note exists");
            let right = notes.get(right).expect("folder note exists");
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| match note_sort {
                    NoteSort::Updated => right.updated_at.cmp(&left.updated_at),
                    NoteSort::Created => right.created_at.cmp(&left.created_at),
                    NoteSort::Title => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
                })
        });
        for note_id in note_ids {
            let Some(note) = notes.get(&note_id) else {
                continue;
            };
            if !note.pinned
                && (normalized_query.is_empty() || note.search_text.contains(normalized_query))
            {
                show_note_row(ui, note, selected_note_id, actions);
            }
        }
    });
}

fn shortcut_pressed(ctx: &egui::Context, shortcut: &str) -> bool {
    let parts = shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let Some(key_name) = parts.last() else {
        return false;
    };
    let key = match key_name.to_ascii_uppercase().as_str() {
        "A" => egui::Key::A,
        "B" => egui::Key::B,
        "C" => egui::Key::C,
        "D" => egui::Key::D,
        "E" => egui::Key::E,
        "F" => egui::Key::F,
        "G" => egui::Key::G,
        "H" => egui::Key::H,
        "I" => egui::Key::I,
        "J" => egui::Key::J,
        "K" => egui::Key::K,
        "L" => egui::Key::L,
        "M" => egui::Key::M,
        "N" => egui::Key::N,
        "O" => egui::Key::O,
        "P" => egui::Key::P,
        "Q" => egui::Key::Q,
        "R" => egui::Key::R,
        "S" => egui::Key::S,
        "T" => egui::Key::T,
        "U" => egui::Key::U,
        "V" => egui::Key::V,
        "W" => egui::Key::W,
        "X" => egui::Key::X,
        "Y" => egui::Key::Y,
        "Z" => egui::Key::Z,
        _ => return false,
    };
    ctx.input(|input| {
        let wants_ctrl = parts.iter().any(|part| part.eq_ignore_ascii_case("ctrl"));
        let wants_shift = parts.iter().any(|part| part.eq_ignore_ascii_case("shift"));
        let wants_alt = parts.iter().any(|part| part.eq_ignore_ascii_case("alt"));
        input.modifiers.ctrl == wants_ctrl
            && input.modifiers.shift == wants_shift
            && input.modifiers.alt == wants_alt
            && input.key_pressed(key)
    })
}

fn shortcut_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
    });
}

impl WidgetApp {
    fn new() -> Self {
        let loaded = storage::load_storage().expect("Failed to initialize Markdown storage");
        let link_index = LinkIndex::build(&loaded.data.notes, &loaded.paths.notes_dir);

        for warning in &loaded.warnings {
            eprintln!("Storage warning: {warning}");
        }

        let storage_message = if loaded.migrated_notes > 0 {
            Some(format!(
                "Migrated {} note(s) to Markdown",
                loaded.migrated_notes
            ))
        } else {
            loaded.warnings.first().cloned()
        };

        let vault_path_buffer = loaded.settings.vault_path.display().to_string();
        let graph_state = graph::GraphState::restore(&loaded.settings.graph_node_offsets);
        let vault_snapshot = storage::vault_snapshot(&loaded.paths.notes_dir).unwrap_or_default();
        #[cfg(target_os = "windows")]
        let _ = set_autostart(loaded.settings.autostart);
        Self {
            data: loaded.data,
            settings: loaded.settings,
            storage_paths: loaded.paths,
            pending_delete_id: None,
            dirty_note_ids: HashSet::new(),
            pending_title_rename_ids: HashSet::new(),
            last_edit_at: None,
            storage_message,
            link_index,
            folder_paths: loaded.folder_paths,
            graph_state,
            view: AppView::Editor,
            search_query: String::new(),
            normalized_search_query: String::new(),
            focus_search: false,
            focus_editor: false,
            show_new_folder_input: false,
            new_folder_name: String::new(),
            editing_folder: None,
            folder_name_buffer: String::new(),
            graph_overlay_open: false,
            vault_path_buffer,
            vault_snapshot,
            last_external_sync: Instant::now(),
            external_conflict: false,
            window_settings_applied: false,
        }
    }

    fn save_settings(&mut self) {
        self.settings.selected_note_id = self.data.selected_note_id;
        if let Err(error) =
            storage::save_settings(&self.storage_paths.settings_path, &self.settings)
        {
            self.storage_message = Some(format!("Failed to save settings: {error}"));
        }
    }

    fn save_note_now(&mut self, id: Uuid) -> bool {
        let result = self
            .data
            .notes
            .iter()
            .find(|note| note.id == id)
            .map(|note| {
                if self.settings.backups_enabled {
                    storage::save_note_with_backup(
                        note,
                        &self.storage_paths.backups_dir,
                        self.settings.backup_limit,
                    )
                } else {
                    storage::save_note(note)
                }
            });

        match result {
            Some(Ok(())) => {
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                true
            }
            Some(Err(error)) => {
                self.storage_message = Some(format!("Failed to save note: {error}"));
                false
            }
            None => false,
        }
    }

    fn mark_note_dirty(&mut self, id: Uuid) {
        self.dirty_note_ids.insert(id);
        self.last_edit_at = Some(Instant::now());
    }

    fn flush_dirty_notes(&mut self) {
        let ids: Vec<Uuid> = self.dirty_note_ids.iter().copied().collect();
        for id in ids {
            if self.save_note_now(id) {
                if self.pending_title_rename_ids.remove(&id) {
                    let rename_result = self
                        .data
                        .notes
                        .iter_mut()
                        .find(|note| note.id == id)
                        .map(storage::rename_note_file);
                    if let Some(Err(error)) = rename_result {
                        self.storage_message = Some(format!("Failed to rename note file: {error}"));
                    } else {
                        self.vault_snapshot =
                            storage::vault_snapshot(&self.storage_paths.notes_dir)
                                .unwrap_or_default();
                    }
                }
                self.dirty_note_ids.remove(&id);
            }
        }

        if self.dirty_note_ids.is_empty() {
            self.last_edit_at = None;
        }
        self.save_settings();
    }

    fn save_after_debounce(&mut self, ctx: &egui::Context) {
        if self.external_conflict {
            return;
        }
        let Some(last_edit_at) = self.last_edit_at else {
            return;
        };
        let elapsed = last_edit_at.elapsed();

        if elapsed >= SAVE_DEBOUNCE {
            self.flush_dirty_notes();
        } else {
            ctx.request_repaint_after(SAVE_DEBOUNCE - elapsed);
        }
    }

    fn create_note(&mut self) {
        let note_directory = match storage::ensure_note_folder(
            &self.storage_paths.notes_dir,
            &self.settings.selected_folder,
        ) {
            Ok(directory) => directory,
            Err(error) => {
                self.storage_message = Some(format!("Failed to open note folder: {error}"));
                return;
            }
        };
        let id = self.data.create_note(&note_directory);
        self.pending_delete_id = None;
        self.view = AppView::Editor;
        self.focus_search = false;
        self.focus_editor = true;
        self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
        self.save_note_now(id);
        self.save_settings();
    }

    fn create_folder_from_input(&mut self) {
        match storage::create_note_folder(
            &self.storage_paths.notes_dir,
            &self.settings.selected_folder,
            &self.new_folder_name,
        ) {
            Ok(relative_path) => {
                if !self.folder_paths.contains(&relative_path) {
                    self.folder_paths.push(relative_path.clone());
                    self.folder_paths.sort();
                }
                self.settings.selected_folder = relative_path;
                self.new_folder_name.clear();
                self.show_new_folder_input = false;
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.save_settings();
            }
            Err(error) => {
                self.storage_message = Some(format!("Failed to create folder: {error}"));
            }
        }
    }

    fn move_selected_note_to_selected_folder(&mut self) {
        let Some(note_id) = self.data.selected_note_id else {
            return;
        };
        if !self.save_note_now(note_id) {
            return;
        }

        let Some(note) = self.data.notes.iter_mut().find(|note| note.id == note_id) else {
            return;
        };
        match storage::move_note_to_folder(
            note,
            &self.storage_paths,
            &self.settings.selected_folder,
        ) {
            Ok(()) => {
                self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.save_settings();
            }
            Err(error) => {
                self.storage_message = Some(format!("Failed to move note: {error}"));
            }
        }
    }

    fn toggle_pin(&mut self, id: Uuid) {
        if let Some(note) = self.data.notes.iter_mut().find(|note| note.id == id) {
            note.pinned = !note.pinned;
            note.mark_as_updated();
            self.mark_note_dirty(id);
        }
    }

    fn rename_selected_folder(&mut self) {
        let Some(source) = self.editing_folder.clone() else {
            return;
        };
        match storage::rename_folder(
            &self.storage_paths.notes_dir,
            &source,
            &self.folder_name_buffer,
        ) {
            Ok(destination) => {
                for note in &mut self.data.notes {
                    if let Ok(relative) = note.file_path.strip_prefix(&self.storage_paths.notes_dir)
                        && relative.starts_with(&source)
                        && let Ok(suffix) = relative.strip_prefix(&source)
                    {
                        note.file_path =
                            self.storage_paths.notes_dir.join(&destination).join(suffix);
                    }
                }
                for folder in &mut self.folder_paths {
                    if folder.starts_with(&source)
                        && let Ok(suffix) = folder.strip_prefix(&source)
                    {
                        *folder = destination.join(suffix);
                    }
                }
                for folder in &mut self.settings.collapsed_folders {
                    if folder.starts_with(&source)
                        && let Ok(suffix) = folder.strip_prefix(&source)
                    {
                        *folder = destination.join(suffix);
                    }
                }
                self.folder_paths.sort();
                self.settings.selected_folder = destination;
                self.editing_folder = None;
                self.folder_name_buffer.clear();
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
                self.save_settings();
            }
            Err(error) => self.storage_message = Some(format!("Failed to rename folder: {error}")),
        }
    }

    fn delete_folder(&mut self, path: &Path) {
        match storage::delete_empty_folder(&self.storage_paths.notes_dir, path) {
            Ok(()) => {
                self.folder_paths.retain(|folder| folder != path);
                self.settings
                    .collapsed_folders
                    .retain(|folder| folder != path);
                if self.settings.selected_folder == path {
                    self.settings.selected_folder = PathBuf::new();
                }
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.save_settings();
            }
            Err(error) => self.storage_message = Some(format!("Failed to delete folder: {error}")),
        }
    }

    fn reload_vault(&mut self, reason: &str) {
        match storage::reload_notes(&self.storage_paths) {
            Ok((notes, warnings, folders)) => {
                let selected = self.data.selected_note_id;
                self.data.notes = notes;
                self.data.selected_note_id =
                    selected.filter(|id| self.data.notes.iter().any(|note| note.id == *id));
                if self.data.selected_note_id.is_none() {
                    self.data.selected_note_id = self.data.notes.first().map(|note| note.id);
                }
                self.folder_paths = folders;
                self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.storage_message = warnings
                    .first()
                    .cloned()
                    .or_else(|| Some(reason.to_owned()));
                self.external_conflict = false;
                self.save_settings();
            }
            Err(error) => self.storage_message = Some(format!("Failed to reload vault: {error}")),
        }
    }

    fn sync_external_changes(&mut self, ctx: &egui::Context) {
        if self.last_external_sync.elapsed() < EXTERNAL_SYNC_INTERVAL {
            ctx.request_repaint_after(EXTERNAL_SYNC_INTERVAL - self.last_external_sync.elapsed());
            return;
        }
        self.last_external_sync = Instant::now();
        let current = storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
        if current != self.vault_snapshot {
            if self.dirty_note_ids.is_empty() {
                self.reload_vault("Reloaded changes from disk");
            } else {
                self.external_conflict = true;
                self.storage_message = Some(
                    "Files changed outside Lilo. Save or discard local edits, then reload."
                        .to_owned(),
                );
            }
        }
        ctx.request_repaint_after(EXTERNAL_SYNC_INTERVAL);
    }

    fn show_trash(&mut self, ui: &mut egui::Ui) {
        ui.heading("Trash");
        ui.small("Deleted notes can be restored to their original folder.");
        ui.separator();
        match storage::list_trash(&self.storage_paths) {
            Ok(entries) if entries.is_empty() => {
                ui.add_space(20.0);
                ui.label("Trash is empty");
            }
            Ok(entries) => {
                let mut restore = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in entries {
                        ui.horizontal(|ui| {
                            ui.label(&entry.display_name);
                            if ui.small_button("Restore").clicked() {
                                restore = Some(entry.relative_path.clone());
                            }
                        });
                    }
                });
                if let Some(relative) = restore {
                    match storage::restore_from_trash(&self.storage_paths, &relative) {
                        Ok(_) => self.reload_vault("Note restored"),
                        Err(error) => {
                            self.storage_message = Some(format!("Restore failed: {error}"))
                        }
                    }
                }
            }
            Err(error) => {
                ui.label(format!("Could not read Trash: {error}"));
            }
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Settings");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Appearance");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.settings.theme, ThemeChoice::Dark, "Dark");
                ui.selectable_value(&mut self.settings.theme, ThemeChoice::Light, "Light");
                ui.selectable_value(&mut self.settings.theme, ThemeChoice::System, "System");
            });
            ui.add(
                egui::Slider::new(&mut self.settings.font_size, 12.0..=22.0).text("Editor font"),
            );
            ui.horizontal(|ui| {
                ui.label("Accent");
                for value in &mut self.settings.accent_rgb {
                    ui.add(egui::DragValue::new(value).range(0..=255));
                }
            });
            if ui
                .checkbox(&mut self.settings.always_on_top, "Always on top")
                .changed()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    if self.settings.always_on_top {
                        egui::viewport::WindowLevel::AlwaysOnTop
                    } else {
                        egui::viewport::WindowLevel::Normal
                    },
                ));
            }
            if ui
                .checkbox(&mut self.settings.autostart, "Start Lilo with Windows")
                .changed()
                && let Err(error) = set_autostart(self.settings.autostart)
            {
                self.storage_message = Some(format!("Autostart update failed: {error}"));
            }

            ui.separator();
            ui.label("Storage");
            ui.text_edit_singleline(&mut self.vault_path_buffer);
            if ui.button("Use this vault after restart").clicked() {
                match storage::set_vault_path(&mut self.settings, &self.vault_path_buffer) {
                    Ok(()) => {
                        self.storage_message =
                            Some("Vault path saved. Restart Lilo to switch vaults.".to_owned())
                    }
                    Err(error) => {
                        self.storage_message = Some(format!("Invalid vault path: {error}"))
                    }
                }
            }
            ui.checkbox(
                &mut self.settings.backups_enabled,
                "Create backups before overwriting notes",
            );
            ui.add(
                egui::Slider::new(&mut self.settings.backup_limit, 1..=100)
                    .text("Backups per note"),
            );

            ui.separator();
            ui.label("Shortcuts (Ctrl/Shift/Alt + A-Z)");
            shortcut_field(ui, "New note", &mut self.settings.shortcuts.new_note);
            shortcut_field(ui, "Search", &mut self.settings.shortcuts.search);
            shortcut_field(ui, "Graph", &mut self.settings.shortcuts.graph);
            shortcut_field(
                ui,
                "Graph overlay",
                &mut self.settings.shortcuts.graph_overlay,
            );
            shortcut_field(ui, "Save", &mut self.settings.shortcuts.save);

            if ui.button("Save settings").clicked() {
                self.save_settings();
                self.storage_message = Some("Settings saved".to_owned());
            }
            if let Some(message) = &self.storage_message {
                ui.small(message);
            }
        });
    }

    fn show_notes_list(&mut self, ui: &mut egui::Ui) {
        let mut create_note_clicked = false;
        let mut submit_new_folder = false;
        let mut move_current_note = false;

        ui.horizontal(|ui| {
            ui.heading("Notes");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+ Folder").clicked() {
                    self.show_new_folder_input = !self.show_new_folder_input;
                    self.new_folder_name.clear();
                }
                if ui.small_button("+ Note").clicked() {
                    create_note_clicked = true;
                }
            });
        });

        let selected_folder_text = if self.settings.selected_folder.as_os_str().is_empty() {
            "Notes (root)".to_owned()
        } else {
            format!("Notes / {}", self.settings.selected_folder.display())
        };
        ui.small(selected_folder_text);

        let previous_sort = self.settings.note_sort;
        egui::ComboBox::from_id_salt("note_sort")
            .selected_text(match self.settings.note_sort {
                NoteSort::Updated => "Recently updated",
                NoteSort::Created => "Recently created",
                NoteSort::Title => "Title",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.settings.note_sort,
                    NoteSort::Updated,
                    "Recently updated",
                );
                ui.selectable_value(
                    &mut self.settings.note_sort,
                    NoteSort::Created,
                    "Recently created",
                );
                ui.selectable_value(&mut self.settings.note_sort, NoteSort::Title, "Title");
            });
        if self.settings.note_sort != previous_sort {
            self.save_settings();
        }

        let current_note_is_elsewhere = self.data.selected_note().is_some_and(|note| {
            note.file_path
                .parent()
                .and_then(|parent| parent.strip_prefix(&self.storage_paths.notes_dir).ok())
                != Some(self.settings.selected_folder.as_path())
        });
        if current_note_is_elsewhere && ui.small_button("Move current note here").clicked() {
            move_current_note = true;
        }

        if self.show_new_folder_input {
            ui.horizontal(|ui| {
                let input_width = (ui.available_width() - 58.0).max(40.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.new_folder_name)
                        .desired_width(input_width)
                        .hint_text("Folder name..."),
                );
                let enter_pressed =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if ui.small_button("Create").clicked() || enter_pressed {
                    submit_new_folder = true;
                }
            });
        }

        ui.add_space(4.0);
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .desired_width(f32::INFINITY)
                .hint_text("Search notes..."),
        );
        if self.focus_search {
            search_response.request_focus();
            self.focus_search = false;
        }
        if search_response.changed() {
            self.normalized_search_query = self.search_query.trim().to_lowercase();
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let actions = {
            let tree = folders::FolderTree::build(
                &self.data.notes,
                &self.storage_paths.notes_dir,
                &self.folder_paths,
            );
            let notes: HashMap<Uuid, &Note> =
                self.data.notes.iter().map(|note| (note.id, note)).collect();
            let mut actions = NotesListActions::default();

            let list_height = (ui.available_height() - 64.0).max(80.0);
            egui::ScrollArea::vertical()
                .max_height(list_height)
                .show(ui, |ui| {
                    let mut pinned = notes
                        .values()
                        .filter(|note| note.pinned)
                        .copied()
                        .collect::<Vec<_>>();
                    pinned.sort_by_key(|note| std::cmp::Reverse(note.updated_at));
                    if !pinned.is_empty() {
                        ui.strong("Pinned");
                        for note in pinned {
                            if self.normalized_search_query.is_empty()
                                || note.search_text.contains(&self.normalized_search_query)
                            {
                                show_note_row(ui, note, self.data.selected_note_id, &mut actions);
                            }
                        }
                        ui.separator();
                        ui.strong("Folders and recent notes");
                    }
                    if !folder_has_visible_notes(&tree.root, &notes, &self.normalized_search_query)
                    {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label("No notes found");
                        });
                    } else {
                        show_folder_node(
                            ui,
                            &tree.root,
                            &notes,
                            &self.normalized_search_query,
                            self.data.selected_note_id,
                            &self.settings.selected_folder,
                            &self.settings.collapsed_folders,
                            self.settings.note_sort,
                            &mut actions,
                        );
                    }
                });
            actions
        };

        if let Some(path) = actions.toggled_folder {
            if let Some(index) = self
                .settings
                .collapsed_folders
                .iter()
                .position(|collapsed| collapsed == &path)
            {
                self.settings.collapsed_folders.remove(index);
            } else {
                self.settings.collapsed_folders.push(path);
            }
            self.save_settings();
        }

        if let Some(path) = actions.selected_folder {
            self.settings.selected_folder = path;
            self.pending_delete_id = None;
            self.save_settings();
        }
        if let Some(id) = actions.requested_delete_id {
            self.pending_delete_id = Some(id);
        }
        if let Some(id) = actions.toggled_pin_id {
            self.toggle_pin(id);
        }
        if let Some(path) = actions.rename_folder {
            self.folder_name_buffer = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            self.editing_folder = Some(path);
        }
        if let Some(path) = actions.delete_folder {
            self.delete_folder(&path);
        }

        if self.editing_folder.is_some() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("New folder name:");
                ui.text_edit_singleline(&mut self.folder_name_buffer);
                if ui.button("Rename").clicked() {
                    self.rename_selected_folder();
                }
                if ui.button("Cancel").clicked() {
                    self.editing_folder = None;
                }
            });
        }

        if let Some(id) = self.pending_delete_id {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Move this note to Trash?");
                if ui.button("Yes").clicked() {
                    self.delete_note(id);
                }
                if ui.button("No").clicked() {
                    self.pending_delete_id = None;
                }
            });
        }

        if submit_new_folder {
            self.create_folder_from_input();
        }
        if move_current_note {
            self.move_selected_note_to_selected_folder();
        }
        if create_note_clicked {
            self.create_note();
        }
        if let Some(id) = actions.selected_note_id {
            self.open_note(id);
        }

        if let Some(message) = &self.storage_message {
            ui.small(message);
        }
    }

    fn open_note(&mut self, id: Uuid) {
        if !self.data.notes.iter().any(|note| note.id == id) {
            return;
        }

        self.data.selected_note_id = Some(id);
        self.pending_delete_id = None;
        self.view = AppView::Editor;
        self.focus_search = false;
        self.focus_editor = true;
        self.save_settings();
    }

    fn create_note_from_link(&mut self, title: &str) {
        let Some((explicit_folder, note_title)) = links::split_target_path(title) else {
            self.storage_message = Some(format!("Cannot create note from unsafe link [[{title}]]"));
            return;
        };

        let current_folder = self
            .data
            .selected_note()
            .and_then(|note| note.file_path.parent())
            .and_then(|parent| parent.strip_prefix(&self.storage_paths.notes_dir).ok())
            .map_or_else(PathBuf::new, Path::to_path_buf);
        let target_folder = if title.contains(['/', '\\']) {
            explicit_folder
        } else {
            current_folder
        };
        let note_directory =
            match storage::ensure_note_folder(&self.storage_paths.notes_dir, &target_folder) {
                Ok(directory) => directory,
                Err(error) => {
                    self.storage_message = Some(format!("Failed to create linked note: {error}"));
                    return;
                }
            };

        // Update the tree without rescanning the vault.
        let mut ancestor = PathBuf::new();
        for component in target_folder.components() {
            ancestor.push(component.as_os_str());
            if !self.folder_paths.contains(&ancestor) {
                self.folder_paths.push(ancestor.clone());
            }
        }
        self.folder_paths.sort();
        self.settings.selected_folder = target_folder;

        let id = self.data.create_note_named(&note_directory, &note_title);
        self.pending_delete_id = None;
        self.view = AppView::Editor;
        self.focus_search = false;
        self.focus_editor = true;
        self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
        self.save_note_now(id);
        self.save_settings();
    }

    fn delete_note(&mut self, id: Uuid) {
        let move_result = self
            .data
            .notes
            .iter()
            .find(|note| note.id == id)
            .map(|note| storage::move_note_to_trash(note, &self.storage_paths));

        match move_result {
            Some(Ok(())) => {
                self.data.remove_note(id);
                self.link_index = LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
                self.dirty_note_ids.remove(&id);
                self.pending_delete_id = None;
                self.vault_snapshot =
                    storage::vault_snapshot(&self.storage_paths.notes_dir).unwrap_or_default();
                self.save_settings();
            }
            Some(Err(error)) => {
                self.storage_message = Some(format!("Failed to move note to Trash: {error}"));
            }
            None => {
                self.pending_delete_id = None;
            }
        }
    }

    fn handle_graph_output(&mut self, output: graph::GraphOutput) -> bool {
        if output.persist_layout {
            self.settings.graph_node_offsets = self.graph_state.persisted_offsets();
            self.save_settings();
        }
        let _graph_interacted = output.state_changed;
        if let Some(id) = output.opened_note_id {
            self.open_note(id);
            return true;
        }
        if let Some(target) = output.create_missing_target {
            self.create_note_from_link(&target);
            return true;
        }
        false
    }
}

impl eframe::App for WidgetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.window_settings_applied {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                if self.settings.always_on_top {
                    egui::viewport::WindowLevel::AlwaysOnTop
                } else {
                    egui::viewport::WindowLevel::Normal
                },
            ));
            self.window_settings_applied = true;
        }

        let mut visuals = match self.settings.theme {
            ThemeChoice::Light => egui::Visuals::light(),
            ThemeChoice::Dark => egui::Visuals::dark(),
            ThemeChoice::System => match ctx.system_theme() {
                Some(egui::Theme::Light) => egui::Visuals::light(),
                _ => egui::Visuals::dark(),
            },
        };
        let accent = egui::Color32::from_rgb(
            self.settings.accent_rgb[0],
            self.settings.accent_rgb[1],
            self.settings.accent_rgb[2],
        );
        visuals.hyperlink_color = accent;
        visuals.selection.bg_fill = accent.gamma_multiply(0.55);
        ctx.set_visuals(visuals);

        let create_note_shortcut = shortcut_pressed(&ctx, &self.settings.shortcuts.new_note);
        let open_search_shortcut = shortcut_pressed(&ctx, &self.settings.shortcuts.search);
        let toggle_graph_shortcut = shortcut_pressed(&ctx, &self.settings.shortcuts.graph);
        let toggle_overlay_shortcut =
            shortcut_pressed(&ctx, &self.settings.shortcuts.graph_overlay);
        let save_shortcut = shortcut_pressed(&ctx, &self.settings.shortcuts.save);
        let escape_pressed = ctx.input(|input| input.key_pressed(egui::Key::Escape));

        if create_note_shortcut {
            self.create_note();
        }
        if open_search_shortcut {
            self.view = AppView::NotesList;
            self.focus_search = true;
            self.focus_editor = false;
            self.pending_delete_id = None;
        }
        if toggle_graph_shortcut {
            self.view = if self.view == AppView::Graph {
                AppView::Editor
            } else {
                AppView::Graph
            };
            self.pending_delete_id = None;
            self.focus_search = false;
            self.focus_editor = self.view == AppView::Editor;
        }
        if toggle_overlay_shortcut {
            self.graph_overlay_open = !self.graph_overlay_open;
        }
        if save_shortcut && !self.external_conflict {
            self.flush_dirty_notes();
        }
        if escape_pressed {
            if self.graph_overlay_open {
                self.graph_overlay_open = false;
            } else if self.pending_delete_id.is_some() {
                self.pending_delete_id = None;
            } else if self.view != AppView::Editor {
                self.view = AppView::Editor;
                self.focus_search = false;
                self.focus_editor = true;
            }
        }

        if self.view != AppView::NotesList {
            self.focus_search = false;
        }
        if self.view != AppView::Editor {
            self.focus_editor = false;
        }

        let current_title = self
            .data
            .selected_note()
            .map(|note| {
                if note.title.trim().is_empty() {
                    "Untitled"
                } else {
                    note.title.as_str()
                }
            })
            .unwrap_or("NOTES")
            .to_owned();

        egui::Panel::top("title_bar")
            .exact_size(36.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.view == AppView::Editor, "E")
                        .on_hover_text("Editor (Escape)")
                        .clicked()
                    {
                        self.view = AppView::Editor;
                        self.focus_search = false;
                        self.focus_editor = true;
                        self.pending_delete_id = None;
                    }
                    if ui
                        .selectable_label(self.view == AppView::NotesList, "L")
                        .on_hover_text("Notes list (Ctrl+P or Ctrl+F)")
                        .clicked()
                    {
                        self.view = AppView::NotesList;
                        self.focus_search = true;
                        self.focus_editor = false;
                        self.pending_delete_id = None;
                    }
                    if ui
                        .selectable_label(self.view == AppView::Graph, "G")
                        .on_hover_text("Graph (Ctrl+G)")
                        .clicked()
                    {
                        self.view = AppView::Graph;
                        self.focus_search = false;
                        self.focus_editor = false;
                        self.pending_delete_id = None;
                    }
                    if ui
                        .selectable_label(self.view == AppView::Trash, "T")
                        .on_hover_text("Trash")
                        .clicked()
                    {
                        self.view = AppView::Trash;
                        self.pending_delete_id = None;
                    }
                    if ui
                        .selectable_label(self.view == AppView::Settings, "S")
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        self.view = AppView::Settings;
                        self.pending_delete_id = None;
                    }

                    // Prevent egui's negative-size panic in narrow windows.
                    let drag_width = (ui.available_width() - 32.0).max(0.0);
                    let drag_area = ui.allocate_response(
                        egui::vec2(drag_width, 30.0),
                        egui::Sense::click_and_drag(),
                    );
                    let react = drag_area.rect;
                    ui.painter().text(
                        react.left_center() + egui::vec2(0.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        current_title,
                        egui::FontId::proportional(16.0),
                        ui.visuals().text_color(),
                    );
                    if drag_area.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if ui.button("X").on_hover_text("Close").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });

        if self.external_conflict {
            egui::Panel::top("external_conflict")
                .exact_size(34.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Note files changed outside Lilo.");
                        if ui.small_button("Reload disk version").clicked() {
                            self.dirty_note_ids.clear();
                            self.pending_title_rename_ids.clear();
                            self.reload_vault("Reloaded disk version");
                        }
                        if ui.small_button("Keep my version").clicked() {
                            self.external_conflict = false;
                            self.flush_dirty_notes();
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            match self.view {
                AppView::Editor => {
                    ui.add_space(4.0);
                    let mut changed_note_id = None;
                    let mut note_name_changed = false;
                    let mut note_content_changed = false;
                    let mut activated_link_target = None;

                    if let Some(note) = self.data.selected_note_mut() {
                        let title_response = ui.add(
                            egui::TextEdit::singleline(&mut note.title)
                                .desired_width(f32::INFINITY)
                                .hint_text("Note title..."),
                        );

                        ui.add_space(6.0);

                        // UUID preserves cursor and undo state between frames.
                        let editor_id = ui.make_persistent_id(("markdown_editor", note.id));
                        let editor_output = markdown::show_editor(
                            ui,
                            &mut note.content,
                            editor_id,
                            self.settings.font_size,
                        );

                        let hovered_character = markdown::hovered_character(ui, &editor_output);
                        let checkbox_toggled = hovered_character.is_some_and(|character_index| {
                            editor_output.response.clicked()
                                && !ui.input(|input| input.modifiers.command)
                                && markdown::toggle_checkbox_at_character(
                                    &mut note.content,
                                    character_index,
                                )
                        });

                        if let Some(character_index) = hovered_character
                            && let Some(wiki_link) =
                                links::wiki_link_at_character(&note.content, character_index)
                        {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            editor_output
                                .response
                                .response
                                .clone()
                                .on_hover_text(format!(
                                    "Ctrl+Click to open [[{}]]",
                                    wiki_link.target
                                ));

                            let command_clicked = editor_output.response.clicked()
                                && ui.input(|input| input.modifiers.command);
                            if command_clicked {
                                activated_link_target = Some(wiki_link.target);
                            }
                        }

                        let content_response = editor_output.response;

                        if self.focus_editor {
                            content_response.request_focus();
                            self.focus_editor = false;
                        }

                        note_name_changed = title_response.changed();
                        note_content_changed = content_response.changed() || checkbox_toggled;
                        if note_name_changed || note_content_changed {
                            note.mark_as_updated();
                            changed_note_id = Some(note.id);
                        }

                        show_note_dates(ui, note);
                    } else {
                        ui.label("No notes yet.");
                        if ui.button("Create note").clicked() {
                            self.create_note();
                        }
                    }

                    if let Some(id) = changed_note_id {
                        if note_name_changed {
                            self.pending_title_rename_ids.insert(id);
                            // Renaming can change link resolution across the vault.
                            self.link_index =
                                LinkIndex::build(&self.data.notes, &self.storage_paths.notes_dir);
                        } else if note_content_changed
                            && let Some(note) = self.data.notes.iter().find(|note| note.id == id)
                        {
                            // Content edits only require reparsing this note.
                            self.link_index.refresh_note_content(note);
                        }
                        self.mark_note_dirty(id);
                    }

                    if let Some(target) = activated_link_target {
                        match self.link_index.resolve_target(&target) {
                            LinkResolution::Resolved(id) => self.open_note(id),
                            LinkResolution::Missing => self.create_note_from_link(&target),
                            LinkResolution::Ambiguous => {
                                self.storage_message = Some(format!(
                                    "Cannot open [[{target}]]: more than one note has this name"
                                ));
                            }
                        }
                    }

                    if let Some(note_id) = self.data.selected_note_id
                        && let Some(links) = self.link_index.links_for(note_id)
                    {
                        let connection_count =
                            links.outgoing.len() + links.backlinks.len() + links.unresolved.len();
                        if connection_count > 0 {
                            let link_status = format!(
                                "Links: {}  ·  Backlinks: {}  ·  Missing: {}",
                                links.outgoing.len(),
                                links.backlinks.len(),
                                links.unresolved.len()
                            );
                            let response = ui.small(link_status);

                            if !links.unresolved.is_empty() {
                                response.on_hover_text(format!(
                                    "Missing notes:\n{}",
                                    links.unresolved.join("\n")
                                ));
                            }
                        }
                    }

                    if let Some(message) = &self.storage_message {
                        ui.small(message);
                    }
                }
                AppView::NotesList => self.show_notes_list(ui),
                AppView::Graph => {
                    let output = graph::show(
                        ui,
                        &mut self.graph_state,
                        &self.data.notes,
                        &self.link_index,
                        self.data.selected_note_id,
                        &self.storage_paths.notes_dir,
                        &self.settings.selected_folder,
                    );
                    self.handle_graph_output(output);
                }
                AppView::Trash => self.show_trash(ui),
                AppView::Settings => self.show_settings(ui, &ctx),
            }
        });

        if self.graph_overlay_open {
            let mut open = true;
            let mut graph_output = None;
            egui::Window::new("Knowledge graph")
                .id(egui::Id::new("graph_overlay"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(320.0, 360.0))
                .show(&ctx, |ui| {
                    let output = graph::show(
                        ui,
                        &mut self.graph_state,
                        &self.data.notes,
                        &self.link_index,
                        self.data.selected_note_id,
                        &self.storage_paths.notes_dir,
                        &self.settings.selected_folder,
                    );
                    graph_output = Some(output);
                });
            self.graph_overlay_open = open;
            if graph_output.is_some_and(|output| self.handle_graph_output(output)) {
                self.graph_overlay_open = false;
            }
        }

        self.save_after_debounce(&ctx);
        self.sync_external_changes(&ctx);
    }

    fn on_exit(&mut self) {
        self.flush_dirty_notes();
    }
}
#[cfg(target_os = "windows")]
fn set_autostart(enabled: bool) -> std::io::Result<()> {
    use std::process::Command;

    if !enabled {
        for value in ["Lilo", "RustWidgets"] {
            let _ = Command::new("reg")
                .arg("delete")
                .arg(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run")
                .arg("/v")
                .arg(value)
                .arg("/f")
                .status();
        }
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    let status = Command::new("reg")
        .arg("add")
        .arg(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run")
        .arg("/v")
        .arg("Lilo")
        .arg("/t")
        .arg("REG_SZ")
        .arg("/d")
        .arg(format!("\"{}\"", exe_path.display()))
        .arg("/f")
        .status()?;
    let _ = Command::new("reg")
        .arg("delete")
        .arg(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run")
        .arg("/v")
        .arg("RustWidgets")
        .arg("/f")
        .status();
    status
        .success()
        .then_some(())
        .ok_or_else(|| std::io::Error::other("Windows registry command failed"))
}

#[cfg(not(target_os = "windows"))]
fn set_autostart(_enabled: bool) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "Autostart is only supported on Windows",
    ))
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([360.0, 450.0])
            .with_min_inner_size([250.0, 180.0])
            .with_decorations(false)
            .with_always_on_top(),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "Lilo",
        native_options,
        Box::new(|_creation_context| Ok(Box::new(WidgetApp::new()))),
    )
}
