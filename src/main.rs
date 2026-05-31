mod actions;
mod app;
mod files;
mod gui;

use iced::Theme;
use iced::event::Event;

pub use app::{Philes, update};
pub use actions::ContextAction;

pub const ITEM_WIDTH: f32 = 92.0;
pub const ICON_SIZE: u16 = 96;

fn main() -> iced::Result {
    iced::application("Philes", update, gui::view)
        .theme(|_| Theme::TokyoNight)
        .subscription(|_| iced::event::listen().map(Message::Event))
        .run_with(Philes::new)
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    /// Left-click on a grid item.
    Click(usize),
    /// Right-click. Some(idx) for an item, None for empty background space.
    RightClick(Option<usize>),
    /// Navigate up one directory level.
    GoUp,
    /// Navigate to an explicit path.
    NavigateTo(std::path::PathBuf),
    /// Raw iced event (keyboard, window resize, mouse…).
    Event(Event),
    // ── Address bar ──
    AddressClicked,
    AddressInput(String),
    AddressSubmit,
    AddressCancel,
    // ── Context menu ──
    ContextAction(ContextAction),
    CloseContextMenu,
}