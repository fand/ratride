mod markdown;

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use markdown::{Slide, SlideLayout, TransitionKind};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    style::Color,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Wrap},
    DefaultTerminal, Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use tachyonfx::{fx, Duration, Effect, EffectRenderer, Interpolation, Motion};

const FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(16); // ~60fps

struct App {
    slides: Vec<Slide>,
    current_page: usize,
    scroll_offsets: Vec<u16>,
    quit: bool,
    image_states: HashMap<String, StatefulProtocol>,
    /// Active transition effect.
    effect: Option<Effect>,
    last_frame: Instant,
}

impl App {
    fn new(markdown: &str, base_dir: &Path) -> Self {
        let slides = markdown::parse_slides(markdown);
        let len = slides.len().max(1);

        let mut image_states: HashMap<String, StatefulProtocol> = HashMap::new();
        let picker = Picker::from_query_stdio().ok();
        if let Some(picker) = picker {
            for slide in &slides {
                for img in &slide.images {
                    if image_states.contains_key(&img.path) {
                        continue;
                    }
                    let img_path = base_dir.join(&img.path);
                    if let Ok(dyn_img) = image::ImageReader::open(&img_path)
                        .and_then(|r| r.decode().map_err(|e| io::Error::new(io::ErrorKind::Other, e)))
                    {
                        let protocol = picker.new_resize_protocol(dyn_img);
                        image_states.insert(img.path.clone(), protocol);
                    }
                }
            }
        }

        Self {
            slides,
            current_page: 0,
            scroll_offsets: vec![0; len],
            quit: false,
            image_states,
            effect: None,
            last_frame: Instant::now(),
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
        match slide.transition {
            TransitionKind::SlideIn => fx::fade_from_fg(
                Color::Black,
                (400, Interpolation::QuadOut),
            ),
            TransitionKind::Fade => fx::fade_from_fg(
                Color::Black,
                (600, Interpolation::SineOut),
            ),
            TransitionKind::Dissolve => fx::dissolve(
                (500, Interpolation::Linear),
            ).reversed(),
            TransitionKind::Coalesce => fx::coalesce(
                (500, Interpolation::QuadOut),
            ),
            TransitionKind::SweepIn => fx::sweep_in(
                Motion::LeftToRight,
                15,
                0,
                Color::Black,
                (600, Interpolation::QuadOut),
            ),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        self.last_frame = Instant::now();
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            // Sleep to maintain frame rate
            let elapsed = self.last_frame.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
            self.last_frame = Instant::now();
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        let layout = self.slides[self.current_page].layout.clone();

        match layout {
            SlideLayout::Default => self.draw_default(frame, main_area),
            SlideLayout::Center => self.draw_center(frame, main_area),
            SlideLayout::TwoColumn => self.draw_two_column(frame, main_area),
        }

        // Apply transition effect
        if let Some(ref mut effect) = self.effect {
            let delta = Duration::from_millis(FRAME_DURATION.as_millis() as u32);
            frame.render_effect(effect, main_area, delta);
            if effect.done() {
                self.effect = None;
            }
        }

        // Status bar
        let layout_label = match layout {
            SlideLayout::Default => "",
            SlideLayout::Center => " [center]",
            SlideLayout::TwoColumn => " [two-column]",
        };
        let status = format!(
            " ←/→:page  j/k:scroll  q:quit{}    [{}/{}]",
            layout_label,
            self.current_page + 1,
            self.total_pages()
        );
        frame.render_widget(
            Paragraph::new(status).style(
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::DarkGray)
                    .fg(ratatui::style::Color::White),
            ),
            status_area,
        );
    }

    fn draw_default(&mut self, frame: &mut Frame, area: Rect) {
        let slide = &self.slides[self.current_page];
        let content_area = area.inner(Margin::new(2, 1));

        let paragraph = Paragraph::new(slide.content.clone())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(paragraph, content_area);

        let content_len = slide.content.lines.len();
        self.draw_scrollbar(frame, area, content_len, content_area.height);

        let scroll = self.scroll_offset();
        let images: Vec<_> = self.slides[self.current_page].images.clone();
        for img in &images {
            self.draw_image(frame, content_area, img.line_index, img.height, scroll, &img.path);
        }
    }

    fn draw_center(&mut self, frame: &mut Frame, area: Rect) {
        let slide = &self.slides[self.current_page];
        let content_height = slide.content.lines.len() as u16;
        let content_area = area.inner(Margin::new(2, 1));

        let [centered_area] = Layout::vertical([Constraint::Length(content_height)])
            .flex(Flex::Center)
            .areas(content_area);

        let paragraph = Paragraph::new(slide.content.clone())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(paragraph, centered_area);

        let scroll = self.scroll_offset();
        let images: Vec<_> = self.slides[self.current_page].images.clone();
        for img in &images {
            self.draw_image(frame, centered_area, img.line_index, img.height, scroll, &img.path);
        }
    }

    fn draw_two_column(&mut self, frame: &mut Frame, area: Rect) {
        let slide = &self.slides[self.current_page];
        let content_area = area.inner(Margin::new(2, 1));

        let [left_area, _gap, right_area] = Layout::horizontal([
            Constraint::Percentage(48),
            Constraint::Percentage(4),
            Constraint::Percentage(48),
        ])
        .areas(content_area);

        let left_para = Paragraph::new(slide.content.clone())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset(), 0));
        frame.render_widget(left_para, left_area);

        if let Some(ref right) = slide.right_content {
            let right_para = Paragraph::new(right.clone())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll_offset(), 0));
            frame.render_widget(right_para, right_area);
        }
    }

    fn draw_image(
        &mut self,
        frame: &mut Frame,
        content_area: Rect,
        line_index: usize,
        height: u16,
        scroll: u16,
        path: &str,
    ) {
        let y_start = line_index as i32 - scroll as i32;
        let y_end = y_start + height as i32;

        if y_end <= 0 || y_start >= content_area.height as i32 {
            return;
        }

        let y = (y_start.max(0) as u16) + content_area.y;
        let h = (y_end.min(content_area.height as i32) - y_start.max(0)) as u16;

        if h == 0 {
            return;
        }

        let img_area = Rect::new(content_area.x, y, content_area.width, h);

        if let Some(state) = self.image_states.get_mut(path) {
            StatefulImage::default().render(img_area, frame.buffer_mut(), state);
        }
    }

    fn draw_scrollbar(&self, frame: &mut Frame, area: Rect, content_len: usize, visible: u16) {
        let visible = visible as usize;
        if content_len > visible {
            let mut scrollbar_state = ScrollbarState::new(content_len.saturating_sub(visible))
                .position(self.scroll_offset() as usize);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Poll with timeout instead of blocking, so animation frames keep running
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

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("main.md");
    let base_dir = Path::new(path)
        .parent()
        .unwrap_or(Path::new("."));
    let markdown = std::fs::read_to_string(path)?;

    let terminal = ratatui::init();
    let result = App::new(&markdown, base_dir).run(terminal);
    ratatui::restore();
    result
}
