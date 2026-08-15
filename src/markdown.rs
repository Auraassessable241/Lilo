//! Cursor-aware Markdown layout for the live editor.

use eframe::egui::{
    Color32, FontFamily, FontId, Id, Stroke, TextEdit, TextFormat, Ui, Visuals, text::LayoutJob,
    text_edit::TextEditOutput, text_edit::TextEditState,
};

const HIDDEN_MARKER_SIZE: f32 = 0.1;

/// Shows one live Markdown editor. `id` owns its cursor and undo state.
pub fn show_editor(ui: &mut Ui, text: &mut String, id: Id, body_size: f32) -> TextEditOutput {
    let active_line = ui
        .memory(|memory| memory.has_focus(id))
        .then(|| TextEditState::load(ui.ctx(), id))
        .flatten()
        .and_then(|state| state.cursor.char_range())
        .map(|range| line_at_character(text, range.primary.index.into()));

    let mut layouter = |ui: &Ui, buffer: &dyn eframe::egui::TextBuffer, wrap_width: f32| {
        let mut layout_job = highlight(buffer.as_str(), ui.visuals(), active_line, body_size);
        layout_job.wrap.max_width = wrap_width;
        ui.fonts_mut(|fonts| fonts.layout_job(layout_job))
    };

    TextEdit::multiline(text)
        .id(id)
        .desired_width(f32::INFINITY)
        .desired_rows(20)
        .hint_text("Enter Markdown here...")
        .layouter(&mut layouter)
        .show(ui)
}

/// Returns the source character under the pointer.
pub fn hovered_character(ui: &Ui, output: &TextEditOutput) -> Option<usize> {
    let pointer = ui.input(|input| input.pointer.hover_pos())?;
    if !output.text_clip_rect.contains(pointer) || !output.response.response.rect.contains(pointer)
    {
        return None;
    }

    let local_position = pointer - output.galley_pos;
    Some(output.galley.cursor_from_pos(local_position).index.into())
}

/// Toggles a task marker only when the pointer is over that marker.
pub fn toggle_checkbox_at_character(text: &mut String, character_index: usize) -> bool {
    let byte_index = text
        .char_indices()
        .nth(character_index)
        .map_or(text.len(), |(index, _)| index);
    let line_start = text[..byte_index].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[byte_index..]
        .find('\n')
        .map_or(text.len(), |index| byte_index + index);
    let line = &text[line_start..line_end];
    let indent = line.len() - line.trim_start().len();
    let marker_start = line_start + indent;
    let marker_end = marker_start + 6;
    if byte_index < marker_start || byte_index >= marker_end || marker_end > text.len() {
        return false;
    }

    let marker = &text[marker_start..marker_end];
    let replacement = match marker {
        "- [ ] " => "- [x] ",
        "- [x] " | "- [X] " => "- [ ] ",
        _ => return false,
    };
    text.replace_range(marker_start..marker_end, replacement);
    true
}

fn line_at_character(text: &str, character_index: usize) -> usize {
    text.chars()
        .take(character_index)
        .filter(|character| *character == '\n')
        .count()
}

fn highlight(
    source: &str,
    visuals: &Visuals,
    active_line: Option<usize>,
    body_size: f32,
) -> LayoutJob {
    let palette = Palette::new(visuals, body_size);
    let mut job = LayoutJob::default();
    let mut inside_code_block = false;

    // Galley text must exactly match the editable source.
    for (line_index, source_line) in source.split_inclusive('\n').enumerate() {
        let (line, newline) = source_line
            .strip_suffix('\n')
            .map_or((source_line, ""), |line| (line, "\n"));

        append_line(
            &mut job,
            line,
            &palette,
            &mut inside_code_block,
            active_line == Some(line_index),
        );
        append(&mut job, newline, palette.body.clone());
    }

    job
}

struct Palette {
    body: TextFormat,
    marker: TextFormat,
    hidden_marker: TextFormat,
    accent: TextFormat,
    inline_code: TextFormat,
    code_block: TextFormat,
}

impl Palette {
    fn new(visuals: &Visuals, body_size: f32) -> Self {
        let body = format(FontFamily::Proportional, body_size, visuals.text_color());
        let marker = format(
            FontFamily::Monospace,
            body_size - 1.0,
            visuals.weak_text_color(),
        );
        let hidden_marker = format(
            FontFamily::Monospace,
            HIDDEN_MARKER_SIZE,
            Color32::TRANSPARENT,
        );

        let mut accent = body.clone();
        accent.color = visuals.hyperlink_color;

        let mut inline_code = format(
            FontFamily::Monospace,
            body_size - 0.5,
            visuals.hyperlink_color,
        );
        inline_code.background = visuals.code_bg_color;

        let mut code_block = format(FontFamily::Monospace, body_size - 0.5, visuals.text_color());
        code_block.background = visuals.code_bg_color;

        Self {
            body,
            marker,
            hidden_marker,
            accent,
            inline_code,
            code_block,
        }
    }

