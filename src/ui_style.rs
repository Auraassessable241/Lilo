//! Shared visual language for compact and expanded layouts.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

pub const TOP_BAR_HEIGHT: f32 = 42.0;
pub const TOOL_SIZE: f32 = 30.0;
pub const PANEL_MARGIN: i8 = 10;
pub const COMPACT_WIDTH: f32 = 430.0;

pub const NAV_BREAKPOINT: f32 = 560.0;
pub const EXPANDED_NAV_BREAKPOINT: f32 = 900.0;
pub const NAV_RAIL_WIDTH: f32 = 52.0;
pub const NAV_PANEL_WIDTH: f32 = 176.0;

#[derive(Clone, Copy)]
pub enum Icon {
    Editor,
    Notes,
    Graph,
    Trash,
    Settings,
    Add,
    Folder,
    Restore,
    Minimize,
    Maximize,
    Close,
}

fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let stroke = Stroke::new(1.6, color);
    let r = 7.0;
    match icon {
        Icon::Editor => {
            painter.line_segment(
                [center + Vec2::new(-5.0, 5.0), center + Vec2::new(5.0, -5.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(-6.0, 6.0), center + Vec2::new(-2.0, 5.0)],
                stroke,
            );
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(15.0)),
                CornerRadius::same(3),
                Stroke::new(1.0, color.gamma_multiply(0.55)),
                StrokeKind::Inside,
            );
        }
        Icon::Notes => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(14.0, 16.0)),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            for y in [-4.0, 0.0, 4.0] {
                painter.line_segment(
                    [center + Vec2::new(-4.0, y), center + Vec2::new(4.5, y)],
                    Stroke::new(1.2, color),
                );
            }
        }
        Icon::Graph => {
            let points = [
                center + Vec2::new(0.0, -6.0),
                center + Vec2::new(-6.0, 5.0),
                center + Vec2::new(7.0, 4.0),
            ];
            painter.line_segment([points[0], points[1]], stroke);
            painter.line_segment([points[0], points[2]], stroke);
            painter.line_segment([points[1], points[2]], stroke);
            for point in points {
                painter.circle_filled(point, 2.4, color);
            }
        }
        Icon::Trash => {
            painter.rect_stroke(
                Rect::from_min_max(center + Vec2::new(-5.5, -3.5), center + Vec2::new(5.5, 7.0)),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-7.0, -6.0),
                    center + Vec2::new(7.0, -6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-2.5, -8.0),
                    center + Vec2::new(2.5, -8.0),
                ],
                stroke,
            );
        }
        Icon::Settings => {
            painter.circle_stroke(center, 6.0, stroke);
            painter.circle_filled(center, 2.0, color);
            for index in 0..8 {
                let direction = Vec2::angled(index as f32 * std::f32::consts::TAU / 8.0);
                painter.line_segment([center + direction * 7.0, center + direction * 9.0], stroke);
            }
        }
        Icon::Add => {
            painter.line_segment(
                [center + Vec2::new(-r, 0.0), center + Vec2::new(r, 0.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(0.0, -r), center + Vec2::new(0.0, r)],
                stroke,
            );
        }
        Icon::Folder => {
            let folder =
                Rect::from_min_max(center + Vec2::new(-8.0, -5.0), center + Vec2::new(8.0, 6.0));
            painter.rect_stroke(folder, CornerRadius::same(2), stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    center + Vec2::new(-6.0, -7.0),
                    center + Vec2::new(0.0, -7.0),
                ],
                stroke,
            );
        }
        Icon::Restore => {
            painter.circle_stroke(center + Vec2::new(1.0, 0.0), 6.0, stroke);
            painter.line_segment(
                [
                    center + Vec2::new(-7.0, -1.0),
                    center + Vec2::new(-2.0, -5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-7.0, -1.0),
                    center + Vec2::new(-7.0, -6.0),
                ],
                stroke,
            );
        }
        Icon::Close => {
            painter.line_segment(
                [center + Vec2::new(-5.0, -5.0), center + Vec2::new(5.0, 5.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(5.0, -5.0), center + Vec2::new(-5.0, 5.0)],
                stroke,
            );
        }
        Icon::Minimize => {
            painter.line_segment(
                [center + Vec2::new(-6.0, 4.0), center + Vec2::new(6.0, 4.0)],
                stroke,
            );
        }
        Icon::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(12.0)),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
        }
    }
}

