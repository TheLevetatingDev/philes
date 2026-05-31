use std::collections::HashSet;
use std::path::PathBuf;
use iced::Task;

use crate::Message;
use crate::files::{load_entries, get_volumes, Entry, Volume};
use crate::actions::{self, Clipboard, ContextMenu, ContextAction};

pub struct Philes {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub error: Option<String>,
    pub selected: HashSet<usize>,
    pub last_clicked: Option<usize>,
    pub last_click_time: Option<(usize, std::time::Instant)>,
    pub shift_held: bool,
    pub window_width: f32,
    // Address bar
    pub address_editing: bool,
    pub address_input: String,
    pub address_last_click: Option<std::time::Instant>,
    // Sidebar
    pub volumes: Vec<Volume>,
    // Context menu
    pub context_menu: ContextMenu,
    // Clipboard
    pub clipboard: Clipboard,
    // Global tracking coordinates
    pub cursor_position: iced::Point,
    // Renaming state
    pub renaming_idx: Option<usize>,
    pub rename_input: String,
}

impl Philes {
    pub fn new() -> (Self, Task<Message>) {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let volumes = get_volumes();
        let entries = load_entries(&home).unwrap_or_default();

        let app = Philes {
            current_dir: home,
            entries,
            error: None,
            selected: HashSet::new(),
            last_clicked: None,
            last_click_time: None,
            shift_held: false,
            window_width: 800.0,
            address_editing: false,
            address_input: String::new(),
            address_last_click: None,
            volumes,
            context_menu: ContextMenu::default(),
            clipboard: Clipboard::default(),
            cursor_position: iced::Point::ORIGIN,
            renaming_idx: None,
            rename_input: String::new(),
        };

        (app, Task::none())
    }

    pub fn cols(&self) -> usize {
        let available = (self.window_width - crate::gui::SIDEBAR_WIDTH_PUB - 24.0).max(0.0);
        ((available / crate::ITEM_WIDTH).floor() as usize).max(1)
    }

