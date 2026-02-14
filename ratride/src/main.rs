mod markdown;
mod render;
mod theme;

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use clap::Parser;

use crate::markdown::{Slide, TransitionKind, parse_slides};
use crate::render::ImagePlacement;
use crate::theme::Theme;
use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::StatefulWidget,
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use tachyonfx::{Duration, Effect, EffectRenderer, Interpolation, Motion, fx};

const FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(16); // ~60fps

/// Linearly blend two colors. At t=0 returns `a`, at t=1 returns `b`.
/// Non-RGB colors (e.g. Color::Reset) are returned as-is to avoid
/// introducing explicit background colors where the terminal default is used.
fn blend_color(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let inv = 1.0 - t;
            Color::Rgb(
                (ar as f32 * inv + br as f32 * t) as u8,
                (ag as f32 * inv + bg as f32 * t) as u8,
                (ab as f32 * inv + bb as f32 * t) as u8,
            )
        }
        _ => b,
    }
}

/// Convert a hue (0-360) to an RGB color (full saturation & value).
fn hue_to_rgb(hue: f32) -> Color {
    let h = (hue % 360.0) / 60.0;
    let i = h.floor() as u8;
    let f = h - h.floor();
    let q = (255.0 * (1.0 - f)) as u8;
    let t = (255.0 * f) as u8;
    match i {
        0 => Color::Rgb(255, t, 0),
        1 => Color::Rgb(q, 255, 0),
        2 => Color::Rgb(0, 255, t),
        3 => Color::Rgb(0, q, 255),
        4 => Color::Rgb(t, 0, 255),
        _ => Color::Rgb(255, 0, q),
    }
}

/// Detect if the terminal supports iTerm2 inline image protocol.
fn is_iterm2() -> bool {
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        if term.contains("iTerm") || term.contains("WezTerm") {
            return true;
        }
    }
    if let Ok(lc) = std::env::var("LC_TERMINAL") {
        if lc.contains("iTerm") {
            return true;
        }
    }
    false
}

enum ImageBackend {
    /// Write iTerm2 escape sequences directly to stdout (presenterm-style).
    Iterm2 { images: HashMap<String, Vec<u8>> },
    /// Use ratatui-image for Kitty/Sixel/Halfblocks.
    RatatuiImage {
        states: HashMap<String, StatefulProtocol>,
    },
}

struct App {
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offsets: Vec<u16>,
    quit: bool,
    image_backend: ImageBackend,
    theme: Theme,
    /// Active transition effect.
    effect: Option<Effect>,
    last_frame: Instant,
    /// Deferred image draws (collected during draw, flushed after ratatui render).
    pending_images: Vec<ImagePlacement>,
    /// Buffer snapshot from the previous frame (used for transition effects).
    prev_buffer: Option<Buffer>,
}