fn painted_button(
    ui: &mut Ui,
    icon: Icon,
    selected: bool,
    label: &str,
    expanded: bool,
    fill_width: bool,
) -> Response {
    let size = Vec2::new(
        if expanded {
            if fill_width {
                (ui.available_width() - 2.0).max(TOOL_SIZE)
            } else {
                (42.0 + label.chars().count() as f32 * 7.0).max(TOOL_SIZE)
            }
        } else {
            TOOL_SIZE
        },
        TOOL_SIZE,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            if selected {
                ui.visuals().selection.bg_fill
            } else {
                visuals.weak_bg_fill
            },
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
        let icon_center = if expanded {
            Pos2::new(rect.left() + 16.0, rect.center().y)
        } else {
            rect.center()
        };
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(20.0));
        let color = if selected {
            ui.visuals().hyperlink_color
        } else {
            visuals.fg_stroke.color
        };
        paint_icon(ui, icon_rect, icon, color);
        if expanded {
            ui.painter().text(
                Pos2::new(rect.left() + 32.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(14.0),
                color,
            );
        }
    }
    response.on_hover_text(label)
}

pub fn apply_theme(ctx: &egui::Context, dark: bool, accent: Color32) {
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    let mut style = (*ctx.style_of(theme)).clone();
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let (panel, surface, hover, border, text, muted) = if dark {
        (
            Color32::from_rgb(17, 19, 24),
            Color32::from_rgb(27, 30, 38),
            Color32::from_rgb(41, 47, 58),
            Color32::from_rgb(42, 47, 56),
            Color32::from_rgb(232, 234, 240),
            Color32::from_rgb(143, 150, 163),
        )
    } else {
        (
            Color32::from_rgb(246, 247, 250),
            Color32::WHITE,
            Color32::from_rgb(229, 233, 241),
            Color32::from_rgb(210, 215, 225),
            Color32::from_rgb(31, 35, 43),
            Color32::from_rgb(104, 111, 124),
        )
    };

    visuals.panel_fill = panel;
    visuals.window_fill = surface;
    visuals.extreme_bg_color = surface;
    visuals.text_edit_bg_color = Some(surface);
    visuals.faint_bg_color = hover.gamma_multiply(0.45);
    visuals.code_bg_color = hover.gamma_multiply(0.65);
    visuals.override_text_color = Some(text);
    visuals.weak_text_color = Some(muted);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent.gamma_multiply(0.55);
    visuals.selection.stroke = Stroke::new(1.0, accent);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0, border);

    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(7);
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_fill = surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);
    visuals.widgets.hovered.weak_bg_fill = hover;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.7));
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);
    visuals.widgets.active.weak_bg_fill = accent.gamma_multiply(0.35);
    visuals.widgets.active.bg_fill = accent.gamma_multiply(0.45);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent);
    visuals.widgets.active.corner_radius = CornerRadius::same(7);
    visuals.widgets.open.weak_bg_fill = hover;
    visuals.widgets.open.bg_fill = hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.75));
    visuals.widgets.open.corner_radius = CornerRadius::same(7);

    style.spacing.item_spacing = Vec2::new(7.0, 6.0);
    style.spacing.button_padding = Vec2::new(9.0, 5.0);
    style.spacing.interact_size = Vec2::new(36.0, 30.0);
    style.spacing.window_margin = egui::Margin::same(PANEL_MARGIN);
    style.visuals = visuals;
    ctx.set_style_of(theme, style);
}

pub fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, label: &str) -> Response {
    painted_button(ui, icon, selected, label, false, false)
}

pub fn navigation_button(
    ui: &mut Ui,
    icon: Icon,
    selected: bool,
    label: &str,
    expanded: bool,
) -> Response {
    painted_button(ui, icon, selected, label, expanded, true)
}

pub fn compact_action(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let compact = ui.available_width() < COMPACT_WIDTH;
    painted_button(ui, icon, false, label, !compact, false)
}

pub fn screen_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).size(22.0).strong());
}

pub fn card_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.inactive.bg_stroke.color,
        ))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
}

pub fn muted(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.label(
        RichText::new(text.into())
            .small()
            .color(ui.visuals().weak_text_color()),
    )
}

pub fn status_color(visuals: &egui::Visuals, is_error: bool) -> Color32 {
    if is_error {
        visuals.error_fg_color
    } else {
        visuals.hyperlink_color
    }
}
