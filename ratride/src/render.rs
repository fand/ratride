use crate::markdown::{HeaderItem, SemanticElement, Slide, SlideLayout};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    text::{Span, Text},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use unicode_width::UnicodeWidthChar;

/// Position where an image should be rendered.
/// Terminal backend uses this to draw images after ratatui render.
#[derive(Clone, Debug)]
pub struct ImagePlacement {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub path: String,
    /// True when image top is scrolled off-screen.
    pub clip_top: bool,
    /// Original (unclipped) image height in cells.
    pub full_height: u16,
}

/// A single hyperlink cell to be rendered via direct stdout writes (bypassing ratatui buffer).
#[derive(Clone, Debug)]
pub struct HyperlinkCell {
    pub sx: u16,
    pub sy: u16,
    pub url: String,
}

/// Draw a slide's main content area (dispatches by layout).
/// Returns image placements for the terminal backend to render.
pub fn draw_slide(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> (Vec<ImagePlacement>, Vec<HyperlinkCell>) {
    match slide.layout {
        SlideLayout::Default => draw_default(slide, scroll, frame, area),
        SlideLayout::Center => draw_center(slide, scroll, frame, area),
        SlideLayout::TwoColumn => {
            draw_two_column(slide, scroll, frame, area);
            (Vec::new(), Vec::new())
        }
    }
}

pub fn draw_default(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> (Vec<ImagePlacement>, Vec<HyperlinkCell>) {
    let content_area = area.inner(Margin::new(2, 1));
    let (content, index_map) = rewrap_bg_lines(&slide.content, content_area.width);

    fill_line_backgrounds(&content, scroll, frame, content_area);

    let paragraph = Paragraph::new(content.clone())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, content_area);

    let hyperlinks = collect_hyperlinks(&slide.semantics, &content, scroll, content_area, Alignment::Left, &index_map);

    let content_len = wrapped_content_height(&content, content_area.width);
    draw_scrollbar(scroll, content_len, content_area.height, frame, area);

    let mut placements = Vec::new();
    for img in &slide.images {
        let li = remap_index(img.line_index, &index_map);
        let y_off = wrapped_y_offset(&content, li, content_area.width);
        if let Some(p) = compute_image_placement(
            content_area,
            y_off,
            img.height,
            scroll,
            &img.path,
            false,
            0,
            0,
            img.max_width_percent,
        ) {
            placements.push(p);
        }
    }
    (placements, hyperlinks)
}

pub fn draw_center(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> (Vec<ImagePlacement>, Vec<HyperlinkCell>) {
    let content_area = area.inner(Margin::new(2, 1));
    let (content, index_map) = rewrap_bg_lines(&slide.content, content_area.width);
    let content_height = wrapped_content_height(&content, content_area.width) as u16;

    let [centered_area] = Layout::vertical([Constraint::Length(content_height)])
        .flex(Flex::Center)
        .areas(content_area);

    fill_line_backgrounds(&content, scroll, frame, centered_area);

    let paragraph = Paragraph::new(content.clone())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, centered_area);

    let hyperlinks = collect_hyperlinks(&slide.semantics, &content, scroll, centered_area, Alignment::Center, &index_map);

    let mut placements = Vec::new();
    for img in &slide.images {
        let li = remap_index(img.line_index, &index_map);
        let y_off = wrapped_y_offset(&content, li, centered_area.width);
        if let Some(p) = compute_image_placement(
            centered_area,
            y_off,
            img.height,
            scroll,
            &img.path,
            true,
            img.pixel_width,
            img.pixel_height,
            img.max_width_percent,
        ) {
            placements.push(p);
        }
    }
    (placements, hyperlinks)
}

pub fn draw_two_column(slide: &Slide, scroll: u16, frame: &mut Frame, area: Rect) {
    let content_area = area.inner(Margin::new(2, 1));

    let [left_area, _gap, right_area] = Layout::horizontal([
        Constraint::Percentage(48),
        Constraint::Percentage(4),
        Constraint::Percentage(48),
    ])
    .areas(content_area);

    let (left_content, _) = rewrap_bg_lines(&slide.content, left_area.width);
    let left_para = Paragraph::new(left_content)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(left_para, left_area);

    if let Some(ref right) = slide.right_content {
        let (right_content, _) = rewrap_bg_lines(right, right_area.width);
        let right_para = Paragraph::new(right_content)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        frame.render_widget(right_para, right_area);
    }
}

pub fn draw_scrollbar(
    scroll: u16,
    content_len: usize,
    visible: u16,
    frame: &mut Frame,
    area: Rect,
) {
    let visible = visible as usize;
    if content_len > visible {
        let mut scrollbar_state =
            ScrollbarState::new(content_len.saturating_sub(visible)).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

pub fn draw_status_bar(
    current_page: usize,
    total: usize,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) {
    draw_status_bar_with_options(current_page, total, frame, area, theme, false);
}

pub fn draw_status_bar_with_options(
    current_page: usize,
    total: usize,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    is_web: bool,
) {
    let quit_str = if is_web { "" } else { "  q:quit" };
    let left = format!(" ←/→:page  ↓/↑:scroll{}", quit_str);
    let right = format!("[{}/{}] ", current_page + 1, total);

    let style = ratatui::style::Style::default()
        .bg(theme.status_bg)
        .fg(theme.status_fg);

    // Fill background
    frame.render_widget(Paragraph::new("").style(style), area);

    let [left_area, right_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(right.len() as u16)])
            .areas(area);

    frame.render_widget(Paragraph::new(left).style(style), left_area);
    frame.render_widget(
        Paragraph::new(right).alignment(Alignment::Right).style(style),
        right_area,
    );
}

/// Draw header items at the top-right of the area, overlaying the content.
/// Items are displayed horizontally, separated by " │ ".
/// Items with a URL are rendered in the theme's link color.
/// Returns hyperlink cells for clickable header links.
pub fn draw_header(
    header: &[HeaderItem],
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) -> Vec<HyperlinkCell> {
    if header.is_empty() {
        return Vec::new();
    }

    let separator = " │ ";
    let mut spans = Vec::new();
    let style = ratatui::style::Style::default()
        .bg(theme.surface)
        .fg(theme.fg);
    let link_style = ratatui::style::Style::default()
        .bg(theme.surface)
        .fg(theme.link);
    let sep_style = ratatui::style::Style::default()
        .bg(theme.surface)
        .fg(theme.list_bullet);

    // Track span offsets for link positions
    let mut span_offsets: Vec<(usize, &HeaderItem)> = Vec::new();
    let mut offset: usize = 1; // starts at 1 for leading padding " "

    for (i, item) in header.iter().enumerate() {
        if i > 0 {
            spans.push(ratatui::text::Span::styled(separator, sep_style));
            offset += separator.len();
        }
        span_offsets.push((offset, item));
        let item_style = if item.url.is_some() { link_style } else { style };
        spans.push(ratatui::text::Span::styled(item.text.clone(), item_style));
        offset += item.text.len();
    }

    // Add padding
    spans.insert(0, ratatui::text::Span::styled(" ", style));
    spans.push(ratatui::text::Span::styled(" ", style));

    let line = ratatui::text::Line::from(spans);
    let width: u16 = line.width() as u16;

    // Position at top-right with 1-cell margin from the right edge
    let x = area.x + area.width.saturating_sub(width + 1);
    let header_area = Rect::new(x, area.y, width, 1);

    // Build hyperlink cells for items with URLs
    let mut hyperlinks = Vec::new();
    for (col_offset, item) in &span_offsets {
        if let Some(url) = &item.url {
            for c in 0..item.text.len() {
                hyperlinks.push(HyperlinkCell {
                    sx: x + *col_offset as u16 + c as u16,
                    sy: area.y,
                    url: url.clone(),
                });
            }
        }
    }

    let paragraph = Paragraph::new(line).alignment(Alignment::Right);
    frame.render_widget(paragraph, header_area);

    hyperlinks
}

/// Highlight hovered hyperlink cells in the buffer by swapping fg/bg.
pub fn highlight_hovered_hyperlinks(
    hyperlinks: &[HyperlinkCell],
    hovered_url: Option<&str>,
    frame: &mut Frame,
) {
    let url = match hovered_url {
        Some(u) => u,
        None => return,
    };
    let buf = frame.buffer_mut();
    for h in hyperlinks {
        if h.url == url {
            if let Some(cell) = buf.cell_mut((h.sx, h.sy)) {
                let fg = cell.fg;
                let bg = cell.bg;
                cell.set_fg(bg);
                cell.set_bg(fg);
            }
        }
    }
}

/// Collect screen positions for hyperlink cells.
/// The actual OSC 8 sequences are written directly to stdout after the frame is
/// flushed, bypassing ratatui's buffer diff (which would miscount the width of
/// cells containing embedded escape sequences).
fn remap_index(old: usize, index_map: &[usize]) -> usize {
    if old < index_map.len() {
        index_map[old]
    } else {
        old
    }
}

fn collect_hyperlinks(
    semantics: &[SemanticElement],
    content: &Text<'_>,
    scroll: u16,
    content_area: Rect,
    alignment: Alignment,
    index_map: &[usize],
) -> Vec<HyperlinkCell> {
    let width = content_area.width as usize;
    if width == 0 {
        return Vec::new();
    }

    let mut cells = Vec::new();

    for sem in semantics {
        let (url, line_index, start_col, end_col) = match sem {
            SemanticElement::Link {
                url,
                line_index,
                start_col,
                end_col,
                ..
            } => (url, *line_index, *start_col, *end_col),
            _ => continue,
        };

        if start_col >= end_col {
            continue;
        }

        let mapped_index = remap_index(line_index, index_map);

        // Compute screen row offset for this logical line
        let mut y_offset: i32 = 0;
        for (i, line) in content.lines.iter().enumerate() {
            if i == mapped_index {
                break;
            }
            y_offset += wrapped_line_height(line, content_area.width) as i32;
        }
        y_offset -= scroll as i32;

        // Compute centering offset for this line
        let center_offset = if alignment == Alignment::Center {
            let line_width = content
                .lines
                .get(mapped_index)
                .map(|l| l.width())
                .unwrap_or(0);
            (width.saturating_sub(line_width)) / 2
        } else {
            0
        };

        for col in start_col..end_col {
            let wrap_row = col / width;
            let x_in_row = col % width + center_offset;

            let screen_y = y_offset + wrap_row as i32;
            if screen_y < 0 || screen_y >= content_area.height as i32 {
                continue;
            }

            let sx = content_area.x + x_in_row as u16;
            let sy = content_area.y + screen_y as u16;

            cells.push(HyperlinkCell {
                sx,
                sy,
                url: url.clone(),
            });
        }
    }
    cells
}
/// Compute how many screen rows a line occupies when word-wrapped to `width` columns.
pub fn wrapped_line_height(line: &ratatui::text::Line<'_>, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let w = width as usize;
    let total_width = line.width();
    ((total_width + w - 1) / w).max(1) as u16
}

/// Total visual rows occupied by `content` after wrapping to `width` columns.
pub fn wrapped_content_height(content: &Text<'_>, width: u16) -> usize {
    content
        .lines
        .iter()
        .map(|l| wrapped_line_height(l, width) as usize)
        .sum()
}

/// Visual row offset of the line at `line_index`, accounting for wrapped lines above it.
fn wrapped_y_offset(content: &Text<'_>, line_index: usize, width: u16) -> usize {
    content
        .lines
        .iter()
        .take(line_index)
        .map(|l| wrapped_line_height(l, width) as usize)
        .sum()
}

/// Pre-fill buffer rows with Line::style background for full-width code block bg.
/// Accounts for line wrapping so backgrounds align with Paragraph's wrapped output.
fn fill_line_backgrounds(content: &Text<'_>, scroll: u16, frame: &mut Frame, area: Rect) {
    let buf = frame.buffer_mut();
    let mut y_row: i32 = -(scroll as i32);

    for line in content.lines.iter() {
        let wrapped_rows = wrapped_line_height(line, area.width) as i32;

        if let Some(bg) = line.style.bg {
            for dy in 0..wrapped_rows {
                let y_offset = y_row + dy;
                if y_offset >= 0 && y_offset < area.height as i32 {
                    let y = area.y + y_offset as u16;
                    for x in area.x..area.x + area.width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_bg(bg);
                        }
                    }
                }
            }
        }

        y_row += wrapped_rows;
    }
}

