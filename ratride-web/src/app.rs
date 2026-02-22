use crate::backend::CanvasBackend;
use crate::overlay::DomOverlay;
use ratride::color::{anim_color, blend_color, hue_to_rgb};
use ratride::markdown::{Frontmatter, Slide, TransitionKind, parse_slides};
use ratride::render::{self, ImagePlacement};
use ratride::theme::Theme;
use ratatui::{
    Terminal,
    buffer::Buffer,
    layout::{Constraint, Layout},
};
use std::collections::HashMap;
use tachyonfx::{Duration, Effect, EffectRenderer, Interpolation, Motion, fx};
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
        base_url: &str,
        overlay: DomOverlay,
    ) -> Self {
        let cols = backend.cols();
        let rows = backend.rows();
        let slides = parse_slides(markdown, &theme, frontmatter);
        let len = slides.len().max(1);
        let terminal = Terminal::new(backend).expect("terminal creation");

        // Collect unique image paths and preload them
        let mut images: HashMap<String, HtmlImageElement> = HashMap::new();
        for slide in &slides {
            for img in &slide.images {
                if images.contains_key(&img.path) {
                    continue;
                }
                let el = HtmlImageElement::new().expect("create img element");
                let src = if img.path.starts_with("http://") || img.path.starts_with("https://") {
                    img.path.clone()
                } else {
                    format!("{}{}", base_url, img.path)
                };
                el.set_src(&src);
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

    pub fn handle_key(&mut self, key: &str) {
        match key {
            "ArrowRight" | "l" | " " => self.next_page(),
            "ArrowLeft" | "h" => self.prev_page(),
            "ArrowDown" | "j" if self.can_scroll() => {
                *self.scroll_offset_mut() = self.scroll_offset().saturating_add(1);
            }
            "ArrowUp" | "k" if self.can_scroll() => {
                *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(1);
            }
            "d" if self.can_scroll() => {
                *self.scroll_offset_mut() = self.scroll_offset().saturating_add(10);
            }
            "u" if self.can_scroll() => {
                *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(10);
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, timestamp: f64) {
        self.last_timestamp = timestamp;

        // Update cols/rows from backend
        self.terminal.backend_mut().resize();
        self.cols = self.terminal.backend().cols();
        self.rows = self.terminal.backend().rows();

        let current_page = self.current_page;
        let total_pages = self.total_pages();
        let slide = self.slides[current_page].clone();
        let scroll = self.scroll_offset();
        let theme = self.theme.clone();
        let layout = slide.layout.clone();

        let mut effect = self.effect.take();
        let mut placements = Vec::new();

        let completed = self
            .terminal
            .draw(|frame| {
                let area = frame.area();
                let [main_area, status_area] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

                // Draw slide content, collect image placements
                placements = render::draw_slide(&slide, scroll, frame, main_area);

                // Apply transition effect
                if let Some(ref mut eff) = effect {
                    let delta = Duration::from_millis(FRAME_DURATION_MS as u32);
                    frame.render_effect(eff, main_area, delta);
                    if eff.done() {
                        effect = None;
                    }
                }

                // Status bar
                render::draw_status_bar(
                    &layout,
                    current_page,
                    total_pages,
                    frame,
                    status_area,
                    &theme,
                );
            })
            .expect("draw");

        self.effect = effect;
        self.prev_buffer = Some(completed.buffer.clone());
        self.pending_placements = placements;

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
        // Content area offset: Margin::new(2, 1) in render.rs draw_default
        let content_offset_x = 2.0 * cell_w;
        let content_offset_y = 1.0 * cell_h;
        let visible_rows = self.rows.saturating_sub(3); // main_area minus margins

        self.overlay.update(
            &slide.semantics,
            scroll,
            content_offset_x,
            content_offset_y,
            cell_w,
            cell_h,
            visible_rows,
        );
        self.overlay.set_visible(true);
    }

    fn draw_images(&self) {
        for placement in &self.pending_placements {
            if let Some(img_el) = self.images.get(&placement.path) {
                self.terminal.backend().draw_image(
                    img_el,
                    placement.x,
                    placement.y,
                    placement.width,
                    placement.height,
                );
            }
        }
    }

    fn create_transition(&self) -> Option<Effect> {
        let slide = &self.slides[self.current_page];
        let bg = self.theme.bg;
        let prev_buf = self.prev_buffer.clone();
        let rows = self.rows;
        Some(match slide.transition {
            TransitionKind::None => return None,
            TransitionKind::SlideIn => fx::fade_from_fg(bg, (400, Interpolation::QuadOut)),
            TransitionKind::Fade => fx::fade_from_fg(bg, (600, Interpolation::SineOut)),
            TransitionKind::Dissolve => fx::dissolve((500, Interpolation::Linear)).reversed(),
            TransitionKind::Coalesce => fx::coalesce((500, Interpolation::QuadOut)),
            TransitionKind::SweepIn => fx::sweep_in(
                Motion::LeftToRight,
                15,
                0,
                bg,
                (600, Interpolation::QuadOut),
            ),
            TransitionKind::Lines => {
                let prev = prev_buf.clone();
                let approx_lines = rows as f32;
                let duration_ms = LINE_DUR_MS + STAGGER_MS * (approx_lines - 1.0).max(0.0);
                fx::effect_fn_buf(
                    (),
                    (duration_ms as u32, Interpolation::Linear),
                    move |_state, ctx, buf| {
                        let elapsed = ctx.alpha() * duration_ms;
                        let area = ctx.area;
                        let width = area.width;

                        for y in area.y..area.y + area.height {
                            let line_index = (y - area.y) as f32;
                            let line_start = line_index * STAGGER_MS;
                            let local_alpha =
                                ((elapsed - line_start) / LINE_DUR_MS).clamp(0.0, 1.0);
                            let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                            let shift = ((1.0 - local_alpha) * width as f32) as u16;

                            let original: Vec<_> = (area.x..area.x + width)
                                .map(|x| buf[(x, y)].clone())
                                .collect();

                            for x in area.x..area.x + width {
                                let col = x - area.x;
                                let src_col = col + shift;
                                let cell = &mut buf[(x, y)];
                                if src_col < width {
                                    *cell = original[src_col as usize].clone();
                                } else {
                                    let d = (src_col - width) as f32;
                                    let fade = (d * 2.0 / width as f32).clamp(0.0, 1.0);
                                    if fade > 0.0 {
                                        if let Some(old) =
                                            prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                        {
                                            cell.set_char(
                                                old.symbol().chars().next().unwrap_or(' '),
                                            );
                                            cell.set_fg(blend_color(bg, old.fg, fade));
                                            cell.set_bg(blend_color(bg, old.bg, fade));
                                        }
                                    } else {
                                        cell.reset();
                                    }
                                }
                            }
                        }
                    },
                )
            }
            TransitionKind::LinesCross => {
                let prev = prev_buf.clone();
                let approx_lines = rows as f32;
                let duration_ms = LINE_DUR_MS + STAGGER_MS * (approx_lines - 1.0).max(0.0);
                fx::effect_fn_buf(
                    (),
                    (duration_ms as u32, Interpolation::Linear),
                    move |_state, ctx, buf| {
                        let elapsed = ctx.alpha() * duration_ms;
                        let area = ctx.area;
                        let width = area.width as f32;

                        for y in area.y..area.y + area.height {
                            let line_index = (y - area.y) as f32;
                            let line_start = line_index * STAGGER_MS;
                            let local_alpha =
                                ((elapsed - line_start) / LINE_DUR_MS).clamp(0.0, 1.0);
                            let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                            let visible_cols = (local_alpha * width) as u16;
                            let is_odd = (y - area.y) % 2 == 1;

                            for x in area.x..area.x + area.width {
                                let col_offset = x - area.x;
                                let should_blank = if is_odd {
                                    col_offset < area.width - visible_cols
                                } else {
                                    col_offset >= visible_cols
                                };
                                if should_blank {
                                    let cell = &mut buf[(x, y)];
                                    let d = if is_odd {
                                        (area.width - visible_cols - 1 - col_offset) as f32
                                    } else {
                                        (col_offset - visible_cols) as f32
                                    };
                                    let fade = (d * 2.0 / area.width as f32).clamp(0.0, 1.0);
                                    if fade > 0.0 {
                                        if let Some(old) =
                                            prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                        {
                                            cell.set_char(
                                                old.symbol().chars().next().unwrap_or(' '),
                                            );
                                            cell.set_fg(blend_color(bg, old.fg, fade));
                                            cell.set_bg(blend_color(bg, old.bg, fade));
                                        }
                                    } else {
                                        cell.reset();
                                    }
                                }
                            }
                        }
                    },
                )
            }
            TransitionKind::LinesRgb => {
                let prev = prev_buf.clone();
                let approx_lines = rows as f32;
                let duration_ms = LINE_DUR_MS + STAGGER_MS * (approx_lines - 1.0).max(0.0);
                fx::effect_fn_buf(
                    (),
                    (duration_ms as u32, Interpolation::Linear),
                    move |_state, ctx, buf| {
                        let elapsed = ctx.alpha() * duration_ms;
                        let area = ctx.area;
                        let width = area.width;

                        for y in area.y..area.y + area.height {
                            let line_index = (y - area.y) as f32;
                            let line_start = line_index * STAGGER_MS;
                            let local_alpha =
                                ((elapsed - line_start) / LINE_DUR_MS).clamp(0.0, 1.0);
                            let local_alpha = Interpolation::QuadOut.alpha(local_alpha);
                            let shift = ((1.0 - local_alpha) * width as f32) as u16;

                            let original: Vec<_> = (area.x..area.x + width)
                                .map(|x| buf[(x, y)].clone())
                                .collect();

                            let color = anim_color(local_alpha);

                            for x in area.x..area.x + width {
                                let col = x - area.x;
                                let cell = &mut buf[(x, y)];

                                let (in_range, src_col) = {
                                    let sc = col + shift;
                                    if sc < width { (true, sc) } else { (false, 0) }
                                };

                                if in_range {
                                    *cell = original[src_col as usize].clone();
                                    if local_alpha < 1.0 {
                                        cell.set_fg(color);
                                    }
                                } else {
                                    let d = (col + shift - width) as f32;
                                    let fade = (d * 2.0 / width as f32).clamp(0.0, 1.0);
                                    if fade > 0.0 {
                                        if let Some(old) =
                                            prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                        {
                                            cell.set_char(
                                                old.symbol().chars().next().unwrap_or(' '),
                                            );
                                            cell.set_fg(blend_color(bg, old.fg, fade));
                                            cell.set_bg(blend_color(bg, old.bg, fade));
                                        }
                                    } else {
                                        cell.reset();
                                    }
                                }
                            }
                        }
                    },
                )
            }
            TransitionKind::SlideRgb => {
                let prev = prev_buf.clone();
                let band_width = 24_u16;
                fx::effect_fn_buf(
                    (),
                    (800, Interpolation::QuadOut),
                    move |_state, ctx, buf| {
                        let alpha = ctx.alpha();
                        let area = ctx.area;
                        let width = area.width as f32;
                        let edge_col = (alpha * (width + band_width as f32)) as u16;

                        for y in area.y..area.y + area.height {
                            for x in area.x..area.x + area.width {
                                let col_offset = x - area.x;
                                if col_offset >= edge_col {
                                    let cell = &mut buf[(x, y)];
                                    if let Some(old) =
                                        prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                    {
                                        *cell = old.clone();
                                    }
                                } else if col_offset + band_width >= edge_col {
                                    let d = edge_col - col_offset;
                                    let t = d as f32 / band_width as f32;
                                    let hue = t * 300.0;
                                    let color = hue_to_rgb(hue);
                                    let cell = &mut buf[(x, y)];
                                    cell.set_fg(color);
                                }
                            }
                        }
                    },
                )
            }
        })
    }
}