impl App {
    fn new(markdown: &str, base_dir: &Path, theme: Theme) -> Self {
        let slides = parse_slides(markdown, &theme);
        let len = slides.len().max(1);

        let image_backend = if is_iterm2() {
            let mut images: HashMap<String, Vec<u8>> = HashMap::new();
            for slide in &slides {
                for img in &slide.images {
                    if images.contains_key(&img.path) {
                        continue;
                    }
                    let img_path = base_dir.join(&img.path);
                    if let Ok(data) = std::fs::read(&img_path) {
                        images.insert(img.path.clone(), data);
                    }
                }
            }
            ImageBackend::Iterm2 { images }
        } else {
            let mut states: HashMap<String, StatefulProtocol> = HashMap::new();
            let picker = Picker::from_query_stdio().ok();
            if let Some(picker) = picker {
                for slide in &slides {
                    for img in &slide.images {
                        if states.contains_key(&img.path) {
                            continue;
                        }
                        let img_path = base_dir.join(&img.path);
                        if let Ok(dyn_img) = image::ImageReader::open(&img_path).and_then(|r| {
                            r.decode()
                                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                        }) {
                            let protocol = picker.new_resize_protocol(dyn_img);
                            states.insert(img.path.clone(), protocol);
                        }
                    }
                }
            }
            ImageBackend::RatatuiImage { states }
        };

        Self {
            slides,
            current_page: 0,
            scroll_offsets: vec![0; len],
            quit: false,
            image_backend,
            theme,
            effect: None,
            last_frame: Instant::now(),
            pending_images: Vec::new(),
            prev_buffer: None,
        }
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

    fn goto_page(&mut self, page: usize) {
        if page < self.total_pages() && page != self.current_page {
            self.current_page = page;
            self.effect = Some(self.create_transition());
        }
    }

    fn next_page(&mut self) {
        let next = self.current_page + 1;
        self.goto_page(next);
    }

    fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.goto_page(self.current_page - 1);
        }
    }

    fn create_transition(&self) -> Effect {
        let slide = &self.slides[self.current_page];
        let bg = self.theme.bg;
        let prev_buf = self.prev_buffer.clone();
        match slide.transition {
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
                let line_dur_ms = 500.0_f32; // how long each line's slide-in takes
                let stagger_ms = 50.0_f32; // delay before next line starts
                let (_, term_h) = crossterm::terminal::size().unwrap_or((80, 24));
                let approx_lines = term_h as f32; // slightly overestimate for safety
                let duration_ms = line_dur_ms + stagger_ms * (approx_lines - 1.0).max(0.0);
                fx::effect_fn_buf(
                    (),
                    (duration_ms as u32, Interpolation::QuadOut),
                    move |_state, ctx, buf| {
                        let elapsed = ctx.alpha() * duration_ms;
                        let area = ctx.area;
                        let width = area.width;

                        for y in area.y..area.y + area.height {
                            let line_index = (y - area.y) as f32;
                            let line_start = line_index * stagger_ms;
                            let local_alpha =
                                ((elapsed - line_start) / line_dur_ms).clamp(0.0, 1.0);
                            let shift = ((1.0 - local_alpha) * width as f32) as u16;

                            // Snapshot row before modifying
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
                let line_dur_ms = 500.0_f32; // how long each line's reveal takes
                let stagger_ms = 50.0_f32; // delay before next line starts
                let (_, term_h) = crossterm::terminal::size().unwrap_or((80, 24));
                let approx_lines = term_h as f32;
                let duration_ms = line_dur_ms + stagger_ms * (approx_lines - 1.0).max(0.0);
                fx::effect_fn_buf(
                    (),
                    (duration_ms as u32, Interpolation::QuadOut),
                    move |_state, ctx, buf| {
                        let elapsed = ctx.alpha() * duration_ms;
                        let area = ctx.area;
                        let width = area.width as f32;

                        for y in area.y..area.y + area.height {
                            let line_index = (y - area.y) as f32;
                            let line_start = line_index * stagger_ms;
                            let local_alpha =
                                ((elapsed - line_start) / line_dur_ms).clamp(0.0, 1.0);
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
                                    let fade =
                                        (d * 2.0 / area.width as f32).clamp(0.0, 1.0);
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
                let band_width = 12_u16; // width of the RGB gradient band
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
                                    // Unrevealed: show old content
                                    let cell = &mut buf[(x, y)];
                                    if let Some(old) =
                                        prev.as_ref().and_then(|pb| pb.cell((x, y)))
                                    {
                                        *cell = old.clone();
                                    }
                                } else if col_offset + band_width >= edge_col {
                                    // Inside the gradient band
                                    let d = edge_col - col_offset; // 1..=band_width
                                    let t = d as f32 / band_width as f32; // 0..1
                                    let hue = t * 300.0; // 0..300 degrees
                                    let color = hue_to_rgb(hue);
                                    let cell = &mut buf[(x, y)];
                                    cell.set_fg(color);
                                }
                                // else: revealed content, keep as-is
                            }
                        }
                    },
                )
            }
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        terminal.draw(|_| {})?;
        self.effect = Some(self.create_transition());
        self.last_frame = Instant::now();
        while !self.quit {
            self.pending_images.clear();
            let completed = terminal.draw(|frame| self.draw(frame))?;
            self.prev_buffer = Some(completed.buffer.clone());
            if self.effect.is_none() {
                self.flush_iterm2_images()?;
            }
            self.handle_events()?;
            let elapsed = self.last_frame.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
            self.last_frame = Instant::now();
        }
        Ok(())
    }

    /// Write iTerm2 inline image escape sequences directly to stdout.
    fn flush_iterm2_images(&self) -> io::Result<()> {
        if let ImageBackend::Iterm2 { ref images } = self.image_backend {
            let pending = &self.pending_images;
            if pending.is_empty() {
                return Ok(());
            }
            let mut stdout = io::stdout();
            for img in pending {
                if let Some(data) = images.get(&img.path) {
                    crossterm::execute!(stdout, MoveTo(img.x, img.y))?;
                    let b64 = STANDARD.encode(data);
                    write!(
                        stdout,
                        "\x1b]1337;File=size={};width={};height={};inline=1;preserveAspectRatio=1:{}\x07",
                        data.len(),
                        img.width,
                        img.height,
                        b64,
                    )?;
                    stdout.flush()?;
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        let slide = &self.slides[self.current_page];
        let layout = slide.layout.clone();
        let scroll = self.scroll_offset();

        // Draw slide content via core render functions
        let mut placements = render::draw_slide(slide, scroll, frame, main_area);

        // Render images via native backend
        for placement in &placements {
            self.draw_image(frame, placement);
        }
        self.pending_images.append(&mut placements);

        // Apply transition effect
        if let Some(ref mut effect) = self.effect {
            let delta = Duration::from_millis(FRAME_DURATION.as_millis() as u32);
            frame.render_effect(effect, main_area, delta);
            if effect.done() {
                self.effect = None;
            }
        }

        // Status bar
        render::draw_status_bar(
            &layout,
            self.current_page,
            self.total_pages(),
            frame,
            status_area,
            &self.theme,
        );
    }

    fn draw_image(&mut self, frame: &mut Frame, placement: &ImagePlacement) {
        let img_area = Rect::new(placement.x, placement.y, placement.width, placement.height);
        match &mut self.image_backend {
            ImageBackend::Iterm2 { .. } => {
                // Deferred to flush_iterm2_images() — placement already stored
            }
            ImageBackend::RatatuiImage { states } => {
                if let Some(state) = states.get_mut(&placement.path) {
                    StatefulImage::default().render(img_area, frame.buffer_mut(), state);
                }
            }
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        while event::poll(std::time::Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => self.next_page(),
                    KeyCode::Left | KeyCode::Char('h') => self.prev_page(),
                    KeyCode::Char('j') | KeyCode::Down => {
                        *self.scroll_offset_mut() = self.scroll_offset().saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(1);
                    }
                    KeyCode::Char('d') => {
                        *self.scroll_offset_mut() = self.scroll_offset().saturating_add(10);
                    }
                    KeyCode::Char('u') => {
                        *self.scroll_offset_mut() = self.scroll_offset().saturating_sub(10);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// A tiny terminal-based Markdown slide presenter built with ratatui
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the Markdown slide file
    file: String,

    /// Theme name [mocha (default), macchiato, frappe, latte]
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let path = &cli.file;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let markdown = std::fs::read_to_string(path)?;

    let theme = cli
        .theme
        .as_deref()
        .and_then(theme::theme_from_name)
        .or_else(|| theme::theme_from_markdown(&markdown))
        .unwrap_or_default();

    let terminal = ratatui::init();
    let result = App::new(&markdown, base_dir, theme).run(terminal);
    ratatui::restore();
    result
}