/// Re-wrap code block lines (those with bg) so that each visual row keeps
/// the 2-NBSP left padding. Returns (new content, index map from old→new line index).
fn rewrap_bg_lines(content: &Text<'_>, max_width: u16) -> (Text<'static>, Vec<usize>) {
    let max = max_width as usize;
    let padding = "\u{00a0}\u{00a0}";
    let padding_width = 2usize;
    let mut new_lines: Vec<ratatui::text::Line<'static>> = Vec::new();
    let mut index_map: Vec<usize> = Vec::new();

    for line in &content.lines {
        index_map.push(new_lines.len());

        if line.style.bg.is_none() || line.width() <= max {
            new_lines.push(line_to_static(line));
            continue;
        }

        let bg = line.style.bg.unwrap();
        let effective_width = max.saturating_sub(padding_width * 2);
        if effective_width == 0 {
            new_lines.push(line_to_static(line));
            continue;
        }

        // Collect (char, style) pairs, skipping existing 2-NBSP padding
        let styled_chars: Vec<(char, ratatui::style::Style)> = line
            .spans
            .iter()
            .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
            .collect();

        let skip = if styled_chars.len() >= 2
            && styled_chars[0].0 == '\u{00a0}'
            && styled_chars[1].0 == '\u{00a0}'
        {
            2
        } else {
            0
        };
        let code_chars = &styled_chars[skip..];
        let padding_style = ratatui::style::Style::default().bg(bg);

        let mut pos = 0;
        let mut width_acc = 0;
        let mut chunk_start = 0;

        while pos < code_chars.len() {
            let ch_width = code_chars[pos].0.width().unwrap_or(0);
            if width_acc + ch_width > effective_width && width_acc > 0 {
                new_lines.push(build_sub_line(
                    &code_chars[chunk_start..pos],
                    padding,
                    padding_style,
                    line.style,
                ));
                chunk_start = pos;
                width_acc = 0;
            }
            width_acc += ch_width;
            pos += 1;
        }

        if chunk_start < code_chars.len() {
            new_lines.push(build_sub_line(
                &code_chars[chunk_start..],
                padding,
                padding_style,
                line.style,
            ));
        }
    }

    (Text::from(new_lines), index_map)
}

