use crate::backend::CanvasBackend;
use crate::overlay::DomOverlay;
use ratride::markdown::{FigletFn, Frontmatter, Slide, SlideLayout, parse_slides};
use ratride::render::{self, ImagePlacement};
use ratride::theme::Theme;
use ratatui::{
    Terminal,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Margin, Rect},
};
use std::collections::{HashMap, HashSet};
use tachyonfx::{Duration, Effect, EffectRenderer};
use web_sys::HtmlImageElement;

const FRAME_DURATION_MS: f64 = 16.0; // ~60fps
const LINE_DUR_MS: f32 = 600.0;
const STAGGER_MS: f32 = 60.0;

pub struct WebApp {
    terminal: Terminal<CanvasBackend>,
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offsets: Vec<u16>,
    theme: Theme,
    effect: Option<Effect>,
    prev_buffer: Option<Buffer>,
    last_timestamp: f64,
    cols: u16,
    rows: u16,
    images: HashMap<String, HtmlImageElement>,
    image_dims_resolved: HashSet<String>,
    pending_placements: Vec<ImagePlacement>,
    overlay: DomOverlay,
    overlay_last_page: usize,
    overlay_last_scroll: u16,
}

impl WebApp {
    pub fn new(
        backend: CanvasBackend,
        markdown: &str,
        theme: Theme,
        frontmatter: &Frontmatter,
        overlay: DomOverlay,
        figlet_fn: Option<&FigletFn>,
        is_mobile: bool,
    ) -> Self {
        let cols = backend.cols();
        let rows = backend.rows();
        let slides = parse_slides(markdown, &theme, frontmatter, figlet_fn, is_mobile);
        let len = slides.len().max(1);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.backend_mut().set_bg_color(theme.bg);

        // Collect unique image paths and preload them
        let mut images: HashMap<String, HtmlImageElement> = HashMap::new();
        for slide in &slides {
            for img in &slide.images {
                if images.contains_key(&img.path) {
                    continue;
                }
                let el = HtmlImageElement::new().expect("create img element");
                el.set_src(&img.path);
                images.insert(img.path.clone(), el);
            }
        }

        Self {
            terminal,
            slides,
            current_page: 0,
            scroll_offsets: vec![0; len],
            theme,
            effect: None,
            prev_buffer: None,
            last_timestamp: 0.0,
            cols,
            rows,
            images,
            image_dims_resolved: HashSet::new(),
            pending_placements: Vec::new(),
            overlay,
            overlay_last_page: usize::MAX,
            overlay_last_scroll: u16::MAX,
        }
    }

    pub fn init(&mut self) {
        self.effect = self.create_transition();
    }

    fn total_pages(&self) -> usize {
        self.slides.len()
    }

    fn scroll_offset(&self) -> u16 {
        self.scroll_offsets[self.current_page]
    }

    fn scroll_offset_mut(&mut self) -> &mut u16 {
        &mut self.scroll_offsets[self.current_page]
    }

    fn can_scroll(&self) -> bool {
        let visible = self.rows.saturating_sub(3) as usize;
        let content_len = self.slides[self.current_page].content.lines.len();
        content_len > visible
    }

    fn max_scroll(&self) -> u16 {
        let visible = self.rows.saturating_sub(3) as usize;
        let slide = &self.slides[self.current_page];
        let content_len = slide.content.lines.len();
        let right_len = slide.right_content.as_ref().map_or(0, |r| r.lines.len());
        content_len.max(right_len).saturating_sub(visible) as u16
    }

    fn goto_page(&mut self, page: usize) {
        if page < self.total_pages() && page != self.current_page {
            self.current_page = page;
            self.effect = self.create_transition();
        }
    }

