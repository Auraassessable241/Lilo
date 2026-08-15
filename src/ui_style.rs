//! Shared visual language for compact and expanded layouts.

use eframe::egui::{self, Button, Color32, Response, RichText, Ui, Vec2};

pub const TOP_BAR_HEIGHT: f32 = 42.0;
pub const TOOL_SIZE: f32 = 30.0;
pub const PANEL_MARGIN: i8 = 10;
pub const COMPACT_WIDTH: f32 = 430.0;

#[derive(Clone, Copy)]
pub enum Icon {
    Editor,
    Notes,
    Graph,
    Trash,
    Settings,
    Add,
    Folder,
    Search,
    Pin,
    More,
    Restore,
    Save,
    Bold,
    Italic,
    Code,
    Link,
    Task,
    Heading,
    Back,
    Close,
}

impl Icon {
    fn symbol(self) -> &'static str {
        match self {
            Self::Editor => "✎",
            Self::Notes => "☷",
            Self::Graph => "⌘",
            Self::Trash => "⌫",
            Self::Settings => "⚙",
            Self::Add => "+",
            Self::Folder => "▣",
            Self::Search => "⌕",
            Self::Pin => "◆",
            Self::More => "⋯",
            Self::Restore => "↶",
            Self::Save => "✓",
            Self::Bold => "B",
            Self::Italic => "I",
            Self::Code => "</>",
            Self::Link => "↗",
            Self::Task => "☑",
            Self::Heading => "H",
            Self::Back => "←",
            Self::Close => "×",
        }
    }
}

pub fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, label: &str) -> Response {
    let text = RichText::new(icon.symbol()).size(15.0);
    ui.add(
        Button::new(text)
            .selected(selected)
            .min_size(Vec2::splat(TOOL_SIZE)),
    )
    .on_hover_text(label)
}

pub fn compact_action(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let compact = ui.available_width() < COMPACT_WIDTH;
    let caption = if compact {
        icon.symbol().to_owned()
    } else {
        format!("{}  {label}", icon.symbol())
    };
    ui.add(Button::new(caption).min_size(Vec2::new(TOOL_SIZE, TOOL_SIZE)))
        .on_hover_text(label)
}

pub fn section_header(ui: &mut Ui, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.strong(title);
        if !detail.is_empty() {
            ui.label(RichText::new(detail).small().color(ui.visuals().weak_text_color()));
        }
    });
}

pub fn status_color(visuals: &egui::Visuals, is_error: bool) -> Color32 {
    if is_error {
        visuals.error_fg_color
    } else {
        visuals.hyperlink_color
    }
}
