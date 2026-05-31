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
            // Close the menu if we left-click anywhere
            if button == iced::mouse::Button::Left {
                if app.context_menu.visible {
                    app.context_menu = ContextMenu::close();
                }
            }
            // Note: We don't handle Right click globally here anymore to avoid conflicts.
            // Right clicks are now entirely handled by `mouse_area` components in `gui.rs`.
        }

        Message::Click(idx) => {
            app.context_menu = ContextMenu::close();

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
            match opt_idx {
                Some(idx) => {
                    // Item right-click
                    if !app.selected.contains(&idx) {
                        app.selected.clear();
                        app.selected.insert(idx);
                        app.last_clicked = Some(idx);
                    }
                    let targets = app.selected_paths();
                    app.context_menu = ContextMenu::open(app.cursor_position.x, app.cursor_position.y, targets);
                }
                None => {
                    // Background right-click
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

            // Intercept Open action on directories so we stay inside Philes
            if action == ContextAction::Open && targets.len() == 1 && targets[0].is_dir() {
                app.navigate(targets[0].clone());
            } else if let Some(err) = actions::execute(&action, &targets, &mut app.clipboard) {
                app.error = Some(err);
            } else {
                if matches!(
                    action,
                    ContextAction::Delete | ContextAction::Cut | ContextAction::Paste | ContextAction::NewFolder
                ) {
                    let dir = app.current_dir.clone();
                    app.navigate(dir);
                }
            }
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