    pub fn next_page(&mut self) {
        let next = self.current_page + 1;
        self.goto_page(next);
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.goto_page(self.current_page - 1);
        }
    }

    pub fn scroll_down(&mut self, lines: u16) {
        if self.can_scroll() {
            *self.scroll_offset_mut() = self.scroll_offset().saturating_add(lines).min(self.max_scroll());
        }
    }

    pub fn scroll_up(&mut self, lines: u16) {
        if self.can_scroll() {
            *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(lines);
        }
    }

    pub fn cell_height(&self) -> f64 {
        self.terminal.backend().cell_height()
    }

    pub fn handle_key(&mut self, key: &str) {
        match key {
            "ArrowRight" | "l" | " " => self.next_page(),
            "ArrowLeft" | "h" => self.prev_page(),
            "ArrowDown" | "j" => self.scroll_down(1),
            "ArrowUp" | "k" => self.scroll_up(1),
            "d" => self.scroll_down(10),
            "u" => self.scroll_up(10),
            _ => {}
        }
    }

    /// Resolve image dimensions for newly loaded images.
    /// Only adjusts placeholder height for images with max_width_percent;
    /// images without it keep the fixed placeholder (matching terminal behavior).
    fn resolve_image_dimensions(&mut self) {
        let content_w = self.cols.saturating_sub(4) as f64;
        let cell_w = self.terminal.backend().cell_width();
        let cell_h = self.terminal.backend().cell_height();
        for slide in &mut self.slides {
            let mut line_delta: i32 = 0;
            for img in &mut slide.images {
                img.line_index = ((img.line_index as i32) + line_delta).max(0) as usize;
                if self.image_dims_resolved.contains(&img.path) {
                    continue;
                }
                let el = match self.images.get(&img.path) {
                    Some(el) => el,
                    None => continue,
                };
                if !el.complete() || el.natural_width() == 0 {
                    continue;
                }
                img.pixel_width = el.natural_width();
                img.pixel_height = el.natural_height();
                self.image_dims_resolved.insert(img.path.clone());

                let pct = match img.max_width_percent {
                    Some(pct) => pct,
                    None => continue,
                };
                let display_w = content_w * pct.clamp(0.0, 1.0);
                let new_h = (display_w * cell_w * img.pixel_height as f64
                    / (img.pixel_width as f64 * cell_h))
                    .ceil() as u16;
                let new_h = new_h.max(1);

                if new_h < img.height {
                    let to_remove = (img.height - new_h) as usize;
                    let start = img.line_index + new_h as usize;
                    if start + to_remove <= slide.content.lines.len() {
                        slide.content.lines.drain(start..start + to_remove);
                        line_delta -= to_remove as i32;
                    }
                    img.height = new_h;
                } else if new_h > img.height {
                    let to_add = (new_h - img.height) as usize;
                    let insert_at = (img.line_index + img.height as usize)
                        .min(slide.content.lines.len());
                    for _ in 0..to_add {
                        slide.content.lines.insert(
                            insert_at,
                            ratatui::text::Line::default(),
                        );
                    }
                    line_delta += to_add as i32;
                    img.height = new_h;
                }
            }
        }
    }

    pub fn tick(&mut self, timestamp: f64) {
        self.last_timestamp = timestamp;

        // Update per-slide line_height if changed
        let slide_lh = self.slides[self.current_page].line_height;
        self.terminal.backend_mut().set_line_height(slide_lh);

        // Update cols/rows from backend
        self.terminal.backend_mut().resize();
        self.cols = self.terminal.backend().cols();
        self.rows = self.terminal.backend().rows();

        // Resolve image dimensions for newly loaded images
        self.resolve_image_dimensions();

        // Canvas doesn't retain cell state like a terminal, so reset viewport
        // buffer every frame to force full redraw (prevents stale pixels on scroll).
        // Use slide's bg color for canvas clear when bg_fill is enabled.
        let current_page = self.current_page;
        let slide = self.slides[current_page].clone();
        if slide.bg_fill {
            self.terminal.backend_mut().set_bg_color(slide.theme.bg);
        } else {
            self.terminal.backend_mut().set_bg_color(self.theme.bg);
        }
        self.terminal.clear().ok();

        let total_pages = self.total_pages();
        let scroll = self.scroll_offset();
        let theme = self.theme.clone();

        let mut effect = self.effect.take();
        let mut placements = Vec::new();

        let completed = self
            .terminal
            .draw(|frame| {
                let area = frame.area();

                // Also fill buffer cells so flush doesn't clear_rect them back to transparent
                if slide.bg_fill {
                    let slide_bg = slide.theme.bg;
                    let buf = frame.buffer_mut();
                    for y in area.y..area.y + area.height {
                        for x in area.x..area.x + area.width {
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_bg(slide_bg);
                            }
                        }
                    }
                }

                let [main_area, status_area] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

                // Draw slide content, collect image placements
                let (img_placements, _hyperlinks) = render::draw_slide(&slide, scroll, frame, main_area);
                placements = img_placements;

                // Apply transition effect
                if let Some(ref mut eff) = effect {
                    let delta = Duration::from_millis(FRAME_DURATION_MS as u32);
                    frame.render_effect(eff, main_area, delta);
                    if eff.done() {
                        effect = None;
                    }
                }

                // Header (top-right overlay)
                let _ = render::draw_header(&slide.header, frame, main_area, &theme);

                // Status bar
                render::draw_status_bar_with_options(
                    current_page,
                    total_pages,
                    frame,
                    status_area,
                    &theme,
                    true,
                );
            })
            .expect("draw");

        self.effect = effect;
        self.pending_placements = placements;
        self.prev_buffer = Some(completed.buffer.clone());

        // Draw images on top of the cell grid (only when not in transition)
        if self.effect.is_none() {
            self.draw_images();
            self.update_overlay();
        } else {
            self.overlay.set_visible(false);
        }
    }

    fn update_overlay(&mut self) {
        let page = self.current_page;
        let scroll = self.scroll_offset();
        if page == self.overlay_last_page && scroll == self.overlay_last_scroll {
            self.overlay.set_visible(true);
            return;
        }
        self.overlay_last_page = page;
        self.overlay_last_scroll = scroll;

        let slide = &self.slides[page];
        let cell_w = self.terminal.backend().cell_width();
        let cell_h = self.terminal.backend().cell_height();
        // Content area offset: Margin::new(2, 1) in render.rs
        let content_offset_x = 2.0 * cell_w;
        let mut content_offset_y = 1.0 * cell_h;
        let visible_rows = self.rows.saturating_sub(3);
        let content_width = self.cols.saturating_sub(4);

        let is_center = matches!(slide.layout, SlideLayout::Center);
        if is_center {
            // Mirror the exact Layout used in render::draw_center
            let main_area = Rect::new(0, 0, self.cols, self.rows.saturating_sub(1));
            let content_area = main_area.inner(Margin::new(2, 1));
            let content_height = slide.content.lines.len() as u16;
            let [centered] = Layout::vertical([Constraint::Length(content_height)])
                .flex(Flex::Center)
                .areas(content_area);
            content_offset_y = centered.y as f64 * cell_h;
        }

        self.overlay.update(
            &slide.semantics,
            scroll,
            content_offset_x,
            content_offset_y,
            cell_w,
            cell_h,
            visible_rows,
            is_center,
            &slide.content,
            content_width,
        );
        // Header links (top-right overlay, row 0 of main_area)
        self.overlay.update_header_links(
            &slide.header,
            0.0,
            0.0,
            cell_w,
            cell_h,
            self.cols,
        );
        self.overlay.set_visible(true);
    }

    fn draw_images(&self) {
        for placement in &self.pending_placements {
            if let Some(img_el) = self.images.get(&placement.path) {
                self.terminal.backend().draw_image(img_el, placement);
            }
        }
    }

    fn create_transition(&self) -> Option<Effect> {
        let slide = &self.slides[self.current_page];
        let bg = self.theme.bg;
        let prev_buf = self.prev_buffer.clone();
        ratride::transition::create_transition(
            &slide.transition,
            bg,
            prev_buf,
            self.rows,
            slide.content.lines.len(),
            LINE_DUR_MS,
            STAGGER_MS,
        )
    }
}