    pub fn navigate(&mut self, path: PathBuf) {
        match load_entries(&path) {
            Ok(entries) => {
                self.current_dir = path;
                self.entries = entries;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        self.selected.clear();
        self.context_menu = ContextMenu::close();
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .into_iter()
            .filter_map(|i| self.entries.get(i))
            .map(|e| e.path.clone())
            .collect()
    }
}

pub fn update(app: &mut Philes, message: Message) -> Task<Message> {
    use iced::event::Event;
    use iced::keyboard::Modifiers;
    use iced::widget::text_input;
    use iced::window;
    use iced::Size;

    match message {
        Message::Event(Event::Window(window::Event::Resized(Size { width, height: _ }))) => {
            app.window_width = width;
        }

        Message::Event(Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m))) => {
            app.shift_held = m.contains(Modifiers::SHIFT);
        }

        Message::Event(Event::Mouse(iced::mouse::Event::CursorMoved { position })) => {
            app.cursor_position = position;
        }

        Message::Event(Event::Mouse(iced::mouse::Event::ButtonPressed(button))) => {
            if button == iced::mouse::Button::Left {
                if app.context_menu.visible {
                    app.context_menu = ContextMenu::close();
                }
            }
        }

        Message::Click(idx) => {
            app.context_menu = ContextMenu::close();

            // Auto-submit rename if the user clicks away to another file or background
            if app.renaming_idx.is_some() {
                if let Some(r_idx) = app.renaming_idx {
                    let old_path = &app.entries[r_idx].path;
                    let new_path = app.current_dir.join(&app.rename_input);
                    if old_path != &new_path && !app.rename_input.is_empty() {
                        let _ = std::fs::rename(old_path, &new_path);
                    }
                }
                app.renaming_idx = None;
                app.navigate(app.current_dir.clone());
                return Task::none(); // Stop processing this click so we don't accidentally open a file while saving
            }

            let now = std::time::Instant::now();
            let is_double = app
                .last_click_time
                .map(|(i, t)| i == idx && now.duration_since(t).as_millis() < 400)
                .unwrap_or(false);

            app.last_click_time = Some((idx, now));

            if is_double {
                let entry = &app.entries[idx];
                let path = entry.path.clone();
                if entry.is_dir {
                    app.navigate(path);
                } else {
                    let _ = open::that(&path);
                }
                return Task::none();
            }

            if app.shift_held {
                if let Some(last) = app.last_clicked {
                    let lo = last.min(idx);
                    let hi = last.max(idx);
                    for i in lo..=hi {
                        app.selected.insert(i);
                    }
                } else {
                    app.selected.insert(idx);
                }
            } else {
                app.selected.clear();
                app.selected.insert(idx);
            }
            app.last_clicked = Some(idx);
        }

        Message::RightClick(opt_idx) => {
            // Auto-submit rename if they right click elsewhere
            if app.renaming_idx.is_some() {
                if let Some(r_idx) = app.renaming_idx {
                    let old_path = &app.entries[r_idx].path;
                    let new_path = app.current_dir.join(&app.rename_input);
                    if old_path != &new_path && !app.rename_input.is_empty() {
                        let _ = std::fs::rename(old_path, &new_path);
                        app.navigate(app.current_dir.clone());
                    }
                }
                app.renaming_idx = None;
            }

            match opt_idx {
                Some(idx) => {
                    if !app.selected.contains(&idx) {
                        app.selected.clear();
                        app.selected.insert(idx);
                        app.last_clicked = Some(idx);
                    }
                    let targets = app.selected_paths();
                    app.context_menu = ContextMenu::open(app.cursor_position.x, app.cursor_position.y, targets);
                }
                None => {
                    app.selected.clear();
                    app.last_clicked = None;
                    app.context_menu = ContextMenu::open(
                        app.cursor_position.x,
                        app.cursor_position.y,
                        vec![app.current_dir.clone()],
                    );
                }
            }
        }

        Message::ContextAction(action) => {
            let targets = app.context_menu.targets.clone();
            app.context_menu = ContextMenu::close();

            match action {
                ContextAction::Rename => {
                    if let Some(&idx) = app.selected.iter().next() {
                        app.renaming_idx = Some(idx);
                        app.rename_input = app.entries[idx].name.clone();
                        return text_input::focus("rename-input");
                    }
                }
                ContextAction::NewFolder => {
                    if let Some(dest_dir) = targets.first() {
                        let mut name = "Untitled Folder".to_string();
                        let mut count = 1;
                        while dest_dir.join(&name).exists() {
                            count += 1;
                            name = format!("Untitled Folder {}", count);
                        }
                        let new_path = dest_dir.join(&name);
                        if let Err(e) = std::fs::create_dir(&new_path) {
                            app.error = Some(format!("Failed to create folder: {e}"));
                        } else {
                            app.navigate(app.current_dir.clone());
                            // Find it in the newly loaded list and auto-focus rename
                            if let Some(idx) = app.entries.iter().position(|e| e.path == new_path) {
                                app.selected.clear();
                                app.selected.insert(idx);
                                app.renaming_idx = Some(idx);
                                app.rename_input = name;
                                return text_input::focus("rename-input");
                            }
                        }
                    }
                }
                ContextAction::Open if targets.len() == 1 && targets[0].is_dir() => {
                    app.navigate(targets[0].clone());
                }
                _ => {
                    if let Some(err) = actions::execute(&action, &targets, &mut app.clipboard) {
                        app.error = Some(err);
                    } else {
                        if matches!(
                            action,
                            ContextAction::Delete | ContextAction::Cut | ContextAction::Paste
                        ) {
                            app.navigate(app.current_dir.clone());
                        }
                    }
                }
            }
        }

        // ── Renaming Logic ──
        Message::RenameInput(s) => {
            app.rename_input = s;
        }

        Message::RenameSubmit => {
            if let Some(idx) = app.renaming_idx {
                let old_path = &app.entries[idx].path;
                let new_path = app.current_dir.join(&app.rename_input);
                if old_path != &new_path && !app.rename_input.is_empty() {
                    if let Err(e) = std::fs::rename(old_path, &new_path) {
                        app.error = Some(format!("Failed to rename: {e}"));
                    }
                }
            }
            app.renaming_idx = None;
            app.navigate(app.current_dir.clone());
        }

        Message::RenameCancel => {
            app.renaming_idx = None;
        }

        Message::GoUp => {
            if let Some(parent) = app.current_dir.parent().map(|p| p.to_path_buf()) {
                app.navigate(parent);
            }
        }

        Message::NavigateTo(path) => {
            app.navigate(path);
        }

        Message::AddressClicked => {
            let now = std::time::Instant::now();
            let is_double = app
                .address_last_click
                .map(|t| now.duration_since(t).as_millis() < 400)
                .unwrap_or(false);
            app.address_last_click = Some(now);

            if is_double {
                app.address_editing = true;
                app.address_input = app.current_dir.to_string_lossy().into_owned();
                return text_input::focus("address-bar");
            }
        }

        Message::AddressInput(s) => {
            app.address_input = s;
        }

        Message::AddressSubmit => {
            let path = PathBuf::from(&app.address_input);
            app.address_editing = false;
            if path.is_dir() {
                app.navigate(path);
            } else {
                app.error = Some(format!("Not a directory: {}", app.address_input));
            }
        }

        Message::AddressCancel => {
            app.address_editing = false;
        }

        _ => {}
    }

    Task::none()
}