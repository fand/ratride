use crate::markdown::{Slide, SlideLayout};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    text::Text,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

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

/// Draw a slide's main content area (dispatches by layout).
/// Returns image placements for the terminal backend to render.
pub fn draw_slide(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> Vec<ImagePlacement> {
    match slide.layout {
        SlideLayout::Default => draw_default(slide, scroll, frame, area),
        SlideLayout::Center => draw_center(slide, scroll, frame, area),
        SlideLayout::TwoColumn => {
            draw_two_column(slide, scroll, frame, area);
            Vec::new()
        }
    }
}

pub fn draw_default(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> Vec<ImagePlacement> {
    let content_area = area.inner(Margin::new(2, 1));

    fill_line_backgrounds(&slide.content, scroll, frame, content_area);

    let paragraph = Paragraph::new(slide.content.clone())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, content_area);

    let content_len = slide.content.lines.len();
    draw_scrollbar(scroll, content_len, content_area.height, frame, area);

    let mut placements = Vec::new();
    for img in &slide.images {
        if let Some(p) = compute_image_placement(
            content_area,
            img.line_index,
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
    placements
}

pub fn draw_center(
    slide: &Slide,
    scroll: u16,
    frame: &mut Frame,
    area: Rect,
) -> Vec<ImagePlacement> {
    let content_height = slide.content.lines.len() as u16;
    let content_area = area.inner(Margin::new(2, 1));

    let [centered_area] = Layout::vertical([Constraint::Length(content_height)])
        .flex(Flex::Center)
        .areas(content_area);

    fill_line_backgrounds(&slide.content, scroll, frame, centered_area);

    let paragraph = Paragraph::new(slide.content.clone())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, centered_area);

    let mut placements = Vec::new();
    for img in &slide.images {
        if let Some(p) = compute_image_placement(
            centered_area,
            img.line_index,
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
    placements
}

pub fn draw_two_column(slide: &Slide, scroll: u16, frame: &mut Frame, area: Rect) {
    let content_area = area.inner(Margin::new(2, 1));

    let [left_area, _gap, right_area] = Layout::horizontal([
        Constraint::Percentage(48),
        Constraint::Percentage(4),
        Constraint::Percentage(48),
    ])
    .areas(content_area);

    let left_para = Paragraph::new(slide.content.clone())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(left_para, left_area);

    if let Some(ref right) = slide.right_content {
        let right_para = Paragraph::new(right.clone())
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
pub fn draw_header(
    header: &[String],
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) {
    if header.is_empty() {
        return;
    }

    let separator = " │ ";
    let mut spans = Vec::new();
    let style = ratatui::style::Style::default()
        .bg(theme.surface)
        .fg(theme.fg);
    let sep_style = ratatui::style::Style::default()
        .bg(theme.surface)
        .fg(theme.list_bullet);

    for (i, item) in header.iter().enumerate() {
        if i > 0 {
            spans.push(ratatui::text::Span::styled(separator, sep_style));
        }
        spans.push(ratatui::text::Span::styled(item.clone(), style));
    }

    // Add padding
    spans.insert(0, ratatui::text::Span::styled(" ", style));
    spans.push(ratatui::text::Span::styled(" ", style));

    let line = ratatui::text::Line::from(spans);
    let width: u16 = line.width() as u16;

    // Position at top-right with 1-cell margin from the right edge
    let x = area.x + area.width.saturating_sub(width + 1);
    let header_area = Rect::new(x, area.y, width, 1);

    let paragraph = Paragraph::new(line).alignment(Alignment::Right);
    frame.render_widget(paragraph, header_area);
}

/// Compute how many screen rows a line occupies when word-wrapped to `width` columns.
fn wrapped_line_height(line: &ratatui::text::Line<'_>, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let w = width as usize;
    let total_width = line.width();
    ((total_width + w - 1) / w).max(1) as u16
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

/// Compute image placement rect within a content area, accounting for scroll.
/// When `center` is true and pixel dimensions are available, the image is
/// horizontally centered based on its aspect ratio.
fn compute_image_placement(
    content_area: Rect,
    line_index: usize,
    height: u16,
    scroll: u16,
    path: &str,
    center: bool,
    pixel_width: u32,
    pixel_height: u32,
    max_width_percent: Option<f64>,
) -> Option<ImagePlacement> {
    let y_start = line_index as i32 - scroll as i32;
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
