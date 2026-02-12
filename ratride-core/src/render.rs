use crate::markdown::{Slide, SlideLayout};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
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

    let paragraph = Paragraph::new(slide.content.clone())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, content_area);

    let content_len = slide.content.lines.len();
    draw_scrollbar(scroll, content_len, content_area.height, frame, area);

    let mut placements = Vec::new();
    for img in &slide.images {
        if let Some(p) = compute_image_placement(content_area, img.line_index, img.height, scroll, &img.path) {
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

    let paragraph = Paragraph::new(slide.content.clone())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, centered_area);

    let mut placements = Vec::new();
    for img in &slide.images {
        if let Some(p) = compute_image_placement(centered_area, img.line_index, img.height, scroll, &img.path) {
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
    layout: &SlideLayout,
    current_page: usize,
    total: usize,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
) {
    let layout_label = match layout {
        SlideLayout::Default => "",
        SlideLayout::Center => " [center]",
        SlideLayout::TwoColumn => " [two-column]",
    };
    let status = format!(
        " ←/→:page  j/k:scroll  q:quit{}    [{}/{}]",
        layout_label,
        current_page + 1,
        total,
    );
    frame.render_widget(
        Paragraph::new(status).style(
            ratatui::style::Style::default()
                .bg(theme.status_bg)
                .fg(theme.status_fg),
        ),
        area,
    );
}

/// Compute image placement rect within a content area, accounting for scroll.
fn compute_image_placement(
    content_area: Rect,
    line_index: usize,
    height: u16,
    scroll: u16,
    path: &str,
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

    Some(ImagePlacement {
        x: content_area.x,
        y,
        width: content_area.width,
        height: h,
        path: path.to_string(),
    })
}
