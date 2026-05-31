use iced::{Element, Task, Theme, Length};
use iced::widget::{button, column, row, scrollable, text, container, image};
use iced::event::Event;
use iced::keyboard::Modifiers;
use std::path::PathBuf;
use std::fs;
use std::collections::HashSet;
use file_icon_provider::get_file_icon;

const COLS: usize = 6;
const ICON_SIZE: u16 = 64;

fn main() -> iced::Result {
    iced::application("Philes", update, view)
        .theme(|_| Theme::TokyoNight)
        .subscription(|_| iced::event::listen().map(Message::Event))
        .run_with(Philes::new)
}

#[derive(Default)]
struct Philes {
    current_dir: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    selected: HashSet<usize>,
    last_clicked: Option<usize>,
    last_click_time: Option<(usize, std::time::Instant)>,
    shift_held: bool,
}

impl Philes {
    fn new() -> (Self, Task<Message>) {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut app = Philes::default();
        load_dir(&mut app, home);
        (app, Task::none())
    }
}

#[derive(Clone)]
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    icon: Option<iced::widget::image::Handle>,
}

#[derive(Debug, Clone)]
enum Message {
    Click(usize),
    GoUp,
    Event(Event),
}

fn update(app: &mut Philes, message: Message) -> Task<Message> {
    match message {
        Message::Event(Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m))) => {
            app.shift_held = m.contains(Modifiers::SHIFT);
        }

        Message::Click(idx) => {
            let now = std::time::Instant::now();
            let is_double = app.last_click_time
                .map(|(i, t)| i == idx && now.duration_since(t).as_millis() < 400)
                .unwrap_or(false);

            app.last_click_time = Some((idx, now));

            if is_double {
                let entry = &app.entries[idx];
                let path = entry.path.clone();
                let is_dir = entry.is_dir;
                app.selected.clear();
                if is_dir {
                    load_dir(app, path);
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

        Message::GoUp => {
            if let Some(parent) = app.current_dir.parent() {
                let parent = parent.to_path_buf();
                app.selected.clear();
                load_dir(app, parent);
            }
        }

        _ => {}
    }
    Task::none()
}

fn load_dir(app: &mut Philes, path: PathBuf) {
    match fs::read_dir(&path) {
        Ok(rd) => {
            let mut entries: Vec<Entry> = rd
                .filter_map(|e| e.ok())
                .map(|e| {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
                    // v1.0.0 API: get_file_icon(path, size: u16) -> Result<Icon, Error>
                    let icon = get_file_icon(&path, ICON_SIZE)
                        .ok()
                        .map(|ic| {
                            iced::widget::image::Handle::from_rgba(
                                ic.width, ic.height, ic.pixels
                            )
                        });
                    Entry { name, path, is_dir, icon }
                })
                .filter(|e| !e.name.starts_with('.'))
                .collect();

            entries.sort_by(|a, b| {
                b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
            });

            app.current_dir = path;
            app.entries = entries;
            app.error = None;
        }
        Err(e) => app.error = Some(e.to_string()),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 1).collect();
        format!("{}…", t)
    }
}

fn view(app: &Philes) -> Element<Message> {
    let path_str = app.current_dir.to_string_lossy().into_owned();

    let sel_count = app.selected.len();
    let status = if sel_count > 0 {
        format!("{} selected", sel_count)
    } else {
        format!("{} items", app.entries.len())
    };

    let header = row![
        button("↑ Up").on_press(Message::GoUp),
        text(path_str).size(13).width(Length::Fill),
        text(status).size(12),
    ]
    .spacing(12)
    .padding([10, 14])
    .align_y(iced::Alignment::Center);

    let grid_rows: Vec<Element<Message>> = app.entries
        .chunks(COLS)
        .enumerate()
        .map(|(row_idx, chunk)| {
            let cells: Vec<Element<Message>> = chunk.iter().enumerate().map(|(col_idx, entry)| {
                let idx = row_idx * COLS + col_idx;
                let is_selected = app.selected.contains(&idx);
                let name = truncate(&entry.name, 13);

                let icon_widget: Element<Message> = if let Some(handle) = &entry.icon {
                    image(handle.clone())
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(48.0))
                        .into()
                } else {
                    let emoji = if entry.is_dir { "📁" } else { "📄" };
                    text(emoji).size(36).into()
                };

                let cell = column![
                    icon_widget,
                    text(name).size(11),
                ]
                .align_x(iced::Alignment::Center)
                .spacing(4)
                .padding(8)
                .width(Length::Fixed(90.0));

                let btn = button(cell).on_press(Message::Click(idx));

                if is_selected {
                    container(btn)
                        .style(|theme: &Theme| {
                            let palette = theme.extended_palette();
                            container::Style {
                                background: Some(palette.primary.weak.color.into()),
                                border: iced::Border {
                                    color: palette.primary.base.color,
                                    width: 2.0,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                        .into()
                } else {
                    container(btn).into()
                }
            }).collect();

            row(cells).spacing(4).into()
        })
        .collect();

    let grid = scrollable(
        column(grid_rows).spacing(4).padding(12)
    )
    .height(Length::Fill);

    let error_row: Element<Message> = if let Some(err) = &app.error {
        text(format!("⚠️ {}", err)).size(12).into()
    } else {
        text("").into()
    };

    container(
        column![header, grid, error_row]
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}