fn line_to_static(line: &ratatui::text::Line<'_>) -> ratatui::text::Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| Span::styled(s.content.to_string(), s.style))
        .collect();
    ratatui::text::Line::from(spans).style(line.style)
}

fn build_sub_line(
    chars: &[(char, ratatui::style::Style)],
    padding: &str,
    padding_style: ratatui::style::Style,
    line_style: ratatui::style::Style,
) -> ratatui::text::Line<'static> {
    let mut spans = vec![Span::styled(padding.to_string(), padding_style)];
    if !chars.is_empty() {
        let mut current_style = chars[0].1;
        let mut current_text = String::new();
        for &(ch, style) in chars {
            if style == current_style {
                current_text.push(ch);
            } else {
                spans.push(Span::styled(current_text, current_style));
                current_text = String::new();
                current_style = style;
                current_text.push(ch);
            }
        }
        if !current_text.is_empty() {
            spans.push(Span::styled(current_text, current_style));
        }
    }
    ratatui::text::Line::from(spans).style(line_style)
}

/// Compute image placement rect within a content area, accounting for scroll.
/// When `center` is true and pixel dimensions are available, the image is
/// horizontally centered based on its aspect ratio.
fn compute_image_placement(
    content_area: Rect,
    y_offset: usize,
    height: u16,
    scroll: u16,
    path: &str,
    center: bool,
    pixel_width: u32,
    pixel_height: u32,
    max_width_percent: Option<f64>,
) -> Option<ImagePlacement> {
    let y_start = y_offset as i32 - scroll as i32;
    let y_end = y_start + height as i32;

    if y_end <= 0 || y_start >= content_area.height as i32 {
        return None;
    }

    let y = (y_start.max(0) as u16) + content_area.y;
    let h = (y_end.min(content_area.height as i32) - y_start.max(0)) as u16;

    if h == 0 {
        return None;
    }

    // Apply max_width_percent constraint
    let max_w = if let Some(pct) = max_width_percent {
        ((content_area.width as f64) * pct.clamp(0.0, 1.0)) as u16
    } else {
        content_area.width
    };

    let (x, w) = if center && pixel_width > 0 && pixel_height > 0 {
        // Estimate display width in cells from aspect ratio.
        // Terminal cells are typically ~2x taller than wide in pixels.
        let cell_aspect = 2.0_f64;
        let display_w =
            ((height as f64) * (pixel_width as f64) / (pixel_height as f64) * cell_aspect) as u16;
        let display_w = display_w.min(max_w);
        let x_offset = (content_area.width.saturating_sub(display_w)) / 2;
        (content_area.x + x_offset, display_w)
    } else {
        let w = max_w;
        let x_offset = if max_width_percent.is_some() {
            (content_area.width.saturating_sub(w)) / 2
        } else {
            0
        };
        (content_area.x + x_offset, w)
    };

    Some(ImagePlacement {
        x,
        y,
        width: w,
        height: h,
        path: path.to_string(),
        clip_top: y_start < 0,
        full_height: height,
    })
}
