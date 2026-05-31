use iced::{Element, Length, Theme};
use iced::widget::{
    button, column, container, image, row, scrollable, text, text_input, Space,
    vertical_rule, mouse_area,
};

use crate::Message;
use crate::app::Philes;
use crate::files::{ancestor_paths, truncate};
use crate::actions::ContextAction;

const ITEM_WIDTH: f32 = crate::ITEM_WIDTH;
const SIDEBAR_WIDTH: f32 = 160.0;
pub const SIDEBAR_WIDTH_PUB: f32 = SIDEBAR_WIDTH;

// ── Sidebar ───────────────────────────────────────────────────────────────────

pub fn sidebar(app: &Philes) -> Element<Message> {
    let mut col: Vec<Element<Message>> = Vec::new();

    col.push(
        container(text("PLACES").size(10))
            .padding(iced::Padding { top: 10.0, right: 14.0, bottom: 4.0, left: 14.0 })
            .into(),
    );

    for (i, vol) in app.volumes.iter().enumerate() {
        let is_active = app.current_dir == vol.path;

        let label_row = row![
            text(sidebar_icon(&vol.label)).size(14),
            Space::with_width(6.0),
            text(vol.label.clone()).size(13),
        ]
        .align_y(iced::Alignment::Center);

        let btn = button(label_row)
            .on_press(Message::NavigateTo(vol.path.clone()))
            .width(Length::Fill)
            .padding([6, 14]);

        let styled: Element<Message> = if is_active {
            container(btn.style(|theme: &Theme, status| {
                let mut s = button::primary(theme, status);
                s.border.radius = 6.0.into();
                s
            }))
            .padding([1, 6])
            .into()
        } else {
            container(btn.style(|theme: &Theme, status| {
                let mut s = button::text(theme, status);
                s.border.radius = 6.0.into();
                s
            }))
            .padding([1, 6])
            .into()
        };

        col.push(styled);

        if i == 6 {
            col.push(Space::with_height(4.0).into());
            col.push(
                container(text("DEVICES").size(10))
                    .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 4.0, left: 14.0 })
                    .into(),
            );
        }
    }

    container(
        scrollable(column(col).spacing(0).width(Length::Fill))
    )
    .width(Length::Fixed(SIDEBAR_WIDTH))
    .height(Length::Fill)
    .style(|theme: &Theme| {
        let p = theme.extended_palette();
        container::Style {
            background: Some(
                iced::Color {
                    r: p.background.base.color.r * 0.88,
                    g: p.background.base.color.g * 0.88,
                    b: p.background.base.color.b * 0.92,
                    a: 1.0,
                }
                .into(),
            ),
            ..Default::default()
        }
    })
    .into()
}

fn sidebar_icon(label: &str) -> &'static str {
    match label {
        "Home"      => "[~]",
        "Desktop"   => "[D]",
        "Documents" => "[Doc]",
        "Downloads" => "[v]",
        "Music"     => "[M]",
        "Pictures"  => "[P]",
        "Videos"    => "[>]",
        "/ (root)"  => "[/]",
        _           => "[Drive]",
    }
}

// ── Breadcrumb address bar ────────────────────────────────────────────────────

pub fn address_bar(app: &Philes) -> Element<Message> {
    if app.address_editing {
        text_input("", &app.address_input)
            .id("address-bar")
            .on_input(Message::AddressInput)
            .on_submit(Message::AddressSubmit)
            .size(13)
            .width(Length::Fill)
            .into()
    } else {
        let ancestors = ancestor_paths(&app.current_dir);
        let len = ancestors.len();
        let crumbs: Vec<Element<Message>> = ancestors
            .into_iter()
            .enumerate()
            .flat_map(|(i, comp_path)| {
                let label = if comp_path == std::path::PathBuf::from("/") {
                    "/".to_string()
                } else {
                    comp_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| comp_path.to_string_lossy().into_owned())
                };

                let is_last = i == len - 1;
                let mut parts: Vec<Element<Message>> = Vec::new();

                if is_last {
                    parts.push(text(label).size(13).into());
                } else {
                    parts.push(
                        button(text(label).size(13))
                            .on_press(Message::NavigateTo(comp_path))
                            .padding([2, 4])
                            .into(),
                    );
                    parts.push(text(" > ").size(13).into());
                }
                parts
            })
            .collect();

        button(
            row(crumbs).spacing(0).align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddressClicked)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
    }
}

// ── File grid ─────────────────────────────────────────────────────────────────