    fn source_marker(&self, visible: bool) -> TextFormat {
        if visible {
            self.marker.clone()
        } else {
            self.hidden_marker.clone()
        }
    }
}

fn format(family: FontFamily, size: f32, color: Color32) -> TextFormat {
    TextFormat {
        font_id: FontId::new(size, family),
        color,
        ..Default::default()
    }
}

fn append_line(
    job: &mut LayoutJob,
    line: &str,
    palette: &Palette,
    inside_code_block: &mut bool,
    show_source: bool,
) {
    let trimmed = line.trim_start();
    let indent_length = line.len() - trimmed.len();

    if trimmed.starts_with("```") {
        append(job, line, palette.source_marker(show_source));
        *inside_code_block = !*inside_code_block;
        return;
    }

    if *inside_code_block {
        append(job, line, palette.code_block.clone());
        return;
    }

    if let Some(level) = heading_level(trimmed) {
        append(job, &line[..indent_length], palette.body.clone());

        let marker_length = level + 1;
        let heading_size = palette.body.font_id.size
            * match level {
                1 => 1.73,
                2 => 1.47,
                3 => 1.27,
                _ => 1.13,
            };
        let heading = format(FontFamily::Proportional, heading_size, palette.body.color);
        let mut heading_marker = heading.clone();
        heading_marker.color = palette.marker.color;

        append(
            job,
            &trimmed[..marker_length],
            if show_source {
                heading_marker
            } else {
                palette.hidden_marker.clone()
            },
        );
        append(job, &trimmed[marker_length..], heading);
        return;
    }

    if let Some(rest) = trimmed.strip_prefix('>') {
        append(job, &line[..indent_length], palette.body.clone());
        append(
            job,
            ">",
            if show_source {
                palette.accent.clone()
            } else {
                palette.hidden_marker.clone()
            },
        );

        let mut quote = palette.body.clone();
        quote.italics = true;
        append_inline(job, rest, &quote, palette, show_source);
        return;
    }

    // TextEdit cannot embed a painted separator in its galley.
    if is_horizontal_rule(trimmed) {
        append(job, line, palette.marker.clone());
        return;
    }

    if let Some(marker_length) = list_marker_length(trimmed) {
        append(job, &line[..indent_length], palette.body.clone());

        // List markers are rendered content, not hidden syntax.
        append(job, &trimmed[..marker_length], palette.accent.clone());
        append_inline(
            job,
            &trimmed[marker_length..],
            &palette.body,
            palette,
            show_source,
        );
        return;
    }

    append_inline(job, line, &palette.body, palette, show_source);
}

fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| line.as_bytes().get(hashes))
        .flatten()
        .filter(|byte| **byte == b' ')
        .map(|_| hashes)
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|character| *character != ' ').collect();
    compact.len() >= 3
        && (compact.chars().all(|character| character == '-')
            || compact.chars().all(|character| character == '*')
            || compact.chars().all(|character| character == '_'))
}

fn list_marker_length(line: &str) -> Option<usize> {
    if line.starts_with("- [ ] ") || line.starts_with("- [x] ") || line.starts_with("- [X] ") {
        return Some(6);
    }
    if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
        return Some(2);
    }

    let digit_count = line.bytes().take_while(u8::is_ascii_digit).count();
    (digit_count > 0 && line[digit_count..].starts_with(". ")).then_some(digit_count + 2)
}

fn append_inline(
    job: &mut LayoutJob,
    mut source: &str,
    base: &TextFormat,
    palette: &Palette,
    show_source: bool,
) {
    while !source.is_empty() {
        let marker_position = source
            .char_indices()
            .find(|(_, character)| matches!(character, '`' | '*' | '_' | '~' | '['))
            .map_or(source.len(), |(position, _)| position);

        if marker_position > 0 {
            append(job, &source[..marker_position], base.clone());
            source = &source[marker_position..];
            continue;
        }

        if let Some(consumed) = append_inline_token(job, source, base, palette, show_source) {
            source = &source[consumed..];
            continue;
        }

        let character_length = source.chars().next().map_or(0, char::len_utf8);
        append(job, &source[..character_length], base.clone());
        source = &source[character_length..];
    }
}

fn append_inline_token(
    job: &mut LayoutJob,
    source: &str,
    base: &TextFormat,
    palette: &Palette,
    show_source: bool,
) -> Option<usize> {
    let marker = || palette.source_marker(show_source);

    if let Some(without_opening) = source.strip_prefix("[[") {
        let closing = without_opening.find("]]")? + 2;
        let end = closing + 2;
        let mut link = palette.accent.clone();
        link.underline = Stroke::new(1.0, link.color);
        append(job, "[[", marker());

        let inner = &source[2..closing];
        if show_source {
            append(job, inner, link);
        } else if let Some(separator) = inner.find('|') {
            // Keep hidden target bytes to preserve cursor indices.
            append(job, &inner[..separator + 1], palette.hidden_marker.clone());
            append(job, &inner[separator + 1..], link);
        } else {
            append(job, inner, link);
        }
        append(job, "]]", marker());
        return Some(end);
    }

    if source.starts_with('[') {
        let label_end = source.find("](")?;
        let target_end = source[label_end + 2..].find(')')? + label_end + 2;
        let mut link = palette.accent.clone();
        link.underline = Stroke::new(1.0, link.color);
        append(job, "[", marker());
        append(job, &source[1..label_end], link);
        append(job, "](", marker());
        append(job, &source[label_end + 2..target_end], marker());
        append(job, ")", marker());
        return Some(target_end + 1);
    }

    if let Some(without_opening) = source.strip_prefix('`') {
        let closing = without_opening.find('`')? + 1;
        append(job, "`", marker());
        append(job, &source[1..closing], palette.inline_code.clone());
        append(job, "`", marker());
        return Some(closing + 1);
    }

    for delimiter in ["**", "__"] {
        if source.starts_with(delimiter) {
            let closing = source[2..].find(delimiter)? + 2;
            let mut strong = base.clone();
            strong.color = palette.accent.color;
            strong.extra_letter_spacing = 0.25;
            append(job, delimiter, marker());
            append(job, &source[2..closing], strong);
            append(job, delimiter, marker());
            return Some(closing + 2);
        }
    }

    if let Some(without_opening) = source.strip_prefix("~~") {
        let closing = without_opening.find("~~")? + 2;
        let mut struck = base.clone();
        struck.strikethrough = Stroke::new(1.0, base.color);
        append(job, "~~", marker());
        append(job, &source[2..closing], struck);
        append(job, "~~", marker());
        return Some(closing + 2);
    }

    for delimiter in ['*', '_'] {
        if source.starts_with(delimiter) {
            let delimiter_length = delimiter.len_utf8();
            let closing = source[delimiter_length..].find(delimiter)? + delimiter_length;
            let mut italic = base.clone();
            italic.italics = true;
            append(job, &source[..delimiter_length], marker());
            append(job, &source[delimiter_length..closing], italic);
            append(job, &source[closing..closing + delimiter_length], marker());
            return Some(closing + delimiter_length);
        }
    }

    None
}

fn append(job: &mut LayoutJob, text: &str, text_format: TextFormat) {
    job.append(text, 0.0, text_format);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighting_never_changes_the_source_text() {
        let source = "# Заголовок\n\n- **жирный** и `код`\n[[Другая заметка|ссылка]] 🦀\n";
        let job = highlight(source, &Visuals::dark(), None, 15.0);

        assert_eq!(job.text, source);
    }

    #[test]
    fn incomplete_markers_remain_editable() {
        let source = "Незакрытые **звёздочки и [ссылка";
        let job = highlight(source, &Visuals::dark(), Some(0), 15.0);

        assert_eq!(job.text, source);
    }

    #[test]
    fn cursor_line_is_counted_by_characters_not_utf8_bytes() {
        let source = "ёж 🦀\nвторая строка";
        let first_line_character_count = "ёж 🦀\n".chars().count();

        assert_eq!(line_at_character(source, first_line_character_count), 1);
    }

    #[test]
    fn inactive_line_contains_collapsed_marker_sections() {
        let job = highlight("**text**", &Visuals::dark(), None, 15.0);

        assert!(
            job.sections
                .iter()
                .any(|section| section.format.font_id.size == HIDDEN_MARKER_SIZE)
        );
    }

    #[test]
    fn checkbox_toggles_only_from_its_marker() {
        let mut text = "  - [ ] task".to_owned();
        assert!(toggle_checkbox_at_character(&mut text, 4));
        assert_eq!(text, "  - [x] task");
        assert!(!toggle_checkbox_at_character(&mut text, 10));
    }
}