pub fn file_grid(app: &Philes) -> Element<Message> {
    let cols = app.cols();

    let grid_rows: Vec<Element<Message>> = app
        .entries
        .chunks(cols)
        .enumerate()
        .map(|(row_idx, chunk)| {
            let mut cells: Vec<Element<Message>> = chunk
                .iter()
                .enumerate()
                .map(|(col_idx, entry)| {
                    let idx = row_idx * cols + col_idx;
                    let is_selected = app.selected.contains(&idx);
                    let name = truncate(&entry.name, 14);

                    let icon_widget: Element<Message> = if let Some(handle) = &entry.icon {
                        image(handle.clone())
                            .width(Length::Fixed(84.0))
                            .height(Length::Fixed(84.0))
                            .into()
                    } else {
                        let text_icon = if entry.is_dir { "[DIR]" } else { "[FILE]" };
                        text(text_icon).size(24).into()
                    };

                    let cell = column![icon_widget, text(name).size(13)]
                        .align_x(iced::Alignment::Center)
                        .spacing(8)
                        .padding(iced::Padding { top: 10.0, right: 2.0, bottom: 10.0, left: 2.0 })
                        .width(Length::Fixed(ITEM_WIDTH));

                    let cell_container = container(cell).width(Length::Fixed(ITEM_WIDTH));

                    let styled_container = if is_selected {
                        cell_container.style(|theme: &Theme| {
                            let palette = theme.extended_palette();
                            container::Style {
                                background: Some(
                                    iced::Color {
                                        a: 0.15,
                                        ..palette.primary.base.color
                                    }
                                    .into(),
                                ),
                                border: iced::Border {
                                    color: iced::Color {
                                        a: 0.5,
                                        ..palette.primary.base.color
                                    },
                                    width: 1.0,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                    } else {
                        cell_container.style(|_theme: &Theme| {
                            container::Style {
                                border: iced::Border {
                                    color: iced::Color::TRANSPARENT,
                                    width: 1.0,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                    };

                    // mouse_area takes the container and routes left/right clicks
                    mouse_area(styled_container)
                        .on_press(Message::Click(idx))
                        .on_right_press(Message::RightClick(Some(idx)))
                        .into()
                })
                .collect();

            let remainder = chunk.len() % cols;
            if remainder != 0 {
                for _ in 0..(cols - remainder) {
                    cells.push(Space::with_width(Length::Fixed(ITEM_WIDTH)).into());
                }
            }

            row(cells).spacing(0).into()
        })
        .collect();

    let grid_scrollable = scrollable(
        column(grid_rows).spacing(16).padding([8, 12]).width(Length::Fill),
    )
    .height(Length::Fill);

    // Background empty space right-click handler
    mouse_area(grid_scrollable)
        .on_right_press(Message::RightClick(None))
        .into()
}

// ── Context menu overlay ──────────────────────────────────────────────────────

pub fn context_menu_overlay(app: &Philes) -> Option<Element<Message>> {
    if !app.context_menu.visible {
        return None;
    }

    let is_background = app.context_menu.targets.len() == 1 && app.context_menu.targets[0] == app.current_dir;

    let actions = if is_background {
        ContextAction::background_actions()
    } else {
        ContextAction::item_actions()
    };

    let items: Vec<Element<Message>> = actions
        .into_iter()
        .map(|action| {
            let label_row = row![
                container(text(action.icon()).size(14))
                    .width(Length::Fixed(30.0)),
                text(action.label()).size(13),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);

            button(label_row)
                .on_press(Message::ContextAction(action))
                .width(Length::Fill)
                .padding([6, 14])
                .style(|theme: &Theme, status| {
                    let mut s = button::text(theme, status);
                    s.border.radius = 4.0.into();
                    s
                })
                .into()
        })
        .collect();

    let mut menu_col: Vec<Element<Message>> = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        menu_col.push(item);
        if !is_background && i == 1 {
            menu_col.push(
                container(Space::with_height(1.0))
                    .style(|theme: &Theme| {
                        let p = theme.extended_palette();
                        container::Style {
                            background: Some(
                                iced::Color { a: 0.2, ..p.background.strong.color }.into(),
                            ),
                            ..Default::default()
                        }
                    })
                    .width(Length::Fill)
                    .padding([2, 8])
                    .into(),
            );
        }
    }

    let menu = container(column(menu_col).spacing(2).padding([6, 0]))
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(
                    iced::Color {
                        r: p.background.base.color.r * 0.95,
                        g: p.background.base.color.g * 0.95,
                        b: p.background.base.color.b * 1.05,
                        a: 0.97,
                    }
                    .into(),
                ),
                border: iced::Border {
                    color: iced::Color { a: 0.25, ..p.background.strong.color },
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow {
                    color: iced::Color { a: 0.35, ..iced::Color::BLACK },
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                text_color: None,
            }
        })
        .width(Length::Fixed(190.0));

    Some(menu.into())
}

// ── Top-level view ────────────────────────────────────────────────────────────

pub fn view(app: &Philes) -> Element<Message> {
    let sel_count = app.selected.len();
    let status = if sel_count > 0 {
        format!("{} selected", sel_count)
    } else {
        format!("{} items", app.entries.len())
    };

    let header = row![
        button("Up").on_press(Message::GoUp).padding([4, 10]),
        address_bar(app),
        text(status).size(12),
    ]
    .spacing(8)
    .padding([8, 12])
    .align_y(iced::Alignment::Center);

    let error_row: Element<Message> = if let Some(err) = &app.error {
        text(format!("[ERROR] {}", err)).size(12).into()
    } else {
        Space::with_height(0.0).into()
    };

    let main_pane = column![header, file_grid(app), error_row]
        .width(Length::Fill);

    let body = row![
        sidebar(app),
        vertical_rule(1),
        main_pane,
    ]
    .height(Length::Fill);

    if let Some(menu) = context_menu_overlay(app) {
        use iced::widget::stack;
        let positioned_menu = container(menu)
            .padding(iced::Padding {
                top: app.context_menu.y,
                left: app.context_menu.x,
                right: 0.0,
                bottom: 0.0,
            });

        stack![
            container(body).width(Length::Fill).height(Length::Fill),
            positioned_menu,
        ]
        .into()
    } else {
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}